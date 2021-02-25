use std::env;
use std::fmt::Formatter;

use mongodb::{Client, Database};
use tracing::info;

use crate::database::players::PlayerCollection;

const DATABASE_NAME: &str = "uc_helper";

pub mod players;

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
