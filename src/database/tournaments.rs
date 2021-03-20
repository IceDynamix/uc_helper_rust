//! Wrapper for the tournament collection and methods that can be used to modify the collection
//!
//! There will only be one active tournament at a time, unless the database is edited by hand.
//!
//! # Example
//!
//! ```
//! use uc_helper_rust::database::tournaments;
//! use chrono::{DateTime, Utc, Duration};
//! use uc_helper_rust::tetrio::Rank;
//!
//! let db = uc_helper_rust::database::connect()?;
//!
//! // Update all ranked players
//! db.players.update_from_leaderboard()?;
//!
//! // Create a tournament
//! let restrictions = tournaments::TournamentRestrictions::default();
//! let tournament = db.tournaments.create_tournament("Test Tournament 1", "TT1", restrictions)?;
//!
//! // Set tournament as active
//! db.tournaments.set_active(Some(&tournament.shorthand))?; // Using None would set all tournaments to inactive
//! ```

use std::str::FromStr;

use bson::{doc, DateTime as BsonDateTime, Document};
use chrono::{DateTime, Utc};
use mongodb::sync::{Collection, Database};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::database::players::{PlayerCollection, PlayerEntry};
use crate::database::{DatabaseError, DatabaseResult};
use crate::tetrio;
use crate::tetrio::{leaderboard::LeaderboardUser, Rank};

const COLLECTION_NAME: &str = "tournaments";

type RegistrationResult = Result<(), RegistrationError>;

#[derive(Error, Debug)]
/// Something that prevents a registration attempt from succeeding
///
/// Contains all relevant information to create a meaningful error message
pub enum RegistrationError {
    #[error("Current rank is too high (currently `{rank}`, ≤ `{expected}` required)")]
    /// User's current rank is outside of the restrictions
    CurrentRankTooHigh {
        /// Current rank
        rank: Rank,
        /// Required rank
        expected: Rank,
    },
    #[error(
        "Rank was too high on announcement day (was `{rank}` by `{date}`, ≤ `{expected}` required)"
    )]
    /// User's announcement rank was outside of the restrictions
    AnnouncementRankTooHigh {
        /// Announcement rank
        rank: Rank,
        /// Required rank
        expected: Rank,
        /// Announcement date
        date: DateTime<Utc>,
    },
    #[error("Not enough ranked games played until announcement day (was `{value}` by `{date}`, ≥ `{expected}` required)")]
    /// User's ranked game count is outside of the restrictions
    NotEnoughGames {
        /// Current value
        value: i64,
        /// Expected value
        expected: i64,
        /// Announcement date
        date: DateTime<Utc>,
    },
    #[error(
        "RD was too high at announcement day (was `{value}` by `{date}`, ≥ `{expected}` required)"
    )]
    /// User's rating deviation is outside of the restrictions
    RdTooHigh {
        /// Current value
        value: f64,
        /// Expected value
        expected: f64,
        /// Announcement date
        date: DateTime<Utc>,
    },
    #[error("Player was unranked on announcement day (`{0}`)")]
    /// User was unranked on announcement day
    UnrankedOnAnnouncementDay(DateTime<Utc>),
    #[error("There is no tournament ongoing")]
    /// There is no active tournament
    NoTournamentActive,
    #[error("Something was missing while registering (`{0}`)")]
    /// Missing information to register the user
    MissingArgument(String),
    #[error("Something went wrong while accessing the database: {0}")]
    /// Something happened while accessing the database
    DatabaseError(#[from] DatabaseError),
    #[error("User is already registered")]
    /// User is already registered
    AlreadyRegistered,
    #[error("User is not registered")]
    /// User is not registered (used when unregistering)
    NotRegistered,
    #[error("Player stat snapshot is missing")]
    /// Snapshot is missing
    SnapshotMissing,
}

#[derive(Deserialize, Serialize, Debug)]
/// Contains tournament registration restrictions
pub struct TournamentRestrictions {
    /// Highest announcement rank a user is allowed to have in order to register
    ///
    /// Current rank will always be `max_rank + 1`
    pub max_rank: Rank,
    /// Highest rating deviation a user is allowed to have in order to register
    pub max_rd: f64,
    /// Minimum amount of played ranked games a user needs to have to have in order to register
    pub min_ranked_games: i64,
}

impl TournamentRestrictions {
    /// Creates tournament restrictions
    pub fn new(max_rank: Rank, max_rd: f64, min_ranked_games: i64) -> TournamentRestrictions {
        TournamentRestrictions {
            max_rank,
            max_rd,
            min_ranked_games,
        }
    }
}

impl Default for TournamentRestrictions {
    fn default() -> Self {
        TournamentRestrictions::new(Rank::Unranked, 999f64, 0)
    }
}

#[derive(Deserialize, Serialize, Debug)]
/// Represents a registration in a tournament entry
pub struct RegistrationEntry {
    date: BsonDateTime,
    tetrio_id: String,
}

impl RegistrationEntry {
    /// Creates a new registration entry
    pub fn new(tetrio_id: &str) -> RegistrationEntry {
        RegistrationEntry {
            date: BsonDateTime::from(Utc::now()),
            tetrio_id: tetrio_id.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
/// Represents an entry as it's saved in the collection
pub struct TournamentEntry {
    /// Name of the tournament, expected to be unique
    /// TODO: Make sure it is unique
    pub name: String,
    /// Shorthand or abbreviation of the tournament, expected to be unique
    /// TODO: Make sure it is unique
    pub shorthand: String,
    /// When the tournament entry was created
    created_at: BsonDateTime,
    /// Tournament registration restrictions
    pub restrictions: TournamentRestrictions,
    /// List of registrations
    pub registered_players: Vec<RegistrationEntry>,
    /// Snapshot of stats to use for checking announcement stats (refer to [`TournamentCollection::add_snapshot()`])
    player_stats_snapshot: Vec<LeaderboardUser>,
    /// When the snapshot was made
    snapshot_at: Option<BsonDateTime>,
    /// Whether the tournament is active right now
    active: bool,
    /// Check-in message
    pub check_in_msg: Option<u64>,
}

impl TournamentEntry {
    /// Creates a new tournament entry
    pub fn new(
        name: &str,
        shorthand: &str,
        restrictions: TournamentRestrictions,
    ) -> TournamentEntry {
        TournamentEntry {
            name: name.to_string(),
            shorthand: shorthand.to_string(),
            created_at: BsonDateTime::from(Utc::now()),
            restrictions,
            registered_players: Vec::new(),
            player_stats_snapshot: Vec::new(),
            snapshot_at: None,
            active: false,
            check_in_msg: None,
        }
    }

    /// Verify whether a player can participate in this tournament
    ///
    /// Uses snapshot data, so [`TournamentCollection::add_snapshot()`] must have been called at least
    /// once before.
    fn check_player_stats(&self, current_data: &LeaderboardUser) -> RegistrationResult {
        let snapshot_at = match self.snapshot_at {
            None => return Err(RegistrationError::SnapshotMissing),
            Some(ts) => *ts,
        };

        let snapshot_data = self
            .player_stats_snapshot
            .iter()
            .find(|u| current_data._id == u._id);

        match snapshot_data {
            None => Err(RegistrationError::UnrankedOnAnnouncementDay(snapshot_at)),
            Some(snap) => {
                let announce_rank = Rank::from_str(&snap.league.rank).unwrap();
                if announce_rank > self.restrictions.max_rank {
                    return Err(RegistrationError::AnnouncementRankTooHigh {
                        rank: announce_rank,
                        expected: self.restrictions.max_rank,
                        date: snapshot_at,
                    });
                }

                let games_played = snap.league.gamesplayed;
                if games_played < self.restrictions.min_ranked_games {
                    return Err(RegistrationError::NotEnoughGames {
                        value: games_played,
                        expected: self.restrictions.min_ranked_games,
                        date: snapshot_at,
                    });
                }

                let rd = snap.league.rd.unwrap_or(999f64);
                if rd > self.restrictions.max_rd {
                    return Err(RegistrationError::RdTooHigh {
                        value: rd,
                        expected: self.restrictions.max_rd,
                        date: snapshot_at,
                    });
                }

                let current_rank = Rank::from_str(&current_data.league.rank).unwrap();
                if current_rank > self.restrictions.max_rank + 1 {
                    return Err(RegistrationError::CurrentRankTooHigh {
                        rank: current_rank,
                        expected: self.restrictions.max_rank + 1,
                    });
                }

                Ok(())
            }
        }
    }

    /// Whether a user is registered to this tournament or not
    pub fn player_is_registered(&self, player: &PlayerEntry) -> bool {
        self.registered_players
            .iter()
            .any(|entry| entry.tetrio_id == player.tetrio_id)
    }
}

/// Main wrapper for a MongoDB collection to manage tournaments
pub struct TournamentCollection {
    collection: Collection,
}

impl TournamentCollection {
    /// Constructs the wrapper struct for the MongoDB collection
    ///
    /// If the collection does not exist, then it will be created implicitly when a new entry is added.
    pub fn new(database: &Database) -> TournamentCollection {
        TournamentCollection {
            collection: database.collection(COLLECTION_NAME),
        }
    }

    /// Create a tournament entry with specified information
    pub fn create_tournament(
        &self,
        name: &str,
        shorthand: &str,
        restrictions: TournamentRestrictions,
    ) -> DatabaseResult<TournamentEntry> {
        tracing::info!("Creating tournament {} ({})", name, shorthand);
        let entry = TournamentEntry::new(name, shorthand, restrictions);
        match self.collection.insert_one(
            bson::to_document(&entry).expect("could not convert to document"),
            None,
        ) {
            Ok(_) => Ok(entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    /// Gets a tournament by name or shorthand
    pub fn get_tournament(&self, name: &str) -> DatabaseResult<Option<TournamentEntry>> {
        crate::database::get_entry(
            &self.collection,
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
        )
    }

    /// Registers a player to the active tournament
    ///
    /// Will call [`PlayerCollection::link()`] internally, so the player is always linked.
    /// If no username is given, then it will try to use the linked player.
    pub fn register_to_active(
        &self,
        players: &PlayerCollection,
        tetrio_id: Option<&str>,
        discord_id: u64,
    ) -> Result<PlayerEntry, RegistrationError> {
        let tournament = match self.get_active()? {
            Some(t) => t,
            None => {
                return Err(RegistrationError::NoTournamentActive);
            }
        };

        // Use the linked player if no username is provided
        // Link already takes care of the cases where tetrio id or discord id do not match
        let player = match tetrio_id {
            None => match players.get_player_by_discord(discord_id)? {
                Some(linked_entry) => linked_entry,
                None => {
                    return Err(RegistrationError::MissingArgument("username".to_string()));
                }
            },
            Some(id) => match players.link(discord_id, id) {
                Ok(new_entry) => new_entry,
                Err(err) => match err {
                    DatabaseError::AlreadyLinked => {
                        players.get_player_by_discord(discord_id)?.unwrap()
                    }
                    _ => {
                        return Err(RegistrationError::DatabaseError(err));
                    }
                },
            },
        };

        let stats = player.tetrio_data.unwrap();
        tracing::info!(
            "Registering {} to tournament {}",
            &stats.username,
            tournament.name
        );

        // throws an error if invalid
        tournament.check_player_stats(&stats)?;

        let tetrio_id = player.tetrio_id;
        if tournament
            .registered_players
            .iter()
            .any(|entry| entry.tetrio_id == tetrio_id)
        {
            return Err(RegistrationError::AlreadyRegistered);
        }

        let reg_entry =
            bson::to_document(&RegistrationEntry::new(&tetrio_id)).expect("bad document");

        self.collection
            .update_one(
                doc! {"active": true},
                doc! {"$push": {"registered_players": reg_entry}},
                None,
            )
            .map_err(|_| RegistrationError::DatabaseError(DatabaseError::CouldNotPush))?;

        Ok(players.get_player_by_discord(discord_id)?.unwrap())
    }

    /// Unregisters a player from the current tournament
    ///
    /// Function to be used internally, you're probably looking for
    /// [`unregister_by_tetrio()`] or [`unregister_by_discord()`]
    fn unregister(&self, player: &PlayerEntry, tournament: &TournamentEntry) -> RegistrationResult {
        if tournament
            .registered_players
            .iter()
            .find(|reg| reg.tetrio_id == player.tetrio_id)
            .is_none()
        {
            return Err(RegistrationError::NotRegistered);
        }

        tracing::info!(
            "Unregistering {} from tournament {}",
            &player.tetrio_id,
            tournament.name
        );

        if self
            .collection
            .update_one(
                doc! {"shorthand": &tournament.shorthand},
                doc! {"$pull": {"registered_players": {"tetrio_id": &player.tetrio_id}}},
                None,
            )
            .is_err()
        {
            return Err(RegistrationError::DatabaseError(
                DatabaseError::CouldNotPush,
            ));
        }

        Ok(())
    }

    /// Unregisters a player specified by username or ID from the active tournament
    pub fn unregister_by_tetrio(
        &self,
        players: &PlayerCollection,
        tetrio_id: &str,
    ) -> RegistrationResult {
        let tournament = match self.get_active()? {
            Some(t) => t,
            None => {
                return Err(RegistrationError::NoTournamentActive);
            }
        };

        let specified = match players.get_player_by_tetrio(tetrio_id)? {
            Some(p) => p,
            None => return Err(RegistrationError::DatabaseError(DatabaseError::NotFound)),
        };

        self.unregister(&specified, &tournament)
    }

    /// Unregisters a player specified by Discord ID from the active tournament
    pub fn unregister_by_discord(
        &self,
        players: &PlayerCollection,
        discord_id: u64,
    ) -> RegistrationResult {
        let tournament = match self.get_active()? {
            Some(t) => t,
            None => {
                return Err(RegistrationError::NoTournamentActive);
            }
        };

        let specified = match players.get_player_by_discord(discord_id)? {
            Some(p) => p,
            None => return Err(RegistrationError::DatabaseError(DatabaseError::NotFound)),
        };

        self.unregister(&specified, &tournament)
    }

    /// Adds a stat snapshot of the current leaderboard entry to a specified tournament
    ///
    /// This data is used to compare announcement stats when registering.
    /// It's around 4MB in size (as measured in March 2021), so hitting a size
    /// limit with MongoDB Atlas (512MB min.) is unlikely, unless hundreds of snapshots are saved.
    pub fn add_snapshot(&self, name: &str) -> DatabaseResult<()> {
        if self.get_tournament(name)?.is_none() {
            return Err(DatabaseError::NotFound);
        }

        tracing::info!("Adding stat snapshot for tournament {}", name);

        // Will ensure that unranked players are not in the snapshot and are therefore easy to identify,
        // since the players collection doesn't remove them when they become unranked
        let snapshot: Vec<Document> = match tetrio::leaderboard::request() {
            Ok(response) => response.data.users,
            Err(e) => return Err(DatabaseError::TetrioApiError(e)),
        }
        .iter()
        .map(|u| bson::to_document(u).expect("Bad document"))
        .collect();

        match self.collection.update_one(
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
            doc! {"$set": {"player_stats_snapshot": &snapshot, "snapshot_at": Utc::now()}},
            None,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    /// Set a specified tournament as active
    ///
    /// If `None` is passed, then it will set all tournaments as inactive.
    pub fn set_active(&self, name: Option<&str>) -> DatabaseResult<Option<TournamentEntry>> {
        let tournament = if let Some(name) = name {
            match self.get_tournament(name)? {
                Some(t) => Some(t),
                None => return Err(DatabaseError::NotFound),
            }
        } else {
            None
        };

        // set all inactive
        if self
            .collection
            .update_many(doc! {}, doc! {"$set": {"active": false}}, None)
            .is_err()
        {
            return Err(DatabaseError::CouldNotPush);
        }

        tracing::info!("Set all tournaments to inactive");

        // set specified tournament active
        if let Some(tournament) = &tournament {
            if self
                .collection
                .update_one(
                    doc! {"name": &tournament.name},
                    doc! {"$set": {"active": true}},
                    None,
                )
                .is_err()
            {
                return Err(DatabaseError::CouldNotPush);
            }
            tracing::info!("Set tournament {} to active", tournament.name);
        }

        Ok(tournament)
    }

    /// Get the currently active tournament
    pub fn get_active(&self) -> DatabaseResult<Option<TournamentEntry>> {
        crate::database::get_entry(&self.collection, doc! {"active": true})
    }

    /// Set a check-in message for a tournament
    pub fn set_check_in_msg(&self, name: &str, message_id: u64) -> DatabaseResult<()> {
        if self.get_tournament(name)?.is_none() {
            return Err(DatabaseError::NotFound);
        }

        match self.collection.update_one(
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
            doc! {"$set": {"check_in_msg": message_id}},
            None,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }
}
