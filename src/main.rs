#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod discord;
mod settings;
mod tetrio;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let settings = Settings::from_profile("debug").unwrap();
    // let data = tetrio::User::request(vec!["icedynamix".to_string(), "electroyan".to_string()]).await?;

    // Downloads a few gigabytes of data so use with care
    // let rank_histories = tenchi::PlayerHistory::refresh().await?;
    // let player_history = tenchi::PlayerHistory::from_cache().await?;
    // let username = "icedynamix";
    // println!("all: {:?}", player_history.get_ranks(username).await);
    // println!("highest: {:?}",player_history.get_ranks(username).await.unwrap().iter().max());
    // discord::bot::start().await?;

    Ok(())
}
