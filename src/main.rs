use std::time::Instant;

use chainweb_indexer::configuration::get_configuration;
use chainweb_indexer::startup::Application;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let configuration = get_configuration().expect("Failed to read configuration");

    let start = Instant::now();

    Application::build(configuration).await?;

    println!("{}", start.elapsed().as_secs());

    Ok(())
}
