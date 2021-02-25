use chrono::{DateTime, Duration, NaiveDateTime, SecondsFormat, Utc};
use mongodb::{
    bson,
    bson::{doc, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use serenity::futures::StreamExt;
use tracing::info;

use crate::database::{DatabaseError, DatabaseResult};
use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::CacheData;

const COLLECTION_NAME: &str = "players";

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerEntry {
    tetrio_id: String,
    discord_id: Option<String>,
    link_timestamp: String,
    tetrio_data: Option<LeaderboardUser>,
    cache_data: Option<CacheData>,
}

pub struct PlayerCollection {
    collection: Collection,
}

impl PlayerCollection {
    pub fn new(database: &Database) -> PlayerCollection {
        PlayerCollection {
            collection: database.collection(COLLECTION_NAME),
        }
    }

    pub async fn add_player(
        &self,
        tetrio_id: &str,
        discord_id: Option<&String>,
    ) -> DatabaseResult<PlayerEntry> {
        info!("Adding {} to players", tetrio_id);
        let now = chrono::offset::Utc::now();
        let link_timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);

        let filter: Document = match discord_id {
            None => doc! {"tetrio_id": tetrio_id},
            Some(id) => doc! {"$or": [{"tetrio_id": tetrio_id}, {"discord_id": id}]},
        };

        match self.get_players(filter).await {
            Ok(entry) => {
                if !entry.is_empty() {
                    return Err(DatabaseError::DuplicateEntry);
                }
            }
            Err(e) => return Err(e),
        }

        let player_entry = PlayerEntry {
            tetrio_id: tetrio_id.to_owned(),
            discord_id: discord_id.cloned(),
            link_timestamp,
            tetrio_data: None,
            cache_data: None,
        };

        match self
            .collection
            .insert_one(bson::to_document(&player_entry).unwrap(), None)
            .await
        {
            Ok(_) => Ok(player_entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    pub async fn get_players(
        &self,
        filter: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<PlayerEntry>> {
        match self.collection.find(filter, None).await {
            Ok(result) => Ok(result
                .map(|entry| bson::from_document(entry.expect("bad entry")).expect("bad entry"))
                .collect()
                .await),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }

    pub async fn remove_players(&self, filter: Document) -> DatabaseResult<()> {
        info!("Deleting players with filter {:?}", filter);
        match self.collection.delete_many(filter, None).await {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }

    // We don't care about cache timeouts here since whats grabbed is already grabbed, might as well put it in, right?
    pub async fn update_all_with_lb(&self) -> DatabaseResult<()> {
        println!("Started updating via leaderboard");

        let response = crate::tetrio::leaderboard::request().await.unwrap();
        let cache_data = bson::to_document(&response.cache).unwrap();

        for user in response.data.users {
            if self
                .collection
                .count_documents(doc! {"tetrio_id": &user._id}, None)
                .await
                .unwrap()
                == 0
            {
                println!("    {} not in database, adding as new", user.username);
                self.add_player(&user._id, None).await?;
            }

            let tetrio_data_doc = bson::to_document(&user).unwrap();
            self.collection
                .update_one(
                    doc! {"tetrio_id": &user._id},
                    doc! {"$set":{"tetrio_data": tetrio_data_doc, "cache_data": &cache_data}},
                    None,
                )
                .await
                .expect("could not update player");
        }

        Ok(())
    }

    // Returns true if not cached or cached for longer than 10 minutes
    // We can't simply use cache.cached_until, since leaderboard data is cached for 1h, while regular user data is cached for 1min
    pub async fn is_cached(&self, filter: impl Into<Option<Document>>) -> bool {
        let cache_timeout = Duration::minutes(10);

        if let Some(entry) = self.collection.find_one(filter, None).await.unwrap() {
            let data: PlayerEntry = bson::from_document(entry).expect("bad entry");
            if data.tetrio_data.is_some() {
                if let Some(cache_data) = data.cache_data {
                    let naive_dt = NaiveDateTime::from_timestamp(cache_data.cached_at / 1000, 0);
                    let last_cached: DateTime<Utc> = DateTime::from_utc(naive_dt, Utc);
                    let now = Utc::now();

                    if now <= last_cached.checked_add_signed(cache_timeout).unwrap_or(now) {
                        return true;
                    }
                }
            }
        }

        false
    }
}
