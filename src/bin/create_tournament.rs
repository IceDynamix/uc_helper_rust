#![allow(dead_code)] // temporary until everything has been implemented

use tracing_subscriber::{EnvFilter, FmtSubscriber};

use uc::{database::tournaments::TournamentRestrictions, tetrio::Rank};
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
    let db = uc::database::connect().expect("Failed to connect to database");

    db.tournaments
        .create_tournament(
            "Underdogs Cup 11",
            "UC11",
            TournamentRestrictions::new(Rank::SPlus, 100f64, 10),
        )
        .unwrap();
}
