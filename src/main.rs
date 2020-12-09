use std::error::Error;

use settings::Settings;
use sheet::Sheet;

mod settings;
mod sheet;
mod tenchi;
mod tetrio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let settings = Settings::from_profile("debug").unwrap();
    // let sheet = Sheet::new(settings.staff_sheet_id).await?;
    // let data = tetrio::User::request(vec!["icedynamix".to_string(), "electroyan".to_string()]).await?;
    let rank_histories = tenchi::PlayerHistory::request().await?;
    println!("{:?}", rank_histories);
    Ok(())
}
