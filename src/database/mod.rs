use std::env;
use std::fmt::Formatter;

use bson::Document;
use mongodb::{Client, Collection, Database};
use serde::de::DeserializeOwned;
use tokio::stream::StreamExt;
use tracing::info;

use crate::database::players::PlayerCollection;

const DATABASE_NAME: &str = "uc_helper";

type DatabaseResult<T> = Result<T, DatabaseError>;

pub mod players;

// too lazy to implement async traits
async fn get_entry<T: DeserializeOwned>(
    collection: &Collection,
    filter: impl Into<Option<Document>>,
) -> DatabaseResult<Option<T>> {
    match collection.find_one(filter, None).await {
        Ok(entry) => {
            let doc: Option<Document> = entry;
            Ok(doc.map(|d| bson::from_document(d).expect("could not convert to document")))
        }
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

async fn get_entries<T: DeserializeOwned>(
    collection: &Collection,
    filter: impl Into<Option<Document>>,
) -> DatabaseResult<Vec<T>> {
    match collection.find(filter, None).await {
        Ok(result) => Ok(result
            .map(|doc| {
                bson::from_document(doc.expect("bad entry")).expect("could not convert to document")
            })
            .collect()
            .await),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionFailed,
    NotFound,
    CouldNotPush,
    DuplicateEntry,
    CouldNotParse(String),
    FieldNotSet,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionFailed => f.write_str("ConnectionFailed"),
            DatabaseError::NotFound => f.write_str("NotFound"),
            DatabaseError::CouldNotPush => f.write_str("CouldNotPush"),
            DatabaseError::DuplicateEntry => f.write_str("DuplicateEntry"),
            DatabaseError::CouldNotParse(e) => f.write_str(e),
            DatabaseError::FieldNotSet => f.write_str("FieldNotSet"),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn description(&self) -> &str {
        match self {
            DatabaseError::ConnectionFailed => "Connection to database failed",
            DatabaseError::NotFound => "Could not find item",
            DatabaseError::CouldNotPush => "Could not push to database",
            DatabaseError::DuplicateEntry => "Item already exists",
            DatabaseError::CouldNotParse(_) => "Could not parse document to entry",
            DatabaseError::FieldNotSet => "A specific field was not set",
        }
    }
}

pub struct LocalDatabase {
    database: Database,
    pub players: PlayerCollection,
}

impl LocalDatabase {
    pub async fn connect() -> Result<LocalDatabase, DatabaseError> {
        let url = env::var("DATABASE_URL").expect("url must be set");
        info!("Connecting to database");
        let client = Client::with_uri_str(&url).await;

        if client.is_err() {
            return Err(DatabaseError::ConnectionFailed);
        }

        let database = client.unwrap().database(DATABASE_NAME);

        Ok(LocalDatabase {
            players: PlayerCollection::new(&database),
            database,
        })
    }
}
