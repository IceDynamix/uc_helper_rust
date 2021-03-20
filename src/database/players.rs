//! Wrapper for the player collection and methods that can be used to modify the collection
//!
//! Usually contains all ranked players because [`PlayerCollection::update_from_leaderboard()`]
//! adds an entry, even if they are not related to the Underdogs Cup.
//!
//! # Example
//!
//! ```
//! let db = uc_helper_rust::database::connect()?;
//! let player = db.players.get_player_by_tetrio("icedynamix")?;
//! db.players.update_from_leaderboard()?;
//! ```

use bson::{doc, DateTime, Document};
use chrono::{Duration, TimeZone, Utc};
use mongodb::sync::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::database::{DatabaseError, DatabaseResult};
use crate::tetrio;
use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::CacheData;

/// Collection name to use in the MongoDB database
const COLLECTION_NAME: &str = "players";

#[derive(Deserialize, Serialize, Debug, Clone)]
/// Represents an entry as it's saved in the collection
///
/// `discord_id` and `link_timestamp` are `Some`, when [`PlayerCollection::link()`] is executed successfully
///
/// `tetrio_data` and `cache_data` are the fields used to cache responses from the API
pub struct PlayerEntry {
    /// Player's Tetrio ID
    pub tetrio_id: String,
    /// Player's linked Discord ID
    pub discord_id: Option<u64>,
    /// When the Discord ID was linked
    link_timestamp: Option<DateTime>,
    /// The cached Tetrio API user data
    pub tetrio_data: Option<LeaderboardUser>,
    /// Cache data about the Tetrio API user data
    pub cache_data: Option<CacheData>,
}

impl PlayerEntry {
    /// Creates a new user
    pub fn new(tetrio_id: &str, discord_id: Option<u64>) -> PlayerEntry {
        PlayerEntry {
            tetrio_id: tetrio_id.to_string(),
            discord_id,
            link_timestamp: None,
            tetrio_data: None,
            cache_data: None,
        }
    }

    /// Parse a [`bson::Document`] to [`PlayerEntry`]
    pub fn from_document(doc: Document) -> PlayerEntry {
        bson::from_document(doc).expect("bad entry")
    }

    /// Whether the data is considered cached (saved for less than 10 minutes)
    ///
    /// Using [`PlayerEntry::cache_data.cached_until`](`crate::tetrio::CacheData`) is not an option, since the amount
    /// of time that Tetrio caches the data server side for is different between endpoints
    /// (compare user endpoint 1min vs leaderboard endpoint 1h).
    pub fn is_cached(&self) -> bool {
        let cache_timeout = Duration::minutes(45);

        if self.tetrio_data.is_some() {
            if let Some(cache_data) = &self.cache_data {
                let last_cached = Utc.timestamp(cache_data.cached_at / 1000, 0);
                let now = Utc::now();

                if now <= last_cached.checked_add_signed(cache_timeout).unwrap_or(now) {
                    return true;
                }
            }
        }
        false
    }
}

/// Main wrapper for a MongoDB collection to manage players
pub struct PlayerCollection {
    collection: Collection,
}

impl PlayerCollection {
    /// Constructs the wrapper struct for the MongoDB collection
    ///
    /// If the collection does not exist, then it will be created implicitly when a new entry is added.
    pub fn new(database: &Database) -> PlayerCollection {
        PlayerCollection {
            collection: database.collection(COLLECTION_NAME),
        }
    }

    /// Update a player with API data with respect to cached data
    ///
    /// Implicitly adds a new player if they don't already exist, no "add" function required.
    /// This usually only happens when the player is unranked.
    pub fn update_player(&self, tetrio_id: &str) -> DatabaseResult<PlayerEntry> {
        tracing::info!("Updating {}", tetrio_id);
        let previous_entry = self.get_player_by_tetrio(tetrio_id)?;
        let is_cached = previous_entry.map_or(false, |e| e.is_cached());

        if is_cached {
            Ok(self.get_player_by_tetrio(tetrio_id)?.unwrap()) // eh who cares about performance
        } else {
            let (new_data, cache_data) = match tetrio::user::request(tetrio_id) {
                Ok(response) => (response.data.user, response.cache),
                Err(_) => return Err(DatabaseError::NotFound),
            };

            self.update(new_data, &cache_data)
        }
    }

    /// Writes the updated player data to the collection
    ///
    /// Doesn't do any requesting or cache checking, and should thus only be used internally.
    /// You're looking for [`update_player()`] or [`update_from_leaderboard()`] instead.
    fn update(
        &self,
        new_data: LeaderboardUser,
        cache_data: &CacheData,
    ) -> DatabaseResult<PlayerEntry> {
        if self
            .collection
            .count_documents(doc! {"tetrio_id": &new_data._id}, None)
            .unwrap()
            == 0
        {
            tracing::info!("{} not in database, adding as new", new_data.username);
            let player_entry = PlayerEntry::new(&new_data._id, None);
            if self
                .collection
                .insert_one(bson::to_document(&player_entry).unwrap(), None)
                .is_err()
            {
                return Err(DatabaseError::CouldNotPush);
            }
        }

        let tetrio_data_doc = bson::to_document(&new_data).unwrap();
        let cache_data = bson::to_document(&cache_data).unwrap();
        self.collection
            .update_one(
                doc! {"tetrio_id": &new_data._id},
                doc! {"$set":{"tetrio_data": tetrio_data_doc, "cache_data": cache_data}},
                None,
            )
            .expect("could not update player");

        Ok(self.get_player_by_tetrio(&new_data._id)?.unwrap())
    }

    /// Uses the Tetrio leaderboard endpoint to update all currently ranked players
    ///
    /// Relatively efficient, since it only uses a single request to the Tetrio API.
    /// Cache timeouts are ignored here, since what has been requested with the
    /// single request is already requested, so there is no harm in updating it anyway.
    ///
    /// New ranked players will be added and currently ranked players will be updated.
    /// Currently unranked players will not be updated.
    ///
    /// Can take a few minutes to update
    pub fn update_from_leaderboard(&self) -> DatabaseResult<()> {
        tracing::info!("Started updating via leaderboard");
        let response = tetrio::leaderboard::request().map_err(DatabaseError::TetrioApiError)?;

        for user in response.data.users {
            self.update(user, &response.cache)?;
        }

        Ok(())
    }

    /// Creates a link between a Discord user ID and a Tetrio user
    ///
    /// Adds the [`PlayerEntry.discord_id`](PlayerEntry) field.
    ///
    /// Performs duplicate checks to make sure that keys cannot be added in incorrect ways.
    pub fn link(&self, discord_id: u64, tetrio_id: &str) -> DatabaseResult<PlayerEntry> {
        tracing::info!("Linking {} to {}", tetrio_id, discord_id);
        if let Some(entry) = self.get_player_by_discord(discord_id)? {
            let data = entry.tetrio_data.expect("Expected data");
            return if tetrio_id == data._id || tetrio_id == data.username {
                Err(DatabaseError::AlreadyLinked)
            } else {
                Err(DatabaseError::DuplicateDiscordEntry)
            };
        }

        let entry = self.update_player(tetrio_id)?; // if the specified player doesnt exist then this will err

        if entry.discord_id.map_or(false, |id| id != discord_id) {
            return Err(DatabaseError::DuplicateTetrioEntry);
        }

        self.collection
            .update_one(
                doc! {"tetrio_id": entry.tetrio_id},
                doc! {"$set":{"discord_id": discord_id, "link_timestamp": Utc::now()}},
                None,
            )
            .map_err(|_| DatabaseError::CouldNotPush)?;

        Ok(self.get_player_by_discord(discord_id)?.unwrap())
    }

    /// Undoes the link made by [`PlayerCollection.link()`]
    ///
    /// Performs the search via a document filter, should only be used internally.
    /// You're probably looking for [`PlayerCollection.unlink_by_discord()`] or
    /// [`PlayerCollection.unlink_by_tetrio()`] instead.
    fn unlink(&self, filter: Document) -> DatabaseResult<()> {
        self.collection
            .update_one(
                filter,
                doc! {"$unset": {"discord_id": "", "link_timestamp": ""}},
                None,
            )
            .map_err(|_| DatabaseError::CouldNotPush)?;
        Ok(())
    }

    /// Undoes the link made by [`PlayerCollection.link()`] for a specified Tetrio user
    pub fn unlink_by_tetrio(&self, tetrio_id: &str) -> DatabaseResult<()> {
        if let Some(entry) = self.get_player_by_tetrio(tetrio_id)? {
            if entry.discord_id.is_none() {
                Err(DatabaseError::FieldNotSet)
            } else {
                self.unlink(doc! {"tetrio_id": tetrio_id})
            }
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    /// Undoes the link made by [`PlayerCollection.link()`] for a specified Discord user ID
    pub fn unlink_by_discord(&self, discord_id: u64) -> DatabaseResult<()> {
        if self.get_player_by_discord(discord_id)?.is_some() {
            self.unlink(doc! {"discord_id": discord_id})
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    /// Gets current player data for a specified Tetrio user
    pub fn get_player_by_tetrio(&self, tetrio_id: &str) -> DatabaseResult<Option<PlayerEntry>> {
        crate::database::get_entry(
            &self.collection,
            doc! {"$or": [{"tetrio_id": tetrio_id}, {"tetrio_data.username": tetrio_id}]},
        )
    }

    /// Gets current player data for the Tetrio user linked with the specified Discord user ID
    pub fn get_player_by_discord(&self, discord_id: u64) -> DatabaseResult<Option<PlayerEntry>> {
        crate::database::get_entry(&self.collection, doc! {"discord_id": discord_id})
    }

    /// Gets a list of players specified by a document filter
    pub fn get_players(
        &self,
        filter: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<PlayerEntry>> {
        crate::database::get_entries(&self.collection, filter)
    }

    /// Removes players matching a filter from the collection
    ///
    /// Should be used very rarely, since there is no real need to remove any entries.
    pub fn remove_players(&self, filter: Document) -> DatabaseResult<()> {
        tracing::info!("Deleting players with filter {:?}", filter);
        match self.collection.delete_many(filter, None) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }

    /// Wipes all entries from the collection
    ///
    /// Created for testing purposes, don't actually use this on a live database please
    pub fn remove_all(&self) -> DatabaseResult<()> {
        tracing::info!("Deleting the entire collection for some reason??");
        match self.collection.drop(None) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }
}
