#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    // db.players.remove_all().await?;
    // db.players.update_from_leaderboard().await?;

    // db.players.update_player("icedynamix").await?;

    // db.players.unlink_by_discord(126806732889522176).await?;
    db.players.link(126806732889522176, "icedynamix").await?;

    Ok(())
}
