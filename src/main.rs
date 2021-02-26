#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    println!("{:?}", db.players.get_player_by_tetrio("icedynamix").await?);

    Ok(())
}
