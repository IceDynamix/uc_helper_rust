//! Data classes and functions regarding the Tetrio API.
//!
//! All functions are written synchronously, for tokio runtime reasons related to usage in the Discord bot client.
//!
//! Only Leaderboard and User endpoints are implemented for now. There is no caching going on, all of the caching is managed by the database module.
//! Therefore in the optimal use-case, the [crate::discord] module should never call from this module directly, only the [crate::database] commands.
//!
//! # Example
//!
//! ```
//! use uc_helper_rust::tetrio;
//!
//! let leaderboard = tetrio::leaderboard::request()?;
//! let user = tetrio::user::request("icedynamix")?;
//! ```

#![warn(missing_docs)]

use std::fmt::Formatter;

use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod leaderboard;
pub mod news;
pub mod user;

/// The base URL of the Tetrio API
const API_URL: &str = "https://ch.tetr.io/api";

#[derive(Error, Debug)]
/// Something that can go wrong while requesting from the Tetrio API
pub enum TetrioApiError {
    #[error("Something happened while requesting from tetrio: {0}")]
    /// Error returned by the Tetrio API on an unsuccessful request
    Error(String),
}

#[derive(Deserialize, Serialize, Debug)]
/// Structure of every Tetrio API response
///
/// - `success`: Whether the request was successful.
/// - `cache`: If successful, data about how this request was cached. (on the API's side)
/// - `data`: If successful, the requested data
/// - `error`: If unsuccessful, the reason the request failed.
pub struct TetrioResponseStruct {
    success: bool,
    cache: Option<CacheData>,
    data: Option<Value>,
    error: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
/// Information about how data was cached server-side
///
/// Cache is not shared between workers. Load balancing may therefore give you unexpected responses.
/// To use the same worker, pass the same X-Session-ID header for all requests that should use the same cache.
pub struct CacheData {
    /// Whether the cache was hit. Either "hit", "miss", or "awaited" (resource was already being requested by another client)
    pub status: String,
    /// When this resource was cached. (format: UNIX timestamp * 1000)
    pub cached_at: i64,
    /// When this resource's cache expires. (format: UNIX timestamp * 1000)
    pub cached_until: i64,
}

#[derive(Debug)]
/// Successful API response, without the `success` or `error` field.
pub struct SuccessfulResponse<T> {
    /// Requested data
    pub data: T,
    /// How the data is cached server-side
    pub cache: CacheData,
}

/// Tetrio API response, represented as a Result type
type TetrioResponse<T> = Result<SuccessfulResponse<T>, TetrioApiError>;

/// General function to request from a Tetrio endpoint.
///
/// While this function is public, you should instead call the request functions directly from the data classes defined for each endpoint.
///
/// - [`leaderboard::request()`]
/// - [`user::request()`]
pub fn request<T: DeserializeOwned>(endpoint: &str) -> TetrioResponse<T> {
    tracing::info!("Requesting from endpoint {}", endpoint);

    let parsed_response: TetrioResponseStruct = tokio::task::block_in_place(|| {
        let client = Client::new();
        let url = format!("{}/{}", API_URL, endpoint);
        let request = client
            .request(reqwest::Method::GET, &url)
            .header("X-Session-Header", "IceDynamix") // i have no idea whether im doing this right
            .build()
            .expect("Could not build request");

        let response = client.execute(request).expect("Could not execute request");

        response.json().expect("Could not parse")
    });

    if !parsed_response.success {
        return Err(TetrioApiError::Error(parsed_response.error.unwrap()));
    }

    if parsed_response.data.is_none() {
        return Err(TetrioApiError::Error("No data".to_string()));
    }

    if parsed_response.cache.is_none() {
        return Err(TetrioApiError::Error("No cache data".to_string()));
    }

    match serde_json::from_value::<T>(parsed_response.data.unwrap()) {
        Ok(parsed_data) => Ok(SuccessfulResponse {
            data: parsed_data,
            cache: parsed_response.cache.unwrap(),
        }),
        Err(_) => Err(TetrioApiError::Error("Could not parse".to_string())),
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq)]
/// A player's league rank
///
/// Supports addition of usizes and comparison of ranks
///
/// # Example
///
/// ```
/// use uc_helper_rust::tetrio::Rank;
/// use std::str::FromStr;
///
/// let rank = Rank::from_str("s+");
///
/// assert_eq!(Rank::SPlus, rank);
/// assert_eq!(Rank::SS, rank + 1);
/// assert!(Rank::U > rank);
/// ```
#[allow(missing_docs)]
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
    #[allow(clippy::upper_case_acronyms)]
    SS,
    U,
    X,
}

impl Rank {
    /// Returns a string representation, to be used in URLs or similar
    ///
    /// You'll want to use the display trait implementation instead most of the time.
    pub fn to_str(&self) -> &'static str {
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

    //noinspection ALL
    /// Returns a hex code color (without the #) for the specified rank
    pub fn to_color(&self) -> &'static str {
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

    /// Returns the Underdogs Cup Discord Emoji representation
    ///
    /// These will, by definition, only work on the Underdogs Cup Discord.
    pub fn to_emoji(&self) -> &'static str {
        match self {
            Rank::X => "<:rank_x:758747882215047169>",
            Rank::U => "<:rank_u:758747882127097887>",
            Rank::SS => "<:rank_ss:758747882425024572>",
            Rank::SPlus => "<:rank_splus:758747881951461417>",
            Rank::S => "<:rank_s:758747881728507986>",
            Rank::SMinus => "<:rank_sminus:758747881820651561>",
            Rank::APlus => "<:rank_aplus:758747881820913684>",
            Rank::A => "<:rank_a:758747881682763797>",
            Rank::AMinus => "<:rank_aminus:758747881657204775>",
            Rank::BPlus => "<:rank_bplus:758747881854337034>",
            Rank::B => "<:rank_b:758747881779232778>",
            Rank::BMinus => "<:rank_bminus:758747881833365505>",
            Rank::CPlus => "<:rank_cplus:758747881833889802>",
            Rank::C => "<:rank_c:758747881808068622>",
            Rank::CMinus => "<:rank_cminus:758747881791422464>",
            Rank::DPlus => "<:rank_dplus:758747881603072061>",
            Rank::D => "<:rank_d:758747881896149052>",
            _ => "<:rank_unranked:790331415836622868>",
        }
    }

    /// Returns an image representation of the rank as an image URL
    pub fn to_img_url(&self) -> String {
        format!("https://tetr.io/res/league-ranks/{}.png", self.to_str())
    }

    /// Returns an iterator, which follows the rank order
    pub fn iter() -> std::slice::Iter<'static, Rank> {
        use Rank::*;
        static RANKS: [Rank; 18] = [
            Unranked, D, DPlus, CMinus, C, CPlus, BMinus, B, BPlus, AMinus, A, APlus, SMinus, S,
            SPlus, SS, U, X,
        ];
        RANKS.iter()
    }
}

impl std::str::FromStr for Rank {
    type Err = ();

    /// Parses a string into a Rank. Will return `Rank::Unranked` if not parsable
    fn from_str(s: &str) -> Result<Self, ()> {
        let parsed = match s {
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
        };

        Ok(parsed)
    }
}

impl std::fmt::Display for Rank {
    /// Returns a pretty string representation
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str().to_uppercase().as_str())
    }
}

impl std::ops::Add<usize> for Rank {
    type Output = Rank;

    fn add(self, n: usize) -> Self::Output {
        let index = Rank::iter().position(|r| self == *r).unwrap_or(0);
        *Rank::iter().nth(index + n).unwrap_or(&Rank::X)
    }
}
