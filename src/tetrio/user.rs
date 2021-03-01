//! Tetrio API User endpoint
//!
//! This represents the endpoint as defined in the [Tetrio API](https://tetr.io/about/api/#usersuser)

use serde::{Deserialize, Serialize};

use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::TetrioResponse;

/// Endpoint url, relative to the base URL
const ENDPOINT: &str = "users";

#[derive(Deserialize, Serialize, Debug)]
/// Data structure of response data
///
/// Uses [`super::leaderboard::LeaderboardUser`], since it's a lighter version of the
/// regular user struct and the extra information is not necessary.
pub struct UserData {
    /// Requested user
    pub user: LeaderboardUser,
}

/// Requests data from the user endpoint and parses the data into the approriate struct
///
/// # Example
/// ```
/// use uc_helper_rust::tetrio;
///
/// let username = "icedynamix";
/// let user = tetrio::user::request(username);
/// match user {
///     Ok(user) => { println!("{}", user.username); },
///     Err(e) => { println("{}", e); } // Most often "not found"
/// }
/// ```
pub fn request(tetrio_id: &str) -> TetrioResponse<UserData> {
    crate::tetrio::request::<UserData>(&format!("{}/{}", ENDPOINT, tetrio_id))
}
