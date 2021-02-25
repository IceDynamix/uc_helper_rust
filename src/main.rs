#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

use mongodb::bson::doc;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    let icedynamix = db.players.get_player("icedynamix").await?.unwrap();

    db.players.update_player("icedynamix").await?;
    println!("{}", icedynamix.is_cached());

    Ok(())
}
