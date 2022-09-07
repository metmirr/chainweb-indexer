use std::str;
use std::{collections::HashMap, time::Instant};

use backoff::{future::retry, ExponentialBackoff};
use log::debug;

use crate::{
    types::{BlockHeaderItems, CurrentCut, NewHead},
    utils::{format_endpoint_with_query_params, req_header_content_type_with_accept},
    BlockHeader,
};

/// Query paramaters for endpoint
pub struct QueryParams {
    pub min_height: Option<u64>,
    pub max_height: Option<u64>,
    pub limit: Option<u16>,
}

/// Ingest chainweb blocks
#[derive(Debug)]
pub struct Ingest {
    client: reqwest::Client,
    base_url: String,
    cut_endpoint: String,
    headers_endpoints: HashMap<u64, String>,
    new_headers_endpoint: String,
}

impl Ingest {
    /// Create a new instance Ingest, based on number of chains it generates full urls
    /// to fetch data from each chain
    pub fn new(base_url: String, num_of_chains: u64, params: Option<QueryParams>) -> Self {
        let endpoint = format_endpoint_with_query_params(params);

        let mut headers_map = HashMap::new();
        for chain_id in 0..num_of_chains {
            // Create a full url with base url, chain id and query params
            headers_map.insert(
                chain_id,
                format!("{}/chain/{}{}", base_url, chain_id, endpoint),
            );
        }

        Self {
            client: reqwest::Client::new(),
            base_url: base_url.clone(),
            cut_endpoint: format!("{}/cut", base_url),
            new_headers_endpoint: format!("{}{}", base_url, "/header/updates"),
            headers_endpoints: headers_map,
        }
    }

    /// Fetch the current chain cut
    pub async fn current_cut(&self) -> Result<CurrentCut, reqwest::Error> {
        let ccut = retry(ExponentialBackoff::default(), || async {
            debug!("Fetching current cut");
            Ok(reqwest::get(&self.cut_endpoint)
                .await?
                .json::<CurrentCut>()
                .await?)
        })
        .await?;

        Ok(ccut)
    }

    /// Listen to new chains heads
    pub async fn listen_new_heads(&self) -> Result<(), reqwest::Error> {
        let mut res = reqwest::get(format!("{}{}", self.base_url, "/header/updates")).await?;

        while let Some(chunk) = res.chunk().await? {
            // Remove non-json from received data
            let s = str::from_utf8(&chunk[23..]).unwrap();

            let _v: NewHead = serde_json::from_str(s).unwrap();
            debug!(
                "Received new head on chain {} at {}",
                _v.header.chain_id, _v.header.height
            );

            // TODO: push to redis queue
        }
        Ok(())
    }

    pub async fn blocks_headers(&self) -> Result<Vec<BlockHeader>, reqwest::Error> {
        let start = Instant::now();

        let current_cut = self.current_cut().await?;
        let hashes = current_cut.hashes.clone();
        let mut headers: Vec<BlockHeader> = vec![];
        let mut handles = vec![];

        let chain_ids = hashes.keys().copied().collect::<Vec<u64>>();

        for chain_id in chain_ids {
            let op = move |url: String, body: String| async {
                debug!("Fetching header payload");
                let client = reqwest::Client::new();
                let result = client
                    .post(url)
                    .headers(req_header_content_type_with_accept())
                    .body(body)
                    .send()
                    .await?;
                Ok(result)
            };

            let full_url = self.headers_endpoints.get(&chain_id).unwrap().clone();
            let hash_height = hashes.get(&chain_id).unwrap();
            let req_body = format!(
                "{{\"upper\": [\"{}\"], \"lower\": []}}",
                hash_height.hash.clone()
            );

            let h = tokio::spawn(async move {
                retry(ExponentialBackoff::default(), || {
                    op(full_url.clone(), req_body.clone())
                })
                .await
            });
            handles.push(h);
        }

        for handle in handles {
            let mut a: BlockHeaderItems = handle.await.unwrap().unwrap().json().await?;
            headers.append(&mut a.items);
        }

        debug!(
            "Took {}s to fetch {} blocks headers",
            start.elapsed().as_secs(),
            headers.len()
        );

        Ok(headers)
    }

    pub async fn blocks(&self) {}
}
