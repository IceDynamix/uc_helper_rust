use std::{collections::HashMap, fs::File, io::BufReader};

use serde::{Deserialize, Serialize};

const PATH: &str = "players.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct AnnouncementDayPlayer {
    #[serde(rename = "RD")]
    pub rd: f32,
    pub rank: String,
}

// use lazy static
pub fn from_cache() -> HashMap<String, AnnouncementDayPlayer> {
    let read_file = File::open(PATH).expect("file not there");
    let reader = BufReader::new(&read_file);
    serde_json::from_reader(reader).expect("bad json")
}
