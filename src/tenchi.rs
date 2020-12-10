use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};
use std::{error::Error, io::BufReader};

const URL: &str = "https://tetrio.team2xh.net/data/player_history.js";

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

const CACHE_PATH: &str = "./cache/rank_history.json";
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
