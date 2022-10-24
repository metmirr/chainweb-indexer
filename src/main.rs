use std::time::Instant;

use chainweb_indexer::ingest::Ingest;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let start = Instant::now();

    let mut workers = vec![];
    for i in 0..20 {
        let worker = tokio::spawn(async move {
            _ = Ingest::new(
                i,
                "https://api.chainweb.com/chainweb/0.0/mainnet01".to_string(),
                0,
                None,
            )
            .loop_()
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
