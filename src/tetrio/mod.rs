use std::fmt::Formatter;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod leaderboard;
pub mod user;

const API_URL: &str = "https://ch.tetr.io/api";

#[derive(Debug)]
pub enum TetrioApiError {
    Error(String),
}

impl std::error::Error for TetrioApiError {
    fn description(&self) -> &str {
        match self {
            TetrioApiError::Error(_) => "Something went wrong",
        }
    }
}

impl std::fmt::Display for TetrioApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TetrioApiError::Error(e) => f.write_str(e),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct TetrioResponseStruct {
    success: bool,
    cache: Option<CacheData>,
    data: Option<Value>,
    error: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CacheData {
    status: String,
    cached_at: i64,
    cached_until: i64,
}

#[derive(Debug)]
pub struct SuccessfulResponse<T> {
    pub data: T,
    pub cache: CacheData,
}

type TetrioResponse<T> = Result<SuccessfulResponse<T>, TetrioApiError>;

pub async fn request<T: DeserializeOwned>(endpoint: &str) -> TetrioResponse<T> {
    let client = Client::new();
    let url = format!("{}/{}", API_URL, endpoint);
    let request = client
        .request(reqwest::Method::GET, &url)
        .header("X-Session-Header", "IceDynamix") // i have no idea whether im doing this right
        .build()
        .expect("Could not build request");

    let response = client
        .execute(request)
        .await
        .expect("Could not execute request");

    let parsed_response: TetrioResponseStruct = response.json().await.expect("Could not parse");

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
