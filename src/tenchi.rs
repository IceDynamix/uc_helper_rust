use reqwest::Error;
use serde::Deserialize;
use std::collections::HashMap;

const URL: &str = "https://tetrio.team2xh.net/data/player_history.js";

#[derive(Deserialize, Debug)]
pub struct RankHistory {
    rank: Vec<String>,
    date: Vec<i64>,
    tr: Vec<i64>,
}

#[derive(Deserialize, Debug)]
pub struct PlayerHistory {
    timestamp_offset: i64,
    ranks: HashMap<String, RankHistory>,
    // deserializing and saving the entire data in memory would be too resource heavy
    // stats: HashMap<String, object>,
}

impl PlayerHistory {
    pub async fn request() -> Result<PlayerHistory, Error> {
        reqwest::get(URL).await?.json().await
    }
}
