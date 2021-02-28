#![allow(dead_code)] // temporary until everything has been implemented

use tracing_subscriber::{EnvFilter, FmtSubscriber};

use uc_helper_rust as uc;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // Set up logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    // Establish database connection
    let db = uc::database::LocalDatabase::connect().expect("Failed to connect to database");

    let mut bot = uc::discord::new_client(db).await;
    if let Err(why) = bot.start().await {
        tracing::error!("Client error: {:?}", why);
    }
}
