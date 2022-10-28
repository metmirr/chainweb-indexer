use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::configuration::{DatabaseSettings, Settings};
use crate::ingest::{Ingest, QueryParams};

pub struct Application;

impl Application {
    pub async fn build(configuration: Settings) -> Result<(), anyhow::Error> {
        let number_of_worker =
            if configuration.application.max_height > configuration.application.chain_fork_height {
                configuration.application.number_of_chains
            } else {
                10
            };
        let db_pool = get_connection_pool(&configuration.database);

        let mut workers = vec![];

        for i in 0..number_of_worker {
            let c = configuration.clone();
            let query_params = QueryParams::new(c.application.limit, c.application.min_height);
            let pool = db_pool.clone();

            let worker = tokio::spawn(async move {
                let url = c.application.host.clone();
                let mut ingest = Ingest::new(i, url, query_params.clone(), pool);
                _ = ingest.start().await;
            });
            workers.push(worker);
        }

        for worker in workers {
            worker.await.unwrap();
        }
        Ok(())
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
