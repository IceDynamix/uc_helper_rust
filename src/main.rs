#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

use mongodb::bson::doc;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    println!(
        "{}",
        db.players
            .is_cached(doc! {"tetrio_id": "5f6756d9484fe92b48f7007c"})
            .await
    );

    Ok(())
}
