use std::error::Error;

use settings::Settings;
use sheet::Sheet;

mod settings;
mod sheet;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let settings = Settings::from_profile("debug").unwrap();
    // let sheet = Sheet::new(settings.staff_sheet_id).await?;
    // let data = tenchi::players::Players::new().await?;
    // println!("{:?}", data.unranked_stats.get("icedynamix").unwrap());
    println!(
        "{:?}",
        tetrio::User::request(vec!["icedynamix".to_string(), "electroyan".to_string()]).await?
    );
    Ok(())
}
