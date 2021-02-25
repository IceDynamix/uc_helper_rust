#![allow(dead_code)] // temporary until everything has been implemented

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    println!("Hello World");
    Ok(())
}
