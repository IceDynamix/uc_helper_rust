use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_URL: &str = "https://ch.tetr.io/api";

#[derive(Deserialize, Serialize, Debug)]
pub struct TetrioResponse {
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

pub async fn request(endpoint: String) -> Option<TetrioResponse> {
    let client = Client::new();
    let url = format!("{}/{}", API_URL, endpoint);
    let request = client
        .request(reqwest::Method::GET, &url)
        .header("X-Session-Header", "IceDynamix") // i have no idea whether im doing this right
        .build()
        .ok()?;

    let response = client.execute(request).await.ok()?;
    response.json().await.ok()?
}
