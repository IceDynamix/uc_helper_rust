use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Deserialize, Debug)]
pub struct Settings {
    participant_channel: u64,
    participant_role: u64,
    staff_channel: u64,
    staff_role: u64,
    public_sheet_id: String,
    staff_sheet_id: String,
    spreadsheet_registration_range: String,
    rank_cap: String,
}

impl Settings {
    pub fn from_profile(profile: &str) -> Result<Settings, Box<dyn Error>> {
        let file_content = fs::read_to_string(format!("./profiles/{}.json", profile))?;
        let settings = serde_json::from_str::<Settings>(&file_content)?;
        Ok(settings)
    }
}
