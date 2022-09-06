pub mod fetch_service;
pub mod types;
pub mod utils;

use std::time::Instant;
use log::debug;
pub use types::{BlockHeader, Transaction};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    env_logger::init();

    debug!("Starting");

    let _mainnet = "https://api.chainweb.com/chainweb/0.0/mainnet01";
    let _testnet = "https://api.testnet.chainweb.com/chainweb/0.0/testnet04";

    let now = Instant::now();
    let mut i = 0;
    loop {
        if i == 5 {
            break;
        }
        let r = fetch_service::blocks(_mainnet).await?;
        println!("{}", r.len());
        i += 1;
    }
    debug!("Finished in {} seconds", now.elapsed().as_secs());


    Ok(())
}