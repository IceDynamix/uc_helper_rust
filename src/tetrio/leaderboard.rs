use serde::{Deserialize, Serialize};

use crate::tetrio::TetrioResponse;

const ENDPOINT: &str = "users/lists/league/all";

#[derive(Deserialize, Serialize, Debug)]
pub struct LeagueData {
    pub gamesplayed: i64,
    pub gameswon: i64,
    pub rating: f64,
    pub rank: String,
    pub glicko: Option<f64>,
    pub rd: Option<f64>,
    pub apm: Option<f64>,
    pub pps: Option<f64>,
    pub vs: Option<f64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LeaderboardUser {
    _id: String,
    username: String,
    role: String,
    country: Option<String>,
    supporter: Option<bool>,
    verified: bool,
    league: LeagueData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LeaderboardData {
    users: Vec<LeaderboardUser>,
}

pub async fn request() -> TetrioResponse<LeaderboardData> {
    crate::tetrio::request::<LeaderboardData>(ENDPOINT).await
}
