use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};
use std::{error::Error, io::BufReader};

use super::Rank;

const URL: &str = "https://tetrio.team2xh.net/data/player_history.js";
const CACHE_PATH: &str = "./cache/rank_history.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct RankHistory {
    pub rank: Vec<String>,
    // don't save what you don't need
    // date: Vec<i64>,
    // tr: Vec<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PlayerHistory {
    timestamp_offset: i64,
    pub ranks: HashMap<String, RankHistory>,
    // deserializing and saving the entire data in memory would be too resource heavy
    // stats: HashMap<String, object>,
}

impl PlayerHistory {
    pub async fn request() -> Result<PlayerHistory, reqwest::Error> {
        reqwest::get(URL).await?.json().await
    }

    pub async fn cache(&self) -> Result<(), Box<dyn Error>> {
        serde_json::to_writer(File::create(CACHE_PATH)?, self)?;
        Ok(())
    }

    pub async fn from_cache() -> Result<PlayerHistory, Box<dyn Error>> {
        let file = File::open(CACHE_PATH)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub async fn refresh() -> Result<PlayerHistory, Box<dyn Error>> {
        PlayerHistory::request().await?.cache().await?;
        PlayerHistory::from_cache().await
    }

    pub async fn get_ranks(&self, username: &str) -> Option<Vec<Rank>> {
        if let Some(rank_history) = self.ranks.get(&username.to_lowercase()) {
            Some(
                rank_history
                    .rank
                    .iter()
                    .map(|rank| Rank::from_str(rank))
                    .collect(),
            )
        } else {
            None
        }
    }
}
