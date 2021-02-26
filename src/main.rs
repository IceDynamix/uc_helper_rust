#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

mod database;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let db = database::LocalDatabase::connect().await?;
    // println!("{:?}", db.players.get_player_by_tetrio("icedynamix").await?);

    // let now = Utc::now();
    // let dates = TournamentDates::new(now, now, now, now);
    // let restrictions = TournamentRestrictions::new(75, 80f64, Rank::S);
    // db.tournaments
    //     .create_tournament("Test Tournament 1", "TT1", dates, restrictions)
    //     .await?;
    //
    // db.tournaments.add_snapshot("TT1").await?;

    let tournament = db.tournaments.get_tournament("TT1").await?.unwrap();
    println!(
        "{:?}",
        tournament
            .register(db, "milkysune", Some(746868300163579915))
            .await
    );

    Ok(())
}
