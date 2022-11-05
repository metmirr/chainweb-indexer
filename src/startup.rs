use std::collections::HashMap;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::configuration::{ApplicationSettings, DatabaseSettings, Settings};
use crate::entities::Block;
use crate::ingest::{Ingest, QueryParams};

pub struct Application {
    indexers: Vec<Ingest>,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Application, anyhow::Error> {
        let db_pool = get_connection_pool(&configuration.database);
        let processed_blocks = get_processed_blocks_logs(&db_pool).await?;

        let mut indexers = vec![];
        let chains_blocks_map =
            get_min_height_for_chains(&processed_blocks, &configuration.application);

        for chain_id in 0..configuration.application.number_of_chains {
            let c = configuration.clone();
            let mm = chains_blocks_map.get(&chain_id).unwrap();
            let query_params = QueryParams::new(c.application.limit, *mm);
            let pool = db_pool.clone();

            let url = c.application.host.clone();
            indexers.push(Ingest::new(chain_id, url, query_params.clone(), pool));
        }

        Ok(Self { indexers })
    }

    pub async fn run_indexers(self) -> Result<(), anyhow::Error> {
        let mut workers = vec![];
        for mut indexer in self.indexers {
            workers.push(tokio::spawn(async move {
                indexer.start().await.unwrap();
            }));
        }
        for worker in workers {
            worker.await?;
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

fn get_min_height_for_chains(
    processed_blocks: &[Block],
    settings: &ApplicationSettings,
) -> HashMap<i16, u64> {
    let mut chains_blocks_map: HashMap<i16, u64> = HashMap::new();
    let min_height = settings.min_height;
    // Min height for chains gt>9
    let min_height2 = settings.chain_fork_height;

    for chain_id in 0..settings.number_of_chains {
        match processed_blocks.iter().find(|b| b.chain_id == chain_id) {
            Some(v) => {
                let next_height = v.height + 1;
                chains_blocks_map.insert(chain_id, next_height as u64);
                // next_height as u64
            }
            None => {
                let m = if chain_id > 9 && min_height < min_height2 {
                    min_height2
                } else {
                    min_height
                };
                chains_blocks_map.insert(chain_id, m);
            }
        }
    }
    chains_blocks_map
}
