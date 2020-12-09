use std::error::Error;

use settings::Settings;

mod settings;

fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::from_profile("debug");
    println!("{:?}", settings);
    Ok(())
}
