use std::env;
use std::fmt::Formatter;

use mongodb::{Client, Database};
use tracing::info;

const DATABASE_NAME: &str = "uc_helper";

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

pub async fn establish_connection() -> Result<Database, DatabaseError> {
    let url = env::var("DATABASE_URL").expect("url must be set");
    info!("Connecting to database");
    match Client::with_uri_str(&url).await {
        Ok(client) => Ok(client.database(DATABASE_NAME)),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}
