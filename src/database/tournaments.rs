use bson::{doc, DateTime as BsonDateTime, Document};
use chrono::{DateTime, Utc};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::database::{DatabaseError, DatabaseResult};
use crate::tetrio;
use crate::tetrio::{leaderboard::LeaderboardUser, Rank};

const COLLECTION_NAME: &str = "tournaments";

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
    min_ranked_games: u32,
    max_rd: f32,
    max_rank: Rank,
}

impl TournamentRestrictions {
    pub fn new(min_ranked_games: u32, max_rd: f32, max_rank: Rank) -> TournamentRestrictions {
        TournamentRestrictions {
            min_ranked_games,
            max_rd,
            max_rank,
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
    registered_players: Vec<String>,
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

    pub async fn create_tournament(
        &self,
        name: &str,
        shorthand: &str,
        dates: TournamentDates,
        restrictions: TournamentRestrictions,
    ) -> DatabaseResult<TournamentEntry> {
        println!("Creating tournament {} ({})", name, shorthand);
        let entry = TournamentEntry::new(name, shorthand, dates, restrictions);
        match self
            .collection
            .insert_one(
                bson::to_document(&entry).expect("could not convert to document"),
                None,
            )
            .await
        {
            Ok(_) => Ok(entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    pub async fn get_tournament(&self, name: &str) -> DatabaseResult<Option<TournamentEntry>> {
        crate::database::get_entry(
            &self.collection,
            doc! {"$or":[{"name": name}, {"shorthand": name}]},
        )
        .await
    }

    pub async fn add_snapshot(&self, name: &str) -> DatabaseResult<()> {
        println!("Adding stat snapshot for tournament {}", name);
        if self.get_tournament(name).await?.is_none() {
            return Err(DatabaseError::NotFound);
        }

        // Will ensure that unranked players are not in the snapshot and are therefore easy to identify,
        // since the players collection doesnt remove them when they become unranked
        let snapshot: Vec<Document> = match tetrio::leaderboard::request().await {
            Ok(response) => response.data.users,
            Err(e) => return Err(DatabaseError::TetrioApiError(e.to_string())),
        }
        .iter()
        .map(|u| bson::to_document(u).expect("Bad document"))
        .collect();

        match self
            .collection
            .update_one(
                doc! {"$or":[{"name": name}, {"shorthand": name}]},
                doc! {"$set": {"player_stats_snapshot": &snapshot}},
                None,
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }
}
