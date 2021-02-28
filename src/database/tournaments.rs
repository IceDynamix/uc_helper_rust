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
pub enum RegistrationError {
    #[error("Current rank is too high (currently `{rank}`, ≤ `{expected}` required)")]
    CurrentRankTooHigh { rank: Rank, expected: Rank },
    #[error(
        "Rank was too high on announcement day (was `{rank}` by `{date}`, ≤ `{expected}` required)"
    )]
    AnnouncementRankTooHigh {
        rank: Rank,
        expected: Rank,
        date: DateTime<Utc>,
    },
    #[error("Not enough ranked games played until announcement day (was `{value}` by `{date}`, ≥ `{expected}` required)")]
    NotEnoughGames {
        value: i64,
        expected: i64,
        date: DateTime<Utc>,
    },
    #[error(
        "RD was too high at announcement day (was `{value}` by `{date}`, ≥ `{expected}` required)"
    )]
    RdTooHigh {
        value: f64,
        expected: f64,
        date: DateTime<Utc>,
    },
    #[error("Player was unranked on announcement day (`{0}`)")]
    UnrankedOnAnnouncementDay(DateTime<Utc>),
    #[error("There is no tournament ongoing")]
    NoTournamentActive,
    #[error("Something was missing while registering (`{0}`)")]
    MissingArgument(String),
    #[error("Something went wrong while accessing the database: {0}")]
    DatabaseError(#[from] DatabaseError),
    #[error("User is already registered")]
    AlreadyRegistered,
    #[error("User is not registered")]
    NotRegistered,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TournamentDates {
    pub announcement_at: BsonDateTime,
    pub registration_end: BsonDateTime,
    pub check_in_start: BsonDateTime,
    pub check_in_end: BsonDateTime,
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
    pub name: String,
    pub shorthand: String,
    created_at: BsonDateTime,
    pub dates: TournamentDates,
    pub restrictions: TournamentRestrictions,
    pub registered_players: Vec<RegistrationEntry>,
    player_stats_snapshot: Vec<LeaderboardUser>,
    active: bool,
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
            active: false,
        }
    }

    fn check_player_stats(&self, current_data: &LeaderboardUser) -> RegistrationResult {
        let snapshot_data = self
            .player_stats_snapshot
            .iter()
            .find(|u| current_data._id == u._id);

        let announcement: DateTime<Utc> = *self.dates.announcement_at;

        match snapshot_data {
            None => Err(RegistrationError::UnrankedOnAnnouncementDay(announcement)),
            Some(snap) => {
                let announce_rank = Rank::from_str(&snap.league.rank).unwrap();
                if announce_rank > self.restrictions.max_rank {
                    return Err(RegistrationError::AnnouncementRankTooHigh {
                        rank: announce_rank,
                        expected: self.restrictions.max_rank,
                        date: announcement,
                    });
                }

                let games_played = snap.league.gamesplayed;
                if games_played < self.restrictions.min_ranked_games {
                    return Err(RegistrationError::NotEnoughGames {
                        value: games_played,
                        expected: self.restrictions.min_ranked_games,
                        date: announcement,
                    });
                }

                let rd = snap.league.rd.unwrap_or(999f64);
                if rd > self.restrictions.max_rd {
                    return Err(RegistrationError::RdTooHigh {
                        value: rd,
                        expected: self.restrictions.max_rd,
                        date: announcement,
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
        tracing::info!("Creating tournament {} ({})", name, shorthand);
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

        // use the linked player if no username is provided
        // link already takes care of the cases where tetrio id or discord id do not match
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

    pub fn add_snapshot(&self, name: &str) -> DatabaseResult<()> {
        tracing::info!("Adding stat snapshot for tournament {}", name);
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

    pub fn get_active(&self) -> DatabaseResult<Option<TournamentEntry>> {
        crate::database::get_entry(&self.collection, doc! {"active": true})
    }
}
