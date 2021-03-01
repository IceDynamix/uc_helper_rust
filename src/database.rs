//! Database management with [`mongodb`]
//!
//! No data is mutated locally, so everything can be called and savedc without `mut`.
//! All modifications are done directly to the database with functions.
//!
//! # Example
//!
//! ```
//! let db = uc_helper_rust::database::connect()?;
//! let player = db.players.get_player_by_tetrio("icedynamix")?;
//! let tournament = db.tournaments.get_tournament("UC7")?;
//! ```

#![warn(missing_docs)]

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

pub mod players;
pub mod tournaments;

/// Database name to use in MongoDB
const DATABASE_NAME: &str = "uc_helper";

type DatabaseResult<T> = Result<T, DatabaseError>;

/// Generic function that finds an entry and parses it into a given structure
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

/// Generic function that finds a list of entries and parses them into a given structure
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
/// Something that can go wrong during database access
pub enum DatabaseError {
    #[error("Connection to database failed")]
    /// Connection to database could not be established
    ConnectionFailed,
    #[error("Could not find item")]
    /// Some item was not present in the database
    NotFound,
    #[error("Could not push to database")]
    /// Pushing an update to the database was not possible
    CouldNotPush,
    #[error("Tetrio user already exists")]
    /// A Tetrio user is already in the database
    DuplicateTetrioEntry,
    #[error("Discord user already exists")]
    /// A Discord user is already in the database
    DuplicateDiscordEntry,
    #[error("Could not parse document to entry: {0}")]
    /// Could not parse document to entry
    CouldNotParse(String),
    #[error("A specific field was not set")]
    /// A document field was not set
    FieldNotSet,
    #[error("Something happened while requesting data from the Tetrio API")]
    /// Tetrio API Error
    TetrioApiError(#[from] TetrioApiError),
    #[error("User is trying to link user that's already linked to them")]
    /// User is trying to link themself to the same person
    AlreadyLinked,
}

/// Represents the database and provides access to the wrapped collections
pub struct LocalDatabase {
    _database: Database,
    /// Represents the player collection
    pub players: PlayerCollection,
    /// Represents the tournament collection
    pub tournaments: TournamentCollection,
}

/// Establishes a connection to MongoDB database as provided by the `DATABASE_URL` environment variable.
pub fn connect() -> Result<LocalDatabase, DatabaseError> {
    let url = env::var("DATABASE_URL").expect("url must be set");
    info!("Connecting to database");
    let client = Client::with_uri_str(&url).map_err(|_| DatabaseError::ConnectionFailed)?;

    let database = client.database(DATABASE_NAME);

    Ok(LocalDatabase {
        players: PlayerCollection::new(&database),
        tournaments: TournamentCollection::new(&database),
        _database: database,
    })
}

/// Used to make a single database connection sharable during Discord bot runtime
impl TypeMapKey for LocalDatabase {
    type Value = Arc<LocalDatabase>;
}
