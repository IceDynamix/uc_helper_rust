use bson::{doc, DateTime, Document};
use chrono::{Duration, TimeZone, Utc};
use mongodb::sync::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::database::{DatabaseError, DatabaseResult};
use crate::tetrio;
use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::CacheData;

const COLLECTION_NAME: &str = "players";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PlayerEntry {
    pub tetrio_id: String,
    pub discord_id: Option<u64>,
    // mongodb cant actually save unsigned integers to their full range but it'll be *fineeeeeeee*
    link_timestamp: Option<DateTime>,
    pub tetrio_data: Option<LeaderboardUser>,
    pub cache_data: Option<CacheData>,
}

impl PlayerEntry {
    pub fn new(tetrio_id: &str, discord_id: Option<u64>) -> PlayerEntry {
        PlayerEntry {
            tetrio_id: tetrio_id.to_string(),
            discord_id,
            link_timestamp: None,
            tetrio_data: None,
            cache_data: None,
        }
    }

    pub fn from_document(doc: Document) -> PlayerEntry {
        bson::from_document(doc).expect("bad entry")
    }

    // Returns true if not cached or cached for longer than 10 minutes
    // We can't simply use cache.cached_until, since leaderboard data is cached for 1h, while regular user data is cached for 1min
    pub fn is_cached(&self) -> bool {
        let cache_timeout = Duration::minutes(10);

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

pub struct PlayerCollection {
    collection: Collection,
}

impl PlayerCollection {
    pub fn new(database: &Database) -> PlayerCollection {
        PlayerCollection {
            collection: database.collection(COLLECTION_NAME),
        }
    }

    // Update a player with API data with respect to cached data
    // Implicitly adds a new player if they don't already exist, no add function required
    // The only situation a user doesn't already exist is when they are unranked or got ranked before the hourly leaderboard update hit
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

    // Writes the updated data to the database
    // Doesnt do any requesting or cache checking, and should thus only be used internally
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

    // Uses leaderboard data to write to the database so only a single request is used
    // We don't care about cache timeouts here since whats grabbed with that one request is already grabbed, might as well put it in, right?
    pub fn update_from_leaderboard(&self) -> DatabaseResult<()> {
        tracing::info!("Started updating via leaderboard");
        let response = tetrio::leaderboard::request().map_err(DatabaseError::TetrioApiError)?;

        for user in response.data.users {
            self.update(user, &response.cache)?;
        }

        Ok(())
    }

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

    pub fn unlink_by_discord(&self, discord_id: u64) -> DatabaseResult<()> {
        if self.get_player_by_discord(discord_id)?.is_some() {
            self.unlink(doc! {"discord_id": discord_id})
        } else {
            Err(DatabaseError::NotFound)
        }
    }

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

    pub fn get_player_by_tetrio(&self, tetrio_id: &str) -> DatabaseResult<Option<PlayerEntry>> {
        crate::database::get_entry(
            &self.collection,
            doc! {"$or": [{"tetrio_id": tetrio_id}, {"tetrio_data.username": tetrio_id}]},
        )
    }

    pub fn get_player_by_discord(&self, discord_id: u64) -> DatabaseResult<Option<PlayerEntry>> {
        crate::database::get_entry(&self.collection, doc! {"discord_id": discord_id})
    }

    pub fn get_players(
        &self,
        filter: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<PlayerEntry>> {
        crate::database::get_entries(&self.collection, filter)
    }

    pub fn remove_players(&self, filter: Document) -> DatabaseResult<()> {
        tracing::info!("Deleting players with filter {:?}", filter);
        match self.collection.delete_many(filter, None) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }

    pub fn remove_all(&self) -> DatabaseResult<()> {
        tracing::info!("Deleting the entire collection for some reason??");
        match self.collection.drop(None) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ConnectionFailed),
        }
    }
}
