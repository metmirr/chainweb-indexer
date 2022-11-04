use chainweb_indexer::configuration::get_configuration;
use chainweb_indexer::startup::Application;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let configuration = get_configuration().expect("Failed to read configuration");

    let application = Application::build(configuration).await?;
    application.run_indexers().await?;

    Ok(())
}
