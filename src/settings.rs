use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub participant_channel: u64,
    pub participant_role: u64,
    pub staff_channel: u64,
    pub staff_role: u64,
    pub rank_cap: String,
}

impl Settings {
    pub fn from_profile(profile: &str) -> Result<Settings, Box<dyn Error>> {
        let file_content = fs::read_to_string(format!("./profiles/{}.json", profile))?;
        let settings = serde_json::from_str::<Settings>(&file_content)?;
        Ok(settings)
    }
}
