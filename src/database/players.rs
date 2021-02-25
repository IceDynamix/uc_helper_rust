use chrono::SecondsFormat;
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
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerEntry {
    tetrio_id: String,
    discord_id: Option<String>,
    link_timestamp: String,
    tetrio_data: Option<LeaderboardUser>,
    cache_data: Option<CacheData>,
}
