use std::env;

use mongodb::{Client, Database};
use serenity::static_assertions::_core::fmt::Formatter;

pub mod discord;
pub mod players;
pub mod registration;

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
