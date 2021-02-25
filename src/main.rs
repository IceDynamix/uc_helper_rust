#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    db.players.add_player("1".to_string(), None).await;

    Ok(())
}
