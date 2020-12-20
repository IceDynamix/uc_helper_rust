use chrono::SecondsFormat;
use mongodb::bson;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::database::{establish_db_connection, DatabaseError};
use crate::tetrio;

const COLLECTION: &str = "discord";

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordEntry {
    pub discord_id: String,
    pub tetrio_id: String,
    pub timestamp: String,
}

pub async fn link(discord_id: u64, username: &str) -> Result<tetrio::User, DatabaseError> {
    let now = chrono::offset::Utc::now();
    let timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let discord_id_str = discord_id.to_string();

    let tetrio_user = match tetrio::User::request(username).await {
        None => return Err(DatabaseError::NotFound),
        Some(user) => user,
    };

    let collection = establish_db_connection().await?.collection(COLLECTION);
    let existing_users_with_id = collection
        .find_one(doc! {"tetrio_id": tetrio_user._id.clone()}, None)
        .await;

    match existing_users_with_id {
        Ok(result) => {
            if result.is_some() {
                return Err(DatabaseError::DuplicateEntry);
            }
        }
        Err(_) => return Err(DatabaseError::CouldNotPush),
    }

    let entry = DiscordEntry {
        discord_id: discord_id_str.to_owned(),
        tetrio_id: tetrio_user._id.clone(),
        timestamp,
    };
    let bson_entry = bson::to_document(&entry).unwrap();

    match collection.insert_one(bson_entry, None).await {
        Ok(_) => Ok(tetrio_user),
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}

pub async fn unlink(discord_id: u64) -> Result<(), DatabaseError> {
    let collection = establish_db_connection().await?.collection(COLLECTION);
    match collection
        .find_one_and_delete(doc! {"discord_id": discord_id.to_string()}, None)
        .await
    {
        Ok(result) => match result {
            Some(_) => Ok(()),
            None => Err(DatabaseError::NotFound),
        },
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}

pub async fn get_from_discord_id(discord_id: u64) -> Result<DiscordEntry, DatabaseError> {
    get(doc! {"discord_id": discord_id.to_string()}).await
}

pub async fn get_from_tetrio(tetrio: &str) -> Result<DiscordEntry, DatabaseError> {
    let result = crate::database::players::get(tetrio).await?;
    let id = result._id;
    get(doc! {"tetrio_id": id}).await
}

async fn get(filter: bson::Document) -> Result<DiscordEntry, DatabaseError> {
    let collection = establish_db_connection().await?.collection(COLLECTION);
    match collection.find_one(filter, None).await {
        Ok(result) => match result {
            Some(doc) => match bson::from_document::<DiscordEntry>(doc) {
                Ok(entry) => Ok(entry),
                Err(_) => Err(DatabaseError::NotFound),
            },
            None => Err(DatabaseError::NotFound),
        },
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}
