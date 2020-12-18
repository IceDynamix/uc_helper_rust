#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod discord;
mod settings;
mod tetrio;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    // let settings = settings::Settings::from_profile("debug").unwrap();
    // let data = tetrio::User::request(&"icedynamix").await;
    // println!("{:?}", data);

    // let player_history = tenchi::PlayerHistory::from_cache().await?;
    // let username = "icedynamix";
    // println!("all: {:?}", player_history.get_ranks(username).await);
    // println!("highest: {:?}",player_history.get_ranks(username).await.unwrap().iter().max());

    // tetrio::database::discord::link(&"12345679abcdef", &"icedynamix).await
    // tetrio::database::discord::unlink(&"icedynamix").await

    //     tetrio::tenchi::HighestRanks::from_cache()
    //         .unwrap()
    //         .get(&"icedynamix");

    // Downloads a few gigabytes of data so use with care
    // tetrio::tenchi::HighestRanks::refresh().await?;

    discord::start().await?;

    Ok(())
}
