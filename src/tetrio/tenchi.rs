use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};
use std::{error::Error, io::BufReader};

use super::Rank;
use chrono::TimeZone;

const URL: &str = "https://tetrio.team2xh.net/data/player_history.js";
const CACHE_PATH: &str = "./cache/rank_history.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct HighestRanks {
    pub timestamp: String,
    pub ranks: HashMap<String, String>,
}

impl HighestRanks {
    pub async fn request() -> Result<HighestRanks, reqwest::Error> {
        let player_history: PlayerHistory = reqwest::get(URL).await?.json().await?;
        let mut ranks = HashMap::new();
        player_history
            .ranks
            .iter()
            .for_each(|(username, user_ranks)| {
                let highest_rank = user_ranks
                    .rank
                    .iter()
                    .map(|rank| Rank::from_str(rank))
                    .max()
                    .unwrap_or(Rank::Unranked);
                ranks.insert(username.to_owned(), highest_rank.to_str().to_string());
            });

        let timestamp = chrono::Utc
            .timestamp(player_history.timestamp_offset, 0)
            .to_rfc3339();

        Ok(HighestRanks { timestamp, ranks })
    }

    pub fn cache(&self) -> Result<(), Box<dyn Error>> {
        serde_json::to_writer(File::create(CACHE_PATH)?, self)?;
        Ok(())
    }

    pub async fn refresh() -> Result<HighestRanks, Box<dyn Error>> {
        HighestRanks::request().await?.cache()?;
        Ok(HighestRanks::from_cache()?)
    }

    pub fn from_cache() -> Result<HighestRanks, Box<dyn Error>> {
        let file = File::open(CACHE_PATH)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn get(&self, username: &str) -> Rank {
        match self.ranks.get(username) {
            Some(rank) => Rank::from_str(rank),
            None => Rank::Unranked,
        }
    }
}

#[derive(Deserialize, Debug)]
struct RankHistory {
    pub rank: Vec<String>,
    // don't save what you don't need
    // date: Vec<i64>,
    // tr: Vec<i64>,
}

#[derive(Deserialize, Debug)]
struct PlayerHistory {
    timestamp_offset: i64,
    pub ranks: HashMap<String, RankHistory>,
    // deserializing and saving the entire data in memory would be too resource heavy
    // stats: HashMap<String, object>,
}
