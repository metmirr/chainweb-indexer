use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::configuration::{DatabaseSettings, Settings};
use crate::entities::Block;
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
        let processed_blocks = get_processed_blocks_logs(&db_pool).await?;

        let mut workers = vec![];

        for chain_id in 0..number_of_worker {
            let c = configuration.clone();
            let min_height = match processed_blocks.iter().find(|b| b.chain_id == chain_id) {
                Some(v) => {
                    let next_height = v.height + 1;
                    next_height as u64
                }
                None => c.application.min_height,
            };
            let query_params = QueryParams::new(c.application.limit, min_height);
            let pool = db_pool.clone();

            let worker = tokio::spawn(async move {
                let url = c.application.host.clone();
                let mut ingest = Ingest::new(chain_id, url, query_params.clone(), pool);
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

pub async fn get_processed_blocks_logs(pool: &PgPool) -> Result<Vec<Block>, sqlx::Error> {
    let blocks = sqlx::query_as!(
        Block,
        r#"
        SELECT * FROM processed_blocks_logs
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(blocks)
}

pub fn get_min_height_for_chain(chain_id: i16, blocks: &[Block]) -> u64 {
    match blocks.iter().find(|b| b.chain_id == chain_id) {
        Some(v) => v.height as u64,
        None => 0,
    }
}
