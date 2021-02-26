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
use crate::tetrio;
use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::{CacheData, TetrioApiError};

const COLLECTION_NAME: &str = "players";

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerEntry {
    tetrio_id: String,
    discord_id: Option<String>,
    link_timestamp: String,
    tetrio_data: Option<LeaderboardUser>,
    cache_data: Option<CacheData>,
}

impl PlayerEntry {
    pub fn new(tetrio_id: &str, discord_id: Option<&String>) -> PlayerEntry {
        let now = chrono::offset::Utc::now();
        let link_timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
        PlayerEntry {
            tetrio_id: tetrio_id.to_string(),
            discord_id: discord_id.cloned(),
            link_timestamp,
            tetrio_data: None,
            cache_data: None,
        }
    }

    pub fn from_document(doc: Document) -> PlayerEntry {
        bson::from_document(doc).expect("bad entry")
    }

    // Returns true if not cached or cached for longer than 10 minutes
    // We can't simply use cache.cached_until, since leaderboard data is cached for 1h, while regular user data is cached for 1min
    pub fn is_cached(&self) -> bool {
        let cache_timeout = Duration::minutes(10);

        if self.tetrio_data.is_some() {
            if let Some(cache_data) = &self.cache_data {
                let naive_dt = NaiveDateTime::from_timestamp(cache_data.cached_at / 1000, 0);
                let last_cached: DateTime<Utc> = DateTime::from_utc(naive_dt, Utc);
                let now = Utc::now();

                if now <= last_cached.checked_add_signed(cache_timeout).unwrap_or(now) {
                    return true;
                }
            }
        }
        false
    }
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

    // This only cares about adding the tetrio/discord link, it will not add tetrio_data! (async recursion issue)
    // Call .update_player() afterwards if you need to add data
    // Checks for duplicate players
    pub async fn add_player(
        &self,
        tetrio_id: &str,
        discord_id: Option<&String>,
    ) -> DatabaseResult<PlayerEntry> {
        info!("Adding {} to players", tetrio_id);

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

        let player_entry = PlayerEntry::new(tetrio_id, discord_id);
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
                .map(|entry| PlayerEntry::from_document(entry.expect("bad entry")))
                .collect()
                .await),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }

    pub async fn get_player(&self, tetrio_id: &str) -> DatabaseResult<Option<PlayerEntry>> {
        match self
            .collection
            .find_one(
                doc! {"$or": [{"tetrio_id": tetrio_id}, {"tetrio_data.username": tetrio_id}]},
                None,
            )
            .await
        {
            Ok(entry) => Ok(entry.map(PlayerEntry::from_document)),
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

    // Writes the updated data to the database
    // Doesnt do any requesting or cache checking, and should thus only be used internally
    async fn update(
        &self,
        new_data: LeaderboardUser,
        cache_data: &CacheData,
    ) -> DatabaseResult<()> {
        if self
            .collection
            .count_documents(doc! {"tetrio_id": &new_data._id}, None)
            .await
            .unwrap()
            == 0
        {
            println!("{} not in database, adding as new", new_data.username);
            self.add_player(&new_data._id, None).await?;
        }

        let tetrio_data_doc = bson::to_document(&new_data).unwrap();
        let cache_data = bson::to_document(&cache_data).unwrap();
        self.collection
            .update_one(
                doc! {"tetrio_id": &new_data._id},
                doc! {"$set":{"tetrio_data": tetrio_data_doc, "cache_data": cache_data}},
                None,
            )
            .await
            .expect("could not update player");

        Ok(())
    }

    // Uses leaderboard data to write to the database
    // We don't care about cache timeouts here since whats grabbed is already grabbed, might as well put it in, right?
    pub async fn update_all_with_lb(&self) -> DatabaseResult<()> {
        println!("Started updating via leaderboard");
        let response = tetrio::leaderboard::request().await.unwrap();

        for user in response.data.users {
            self.update(user, &response.cache).await?;
        }

        Ok(())
    }

    // Update a list of players with API data
    pub async fn update_player(&self, tetrio_id: &str) -> DatabaseResult<()> {
        let is_cached = self
            .get_player(tetrio_id)
            .await?
            .map_or(false, |p| p.is_cached());

        if !is_cached {
            let (new_data, cache_data) = match tetrio::user::request(tetrio_id).await {
                Ok(response) => (response.data.user, response.cache),
                Err(_) => return Err(DatabaseError::NotFound),
            };

            self.update(new_data, &cache_data).await?;
        }

        Ok(())
    }
}
