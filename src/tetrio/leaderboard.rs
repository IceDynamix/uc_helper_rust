//! Tetrio API Leaderboard endpoint
//!
//! This represents the endpoint as defined in the [Tetrio API](https://tetr.io/about/api/#userlistsleagueall)

use serde::{Deserialize, Serialize};

use crate::tetrio::TetrioResponse;

/// Endpoint url, relative to the base URL
const ENDPOINT: &str = "users/lists/league/all";

#[derive(Deserialize, Serialize, Debug, Clone)]
/// League specific data
///
/// Defined in detail in the Tetrio API [Tetrio API](https://tetr.io/about/api/#userlistsleagueall)
///
/// The `Option<f64>` fields are `None` when the user is unranked and has never played a ranked game
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

#[derive(Deserialize, Serialize, Debug, Clone)]
/// User data as contained in [`LeaderboardData`]
///
/// Defined in detail in the Tetrio API [Tetrio API](https://tetr.io/about/api/#userlistsleagueall)
///
/// Country and supporter are Options, as users who registered before country or supporter status was tracked don't contain the value.
/// This is a lighter version of the full user object, returned by [`/users/:user`](https://tetr.io/about/api/#usersuser).
pub struct LeaderboardUser {
    pub _id: String,
    pub username: String,
    pub role: String,
    pub country: Option<String>,
    pub supporter: Option<bool>,
    pub verified: bool,
    pub league: LeagueData,
}

#[derive(Deserialize, Serialize, Debug)]
/// Data structure of response data
pub struct LeaderboardData {
    pub users: Vec<LeaderboardUser>,
}

/// Requests data from the leaderboard endpoint and parses the data into the approriate struct
///
/// # Example
/// ```
/// use uc_helper_rust::tetrio;
///
/// let leaderboard = tetrio::leaderboard::request()?;
/// for user in leaderboard.users {
///     println!("{}", user.username);
/// }
/// ```
pub fn request() -> TetrioResponse<LeaderboardData> {
    crate::tetrio::request::<LeaderboardData>(ENDPOINT)
}
