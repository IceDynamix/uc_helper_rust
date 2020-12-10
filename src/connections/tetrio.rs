use std::collections::HashMap;

use reqwest::{Client, Error};
use serde::Deserialize;

const URL: &str = "https://ch.tetr.io/api";

#[derive(Deserialize, Debug)]
pub struct LeagueData {
    // The amount of TETRA LEAGUE games played by this user.
    gamesplayed: i64,
    // The amount of TETRA LEAGUE games won by this user.
    gameswon: i64,
    // This user's TR (Tetra Rating), or -1 if less than 10 games were played.
    rating: f64,
    // This user's letter rank. Z is unranked.
    rank: String,
    // This user's position in global leaderboards, or -1 if not applicable.
    standing: i64,
    // This user's position in local leaderboards, or -1 if not applicable.
    standing_local: i64,
    // This user's percentile position (0 is best, 1 is worst).
    percentile: f64,
    // This user's percentile rank, or Z if not applicable.
    percentile_rank: String,
    // This user's Glicko-2 rating.
    glicko: Option<i64>,
    // This user's Glicko-2 Rating Deviation. If over 100, this user is unranked.
    rd: Option<i64>,
    // This user's average APM (attack per minute) over the last 10 games.
    apm: Option<f64>,
    // This user's average PPS (pieces per second) over the last 10 games.
    pps: Option<f64>,
    // This user's average VS (versus score) over the last 10 games.
    vs: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct Badge {
    // The badge's i64ernal ID, and the filename of the badge icon (all PNGs within /res/badges/)
    id: String,
    // The badge's label, shown when hovered.
    label: String,
    // The badge's timestamp, if shown.
    ts: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    // The user's internal ID.
    #[serde(rename = "_id")]
    id: String,
    // The user's username.
    username: String,
    // The user's role (one of "anon", "user", "bot", "mod", "admin").
    role: String,
    // When the user account was created. If not set, this account was created before join dates were recorded.
    ts: Option<String>,
    // If this user is a bot, the bot's operator.
    botmaster: Option<String>,
    // The user's badges:
    badges: Vec<Badge>,
    // The user's XP in poi64s.
    xp: f64,
    // The amount of online games played by this user. If the user has chosen to hide this statistic, it will be -1.
    gamesplayed: i64,
    // The amount of online games won by this user. If the user has chosen to hide this statistic, it will be -1.
    gameswon: i64,
    // The amount of seconds this user spent playing, both on- and offline. If the user has chosen to hide this statistic, it will be -1.
    gametime: f64,
    // The user's ISO 3166-1 country code, or null if hidden/unknown. Some vanity flags exist.
    country: Option<String>,
    // Whether this user currently has a bad standing (recently banned).
    // Added late so not every user has it
    badstanding: Option<bool>,
    // Whether this user is currently supporting TETR.IO <3
    supporter: bool,
    // Whether this user is a verified account.
    verified: bool,
    // This user's current TETRA LEAGUE standing:
    league: LeagueData,
    // This user's avatar ID. Get their avatar at https://tetr.io/user-content/avatars/{ USERID }.jpg?rv={ AVATAR_REVISION }
    avatar_revision: Option<i64>,
    // This user's banner ID. Get their banner at https://tetr.io/user-content/banners/{ USERID }.jpg?rv={ BANNER_REVISION }. Ignore this field if the user is not a supporter.
    banner_revision: Option<i64>,
    // This user's "About Me" section. Ignore this field if the user is not a supporter.
    bio: Option<String>,
}

impl User {
    pub async fn request(usernames: Vec<String>) -> Result<HashMap<String, User>, Error> {
        let mut user_data = HashMap::new();
        let client = Client::new();
        for username in usernames {
            let url = format!("{}/users/{}", URL, username);
            let request = client
                .request(reqwest::Method::GET, &url)
                .header("X-Session-Header", "IceDynamix");
            let response: PlayerResponse = client.execute(request.build()?).await?.json().await?;
            if response.success {
                let user = response.data.unwrap().user;
                user_data.insert(user.id.clone(), user);
            }
        }
        Ok(user_data)
    }
}

#[derive(Deserialize, Debug)]
struct Player {
    user: User,
}

#[derive(Deserialize, Debug)]
pub struct PlayerResponse {
    // Whether the request was successful.
    success: bool,
    // If unsuccessful, the reason the request failed.
    error: Option<String>,
    // If successful, data about how this request was cached.
    // cache: Option<object>,
    // If successful, the requested data:
    data: Option<Player>,
}
