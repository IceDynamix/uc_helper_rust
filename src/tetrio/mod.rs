pub mod database;
pub mod tenchi;

use reqwest::Client;
use serde::{Deserialize, Serialize};

const URL: &str = "https://ch.tetr.io/api";

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq)]
pub enum Rank {
    Unranked,
    D,
    DPlus,
    CMinus,
    C,
    CPlus,
    BMinus,
    B,
    BPlus,
    AMinus,
    A,
    APlus,
    SMinus,
    S,
    SPlus,
    SS,
    U,
    X,
}

impl Rank {
    pub fn from_str(s: &str) -> Rank {
        match s {
            "d" => Rank::D,
            "d+" => Rank::DPlus,
            "c-" => Rank::CMinus,
            "c" => Rank::C,
            "c+" => Rank::CPlus,
            "b-" => Rank::BMinus,
            "b" => Rank::B,
            "b+" => Rank::BPlus,
            "a-" => Rank::AMinus,
            "a" => Rank::A,
            "a+" => Rank::APlus,
            "s-" => Rank::SMinus,
            "s" => Rank::S,
            "s+" => Rank::SPlus,
            "ss" => Rank::SS,
            "u" => Rank::U,
            "x" => Rank::X,
            _ => Rank::Unranked,
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            Rank::D => "d",
            Rank::DPlus => "d+",
            Rank::CMinus => "c-",
            Rank::C => "c",
            Rank::CPlus => "c+",
            Rank::BMinus => "b-",
            Rank::B => "b",
            Rank::BPlus => "b+",
            Rank::AMinus => "a-",
            Rank::A => "a",
            Rank::APlus => "a+",
            Rank::SMinus => "s-",
            Rank::S => "s",
            Rank::SPlus => "s+",
            Rank::SS => "ss",
            Rank::U => "u",
            Rank::X => "x",
            Rank::Unranked => "z",
        }
    }

    pub fn to_color(&self) -> &str {
        match self {
            Rank::Unranked => "828282",
            Rank::D => "856C84",
            Rank::DPlus => "815880",
            Rank::CMinus => "6C417C",
            Rank::C => "67287B",
            Rank::CPlus => "522278",
            Rank::BMinus => "5949BE",
            Rank::B => "4357B5",
            Rank::BPlus => "4880B2",
            Rank::AMinus => "35AA8C",
            Rank::A => "3EA750",
            Rank::APlus => "43b536",
            Rank::SMinus => "B79E2B",
            Rank::S => "d19e26",
            Rank::SPlus => "dbaf37",
            Rank::SS => "e39d3b",
            Rank::U => "c75c2e",
            Rank::X => "b852bf",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeagueData {
    gamesplayed: i64,
    gameswon: i64,
    rating: f64,
    rank: String,
    standing: i64,
    standing_local: i64,
    percentile: f64,
    percentile_rank: String,
    glicko: Option<i64>,
    rd: Option<i64>,
    apm: Option<f64>,
    pps: Option<f64>,
    vs: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Badge {
    id: String,
    label: String,
    ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    _id: String,
    username: String,
    role: String,
    ts: Option<String>,
    botmaster: Option<String>,
    badges: Vec<Badge>,
    xp: f64,
    gamesplayed: i64,
    gameswon: i64,
    gametime: f64,
    country: Option<String>,
    badstanding: Option<bool>, // Added late so not every user has it
    supporter: bool,
    verified: bool,
    league: LeagueData,
    avatar_revision: Option<i64>,
    banner_revision: Option<i64>,
    bio: Option<String>,
}

impl User {
    pub async fn request(username: &str) -> Option<User> {
        let client = Client::new();
        let url = format!("{}/users/{}", URL, username);
        let request = client
            .request(reqwest::Method::GET, &url)
            .header("X-Session-Header", "IceDynamix");

        let response: PlayerResponse = client
            .execute(request.build().ok()?)
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        if response.success {
            return Some(response.data.unwrap().user);
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Player {
    user: User,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerResponse {
    success: bool,
    error: Option<String>,
    data: Option<Player>,
}
