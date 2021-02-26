use std::fmt::Formatter;

use bson::{doc, DateTime as BsonDateTime, Document};
use chrono::{DateTime, Utc};
use mongodb::sync::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::database::{DatabaseError, DatabaseResult, LocalDatabase};
use crate::tetrio;
use crate::tetrio::{leaderboard::LeaderboardUser, Rank};

const COLLECTION_NAME: &str = "tournaments";

type RegistrationResult = Result<(), RegistrationError>;

#[derive(Debug)]
pub enum RegistrationError {
    CurrentRankTooHigh(Rank),
    AnnouncementRankTooHigh(Rank),
    NotEnoughGames(i64),
    RdTooHigh(f64),
    UnrankedOnAnnouncementDay,
    NoTournamentActive,
    MissingArgument,
    DatabaseError(DatabaseError),
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::CurrentRankTooHigh(_) => f.write_str("CurrentRankTooHigh"),
            RegistrationError::AnnouncementRankTooHigh(_) => f.write_str("AnnouncementRankTooHigh"),
            RegistrationError::NotEnoughGames(_) => f.write_str("NotEnoughGames"),
            RegistrationError::RdTooHigh(_) => f.write_str("RdTooHigh"),
            RegistrationError::UnrankedOnAnnouncementDay => {
                f.write_str("UnrankedOnAnnouncementDay")
            }
            RegistrationError::NoTournamentActive => f.write_str("NoTournamentActive"),
            RegistrationError::MissingArgument => f.write_str("MissingArgument"),
            RegistrationError::DatabaseError(_) => f.write_str("DatabaseError"),
        }
    }
}

impl std::error::Error for RegistrationError {
    fn description(&self) -> &str {
        match self {
            RegistrationError::CurrentRankTooHigh(_) => "Current rank is too high",
            RegistrationError::AnnouncementRankTooHigh(_) => {
                "Rank was too high on announcement day"
            }
            RegistrationError::NotEnoughGames(_) => "Not enough games played by announcement day",
            RegistrationError::RdTooHigh(_) => "RD was too high at announcement day",
            RegistrationError::UnrankedOnAnnouncementDay => {
                "Player was unranked on announcement day"
            }
            RegistrationError::NoTournamentActive => "There is no tournament ongoing",
            RegistrationError::MissingArgument => "Something was missing while registering",
            RegistrationError::DatabaseError(_) => {
                "Something went wrong while accessing the database"
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TournamentDates {
    announcement_at: BsonDateTime,
    registration_end: BsonDateTime,
    check_in_start: BsonDateTime,
    check_in_end: BsonDateTime,
}

impl TournamentDates {
    pub fn new(
        announcement_at: DateTime<Utc>,
        registration_end: DateTime<Utc>,
        check_in_start: DateTime<Utc>,
        check_in_end: DateTime<Utc>,
    ) -> TournamentDates {
        TournamentDates {
            announcement_at: BsonDateTime::from(announcement_at),
            registration_end: BsonDateTime::from(registration_end),
            check_in_start: BsonDateTime::from(check_in_start),
            check_in_end: BsonDateTime::from(check_in_end),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TournamentRestrictions {
    pub min_ranked_games: i64,
    pub max_rd: f64,
    pub max_rank: Rank,
}

impl TournamentRestrictions {
    pub fn new(min_ranked_games: i64, max_rd: f64, max_rank: Rank) -> TournamentRestrictions {
        TournamentRestrictions {
            min_ranked_games,
            max_rd,
            max_rank,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegistrationEntry {
    date: BsonDateTime,
    tetrio_id: String,
}

impl RegistrationEntry {
    pub fn new(tetrio_id: &str) -> RegistrationEntry {
        RegistrationEntry {
            date: BsonDateTime::from(Utc::now()),
            tetrio_id: tetrio_id.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TournamentEntry {
    name: String,
    shorthand: String,
    created_at: BsonDateTime,
    dates: TournamentDates,
    restrictions: TournamentRestrictions,
    registered_players: Vec<RegistrationEntry>,
    // tetrio ids
    player_stats_snapshot: Vec<LeaderboardUser>,
}

impl TournamentEntry {
    pub fn new(
        name: &str,
        shorthand: &str,
        dates: TournamentDates,
        restrictions: TournamentRestrictions,
    ) -> TournamentEntry {
        TournamentEntry {
            name: name.to_string(),
            shorthand: shorthand.to_string(),
            created_at: BsonDateTime::from(Utc::now()),
            dates,
            restrictions,
            registered_players: Vec::new(),
            player_stats_snapshot: Vec::new(),
        }
    }

    fn check_player_stats(
        &self,
        snapshot_data: Option<&LeaderboardUser>,
        current_data: &LeaderboardUser,
    ) -> RegistrationResult {
        match snapshot_data {
            None => Err(RegistrationError::UnrankedOnAnnouncementDay),
            Some(snap) => {
                let announce_rank = Rank::from_str(&snap.league.rank);
                if announce_rank > self.restrictions.max_rank {
                    return Err(RegistrationError::AnnouncementRankTooHigh(announce_rank));
                }

                let games_played = snap.league.gamesplayed;
                if games_played < self.restrictions.min_ranked_games {
                    return Err(RegistrationError::NotEnoughGames(games_played));
                }

                let rd = snap.league.rd.unwrap_or(999f64);
                if rd > self.restrictions.max_rd {
                    return Err(RegistrationError::RdTooHigh(rd));
                }

                let current_rank = Rank::from_str(&current_data.league.rank);
                if current_rank > self.restrictions.max_rank + 1 {
                    return Err(RegistrationError::CurrentRankTooHigh(current_rank));
                }

                Ok(())
            }
        }
    }

    pub fn register(
        &self,
        database: LocalDatabase,
        tetrio_id: &str,
        discord_id: Option<u64>,
    ) -> RegistrationResult {
        let current_data = database
            .players
            .update_player(tetrio_id)
            .map_err(RegistrationError::DatabaseError)?;

        if current_data.discord_id.is_none() {
            match discord_id {
                Some(id) => {
                    database
                        .players
                        .link(id, tetrio_id)
                        .map_err(RegistrationError::DatabaseError)?;
                }
                None => return Err(RegistrationError::MissingArgument),
            }
        }

        let snapshot_data = self
            .player_stats_snapshot
            .iter()
            .find(|u| current_data.tetrio_id == u._id);

        self.check_player_stats(snapshot_data, &current_data.tetrio_data.unwrap())?;

        let reg_entry = bson::to_document(&RegistrationEntry::new(&current_data.tetrio_id))
            .expect("bad document");

        database
            .tournaments
            .collection
            .update_one(
                doc! {"shorthand": &self.shorthand},
                doc! {"$push": {"registered_players": reg_entry}},
                None,
            )
            .map_err(|_| RegistrationError::DatabaseError(DatabaseError::CouldNotPush))?;

        Ok(())
    }
}

pub struct TournamentCollection {
    collection: Collection,
}

impl TournamentCollection {
    pub fn new(database: &Database) -> TournamentCollection {
        TournamentCollection {
            collection: database.collection(COLLECTION_NAME),
        }
    }

    pub fn create_tournament(
        &self,
        name: &str,
        shorthand: &str,
        dates: TournamentDates,
        restrictions: TournamentRestrictions,
    ) -> DatabaseResult<TournamentEntry> {
        println!("Creating tournament {} ({})", name, shorthand);
        let entry = TournamentEntry::new(name, shorthand, dates, restrictions);
        match self.collection.insert_one(
            bson::to_document(&entry).expect("could not convert to document"),
            None,
        ) {
            Ok(_) => Ok(entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    pub fn get_tournament(&self, name: &str) -> DatabaseResult<Option<TournamentEntry>> {
        crate::database::get_entry(
            &self.collection,
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
        )
    }

    pub fn add_snapshot(&self, name: &str) -> DatabaseResult<()> {
        println!("Adding stat snapshot for tournament {}", name);
        if self.get_tournament(name)?.is_none() {
            return Err(DatabaseError::NotFound);
        }

        // Will ensure that unranked players are not in the snapshot and are therefore easy to identify,
        // since the players collection doesnt remove them when they become unranked
        let snapshot: Vec<Document> = match tetrio::leaderboard::request() {
            Ok(response) => response.data.users,
            Err(e) => return Err(DatabaseError::TetrioApiError(e)),
        }
        .iter()
        .map(|u| bson::to_document(u).expect("Bad document"))
        .collect();

        match self.collection.update_one(
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
            doc! {"$set": {"player_stats_snapshot": &snapshot}},
            None,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }
}
