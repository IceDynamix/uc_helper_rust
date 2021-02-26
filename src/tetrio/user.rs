use serde::{Deserialize, Serialize};

use crate::tetrio::leaderboard::LeaderboardUser;
use crate::tetrio::TetrioResponse;

const ENDPOINT: &str = "users";

// LeaderboardUser is just a lighter version of the full user data so I'm using that instead!
// I don't need all of the extra information
#[derive(Deserialize, Serialize, Debug)]
pub struct UserData {
    pub user: LeaderboardUser,
}

pub fn request(tetrio_id: &str) -> TetrioResponse<UserData> {
    crate::tetrio::request::<UserData>(&format!("{}/{}", ENDPOINT, tetrio_id))
}
