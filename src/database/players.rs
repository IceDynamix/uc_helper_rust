use chrono::SecondsFormat;
use mongodb::bson;
use mongodb::bson::doc;
use mongodb::options::FindOneAndReplaceOptions;
use serde::{Deserialize, Serialize};
use serenity::builder::CreateEmbed;
use tokio::stream::StreamExt;

use crate::database::{establish_db_connection, DatabaseError};
use crate::tetrio::{tenchi, Rank, User};

const COLLECTION: &str = "players";

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerEntry {
    pub _id: String,
    pub username: String,
    pub data: User,
    pub highest_rank: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub enum RegistrationError {
    CurrentRankTooHigh(Rank),
    HighestRankTooHigh(Rank),
    AnnouncementRankTooHigh(Rank),
    UnrankedOnAnnouncementDay,
}

impl RegistrationError {
    #[allow(inherent_to_string)]
    pub fn to_string(&self) -> String {
        match *self {
            RegistrationError::CurrentRankTooHigh(rank) => {
                format!("Current rank is too high ({:?})", rank)
            }
            RegistrationError::HighestRankTooHigh(rank) => {
                format!("Highest-ever rank is too high ({:?})", rank)
            }
            RegistrationError::AnnouncementRankTooHigh(rank) => {
                format!("Rank on announcement day was too high ({:?})", rank)
            }
            RegistrationError::UnrankedOnAnnouncementDay => {
                "Unranked on announcement day".to_string()
            }
        }
    }
}

impl PlayerEntry {
    pub fn generate_embed(&self) -> CreateEmbed {
        let mut e = CreateEmbed::default();
        e.title(&self.username);
        e.url(format!("https://ch.tetr.io/u/{}", &self.username));

        let league = &self.data.league;

        e.color(
            u64::from_str_radix(crate::tetrio::Rank::from_str(&league.rank).to_color(), 16)
                .unwrap_or(0),
        );

        e.thumbnail(format!(
            "https://tetrio.team2xh.net/images/ranks/{}.png",
            &self.data.league.rank
        ));

        e.fields(vec![
            (
                "Tetra Rating",
                format!("{:.0} ± {}", &league.rating, &league.rd.unwrap_or_default()),
                false,
            ),
            (
                "APM",
                format!("{:.2}", &league.apm.unwrap_or_default()),
                true,
            ),
            (
                "PPS",
                format!("{:.2}", &league.pps.unwrap_or_default()),
                true,
            ),
            ("VS", format!("{:.2}", &league.vs.unwrap_or_default()), true),
        ]);

        e.timestamp(
            chrono::DateTime::parse_from_rfc3339(&self.timestamp)
                .expect("Bad timestamp")
                .to_rfc3339_opts(SecondsFormat::Secs, false),
        );

        e
    }

    pub fn can_participate(&self) -> Result<(), RegistrationError> {
        let stats = crate::tetrio::announcement_day::from_cache();
        let announcement_stats = stats.get(&self.username);
        let rank_cap = Rank::SS; // TODO replace with settings

        if let Some(announcement_stats) = announcement_stats {
            let ann_rank = Rank::from_str(&announcement_stats.rank);
            if ann_rank == Rank::Unranked {
                return Err(RegistrationError::UnrankedOnAnnouncementDay);
            } else if ann_rank > rank_cap {
                return Err(RegistrationError::AnnouncementRankTooHigh(ann_rank));
            }
        }

        let current_rank = Rank::from_str(&self.data.league.rank);
        if current_rank > rank_cap {
            return Err(RegistrationError::CurrentRankTooHigh(current_rank));
        }

        let highest_rank = Rank::from_str(&self.highest_rank);
        if highest_rank > rank_cap {
            return Err(RegistrationError::HighestRankTooHigh(highest_rank));
        }

        Ok(())
    }
}

pub async fn get_cached(username: &str) -> Result<Option<PlayerEntry>, DatabaseError> {
    let collection = establish_db_connection().await?.collection(COLLECTION);

    let mut results = collection
        .find(
            doc! {"$or": [{"_id": username}, {"username": username}]},
            None,
        )
        .await
        .unwrap();

    match results.next().await {
        Some(result) => match result {
            Ok(r) => Ok(Some(bson::from_document(r).unwrap())),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

pub async fn get(username: &str) -> Result<PlayerEntry, DatabaseError> {
    let cached = get_cached(username).await?;

    let now = chrono::offset::Utc::now();
    if let Some(cached) = cached {
        let last_update = chrono::DateTime::parse_from_rfc3339(&cached.timestamp).unwrap();
        if now - chrono::Duration::minutes(10) < last_update {
            return Ok(cached);
        }
    }

    let data = match crate::tetrio::User::request(username).await {
        Some(data) => data,
        None => return Err(DatabaseError::NotFound),
    };

    // for our purposes its ok if it fails
    let highest_ranks = tenchi::HighestRanks::from_cache().ok();
    let highest_rank = match highest_ranks {
        Some(history) => history.get(username),
        None => Rank::Unranked,
    }
    .to_str()
    .to_string();

    let timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let _id = data.clone()._id;
    let username = data.clone().username;

    let entry = PlayerEntry {
        _id: _id.clone(),
        username,
        data,
        highest_rank,
        timestamp,
    };
    let bson_entry = bson::to_document(&entry).unwrap();

    let collection = establish_db_connection().await?.collection(COLLECTION);
    let options = FindOneAndReplaceOptions::builder().upsert(true).build();
    match collection
        .find_one_and_replace(doc! {"_id": _id.clone()}, bson_entry, options)
        .await
    {
        Ok(_) => Ok(entry),
        Err(_) => Err(DatabaseError::CouldNotPush),
    }
}
