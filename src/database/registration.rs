use chrono::SecondsFormat;
use mongodb::bson;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::database::{establish_db_connection, DatabaseError};

const COLLECTION: &str = "uc6_participants";

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistrationEntry {
    pub discord_id: String,
    pub tetrio_id: String,
    pub timestamp: String,
}

pub async fn register(discord_id: u64, tetrio_id: &str) -> Result<(), DatabaseError> {
    let now = chrono::offset::Utc::now();
    let timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);

    let collection = establish_db_connection().await?.collection(COLLECTION);
    let existing_users_with_id = collection
        .find_one(
            doc! {"$or": [{"discord_id": discord_id},{"tetrio_id": tetrio_id}]},
            None,
        )
        .await;

    match existing_users_with_id {
        Ok(result) => {
            if result.is_some() {
                return Err(DatabaseError::DuplicateEntry);
            }
        }
        Err(_) => return Err(DatabaseError::CouldNotPush),
    }

    let entry = RegistrationEntry {
        discord_id: discord_id.to_string(),
        tetrio_id: tetrio_id.to_string(),
        timestamp,
    };
    let bson_entry = bson::to_document(&entry).unwrap();

    match collection.insert_one(bson_entry, None).await {
        Ok(_) => Ok(()),
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}

pub async fn unregister_discord(discord_id: u64) -> Result<(), DatabaseError> {
    unregister(doc! {"discord_id": discord_id.to_string()}).await
}

pub async fn unregister_tetrio(tetrio_id: &str) -> Result<(), DatabaseError> {
    unregister(doc! {"tetrio_id": tetrio_id}).await
}

// TODO: yeet overrankers
async fn unregister(filter: bson::Document) -> Result<(), DatabaseError> {
    let collection = establish_db_connection().await?.collection(COLLECTION);
    match collection.find_one_and_delete(filter, None).await {
        Ok(result) => match result {
            Some(_) => Ok(()),
            None => Err(DatabaseError::NotFound),
        },
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}
