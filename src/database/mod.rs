use std::env;

use mongodb::bson::doc;
use mongodb::{bson, Client, Database};
use serde::de::DeserializeOwned;
use serenity::static_assertions::_core::fmt::Formatter;
use tokio::stream::StreamExt;

pub mod discord;
pub mod players;
pub mod tournament;

const DATABASE: &str = "uc_helper";

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionFailed,
    NotFound,
    CouldNotPush,
    DuplicateEntry,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            DatabaseError::ConnectionFailed => f.write_str("ConnectionFailed"),
            DatabaseError::NotFound => f.write_str("NotFound"),
            DatabaseError::CouldNotPush => f.write_str("CouldNotPush"),
            DatabaseError::DuplicateEntry => f.write_str("DuplicateEntry"),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn description(&self) -> &str {
        match *self {
            DatabaseError::ConnectionFailed => "Connection to database failed",
            DatabaseError::NotFound => "Could not find item",
            DatabaseError::CouldNotPush => "Could not push to database",
            DatabaseError::DuplicateEntry => "Item already exists",
        }
    }
}

async fn establish_db_connection() -> Result<Database, DatabaseError> {
    let url = env::var("DATABASE_URL").expect("url must be set");
    match Client::with_uri_str(&url).await {
        Ok(client) => Ok(client.database(DATABASE)),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

pub async fn get_all<T: DeserializeOwned>(collection: &str) -> Result<Vec<T>, DatabaseError> {
    let collection = establish_db_connection().await?.collection(&collection);
    let cursor = collection.find(doc! {}, None).await;
    match cursor {
        Ok(results) => Ok(results
            .map(|entry| bson::from_document(entry.expect("bad entry")).expect("bad entry"))
            .collect()
            .await),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}
