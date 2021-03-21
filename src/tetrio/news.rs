//! Tetrio API News endpoint
//!
//! Undocumented as of now, so things can be kind of janky

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tetrio::TetrioResponse;

/// Endpoint url, relative to the base URL
const ENDPOINT: &str = "news";

#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NewsPost {
    pub _id: String,
    pub stream: String,
    #[serde(rename = "type")]
    pub post_type: String,
    pub data: Value,
    pub ts: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
/// Data structure of response data
pub struct NewsData {
    /// Requested news data
    pub news: Vec<NewsPost>,
}

/// Requests data from the news endpoint and parses the data into the approriate struct
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
pub fn request(stream: &str) -> TetrioResponse<NewsData> {
    crate::tetrio::request::<NewsData>(&format!("{}/{}", ENDPOINT, stream))
}
