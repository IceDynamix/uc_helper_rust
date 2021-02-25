use chrono::SecondsFormat;
use mongodb::{bson, Collection, Database};
use serde::{Deserialize, Serialize};

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

    pub async fn add_player(&self, tetrio_id: String, discord_id: Option<String>) -> PlayerEntry {
        let now = chrono::offset::Utc::now();
        let link_timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);

        let player_entry = PlayerEntry {
            tetrio_id,
            discord_id,
            link_timestamp,
        };

        self.collection
            .insert_one(bson::to_document(&player_entry).unwrap(), None)
            .await
            .unwrap();

        player_entry
    }
}

#[derive(Deserialize, Serialize)]
pub struct PlayerEntry {
    tetrio_id: String,
    discord_id: Option<String>,
    link_timestamp: String,
}
