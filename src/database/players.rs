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
        tetrio_id: String,
        discord_id: Option<String>,
    ) -> DatabaseResult<PlayerEntry> {
        info!("Adding {} to players", tetrio_id);
        let now = chrono::offset::Utc::now();
        let link_timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);

        let filter: Document = match discord_id.clone() {
            None => doc! {"tetrio_id": tetrio_id.clone()},
            Some(id) => doc! {"$or": [{"tetrio_id": tetrio_id.clone()}, {"discord_id": id}]},
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
            tetrio_id,
            discord_id,
            link_timestamp,
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
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerEntry {
    tetrio_id: String,
    discord_id: Option<String>,
    link_timestamp: String,
}
