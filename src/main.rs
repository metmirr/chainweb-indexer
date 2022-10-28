use std::time::Instant;

use chainweb_indexer::ingest::{Ingest, QueryParams};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let start = Instant::now();

    let mut workers = vec![];

    for i in 0..10 {
        let worker = tokio::spawn(async move {
            let limit = 20;
            let start_height = 0;
            let url = "https://api.chainweb.com/chainweb/0.0/mainnet01".to_string();

            _ = Ingest::new(i, url, QueryParams::new(limit, start_height))
                .start()
                .await;
        });
        workers.push(worker);
    }

    for worker in workers {
        worker.await.unwrap();
    }

    println!("{}", start.elapsed().as_secs());

    Ok(())
}
