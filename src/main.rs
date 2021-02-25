#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    // let db = database::LocalDatabase::connect().await?;
    //
    // db.players.remove_players(doc! {}).await?;
    //
    // db.players.add_player("1".to_string(), None).await?;
    // db.players
    //     .add_player("2".to_string(), Some("2".to_string()))
    //     .await?;
    // db.players.add_player("3".to_string(), None).await?;
    //
    // let entries = db.players.get_players(None).await?;
    // for e in entries {
    //     println!("{:?}", e);
    // }
    //
    // // should err with "duplicate entry"
    // db.players
    //     .add_player("3".to_string(), Some("3".to_string()))
    //     .await?;

    println!("{:?}", tetrio::request("general/stats".to_string()).await);

    Ok(())
}
