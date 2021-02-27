use std::env;
use std::sync::Arc;

use bson::Document;
use mongodb::sync::{Client, Collection, Database};
use serde::de::DeserializeOwned;
use serenity::prelude::TypeMapKey;
use thiserror::Error;
use tracing::info;

use crate::database::players::PlayerCollection;
use crate::database::tournaments::TournamentCollection;
use crate::tetrio::TetrioApiError;

const DATABASE_NAME: &str = "uc_helper";

type DatabaseResult<T> = Result<T, DatabaseError>;

pub mod players;
pub mod tournaments;

// too lazy to implement async traits
fn get_entry<T: DeserializeOwned>(
    collection: &Collection,
    filter: impl Into<Option<Document>>,
) -> DatabaseResult<Option<T>> {
    match collection.find_one(filter, None) {
        Ok(entry) => {
            let doc: Option<Document> = entry;
            Ok(doc.map(|d| bson::from_document(d).expect("could not convert to document")))
        }
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

fn get_entries<T: DeserializeOwned>(
    collection: &Collection,
    filter: impl Into<Option<Document>>,
) -> DatabaseResult<Vec<T>> {
    match collection.find(filter, None) {
        Ok(result) => Ok(result
            .map(|doc| {
                bson::from_document(doc.expect("bad entry")).expect("could not convert to document")
            })
            .collect()),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection to database failed")]
    ConnectionFailed,
    #[error("Could not find item")]
    NotFound,
    #[error("Could not push to database")]
    CouldNotPush,
    #[error("Tetrio user already exists")]
    DuplicateTetrioEntry,
    #[error("Discord user already exists")]
    DuplicateDiscordEntry,
    #[error("Could not parse document to entry: {0}")]
    CouldNotParse(String),
    #[error("A specific field was not set")]
    FieldNotSet,
    #[error("Something happened while requesting data from the Tetrio API")]
    TetrioApiError(#[from] TetrioApiError),
}

pub struct LocalDatabase {
    _database: Database,
    pub players: PlayerCollection,
    pub tournaments: TournamentCollection,
}

impl LocalDatabase {
    pub fn connect() -> Result<LocalDatabase, DatabaseError> {
        let url = env::var("DATABASE_URL").expect("url must be set");
        info!("Connecting to database");
        let client = Client::with_uri_str(&url);

        if client.is_err() {
            return Err(DatabaseError::ConnectionFailed);
        }

        let database = client.unwrap().database(DATABASE_NAME);

        Ok(LocalDatabase {
            players: PlayerCollection::new(&database),
            tournaments: TournamentCollection::new(&database),
            _database: database,
        })
    }
}

impl TypeMapKey for LocalDatabase {
    type Value = Arc<LocalDatabase>;
}
