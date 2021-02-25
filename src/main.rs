#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::establish_connection().await?;
    for coll_name in db.list_collection_names(None).await? {
        println!("{}", coll_name);
    }

    Ok(())
}
