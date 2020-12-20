#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod discord;
mod settings;
mod tetrio;

pub mod database;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    // let settings = settings::Settings::from_profile("debug").unwrap();

    // Downloads a few gigabytes of data so use with care
    // tetrio::tenchi::HighestRanks::refresh().await?;
    discord::start().await?;

    Ok(())
}
