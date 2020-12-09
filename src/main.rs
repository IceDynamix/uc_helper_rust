use std::error::Error;

use settings::Settings;
use sheet::Sheet;

mod settings;
mod sheet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::from_profile("debug").unwrap();
    let sheet = Sheet::new(settings.staff_sheet_id).await.unwrap();
    println!("{:?}", sheet.read_range(&"regs!A3:D3").await);
    Ok(())
}
