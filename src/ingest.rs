use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use serde_json::json;
use sqlx::PgPool;
use std::str;

use crate::entities::Block;
use crate::types::{
    BlockHeaderItems, BlockPayload, CurrentCut, HashHeight, NewHead, Output, Transaction,
    TransactionWithCmdSigs,
};
use crate::utils::{
    decode_from_base64_url, format_endpoint_with_query_params, req_header_content_type,
    req_header_content_type_with_accept,
};

///
/// TODOs:
/// - Fetch multiple blocks at once
/// - Use `StateManager` to manage indexer state
///
/// New heads should be handled outside of the indexer i.e NewHeadManager
/// The indexer can fetch new_heads from a queue or a channel and update
/// chain_head field. chain_head is used by the indexer to determine the
/// higest block for us for fetching blocks.
///
#[derive(Debug, Clone)]
pub struct Ingest {
    pub chain_id: u16,
    pub base_url: String,
    pub http_client: reqwest::Client,
    pub cut_url: String,
    pub qparams: QueryParams,
    pub root_url: String,
    pub chain_head: HashHeight,
    pub pool: PgPool,
}

/// Query paramaters for endpoint
#[derive(Default, Debug, Clone)]
pub struct QueryParams {
    pub min_height: u64,
    pub max_height: u64,
    pub limit: u64,
}

impl QueryParams {
    pub fn new(limit: u64, min_height: u64) -> Self {
        Self {
            limit,
            min_height,
            max_height: limit,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ApiFetchResult {
    #[error("{0}")]
    Success(String),
    #[error("{0}")]
    Failure(#[from] anyhow::Error),
}

impl Ingest {
    pub fn new(chain_id: u16, base_url: String, params: QueryParams, pool: PgPool) -> Self {
        let http_client = reqwest::Client::new();
        let cut_url = format!("{}/cut", base_url);
        let root_url = base_url.clone();
        let base_url = format!(
            "{}/chain/{}{}",
            base_url,
            chain_id,
            format_endpoint_with_query_params(&params)
        );
        Self {
            chain_id,
            base_url,
            http_client,
            cut_url,
            qparams: params,
            root_url,
            chain_head: HashHeight {
                hash: "".to_string(),
                height: 0,
            },
            pool,
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let cut = self.current_cut().await?;

        let hh = cut.hashes.get(&self.chain_id).unwrap();
        self.chain_head = hh.clone();
        self.qparams.max_height = self.qparams.limit + self.qparams.min_height;

        let mut i = 0;
        loop {
            match self.blocks().await {
                Ok(_) => {
                    self.qparams.min_height += self.qparams.limit;
                    i += 1;
                    if i == 1 {
                        return Ok(());
                    }
                }
                Err(e) => println!("{}", e),
            }
        }
    }

    pub async fn current_cut(&self) -> Result<CurrentCut, ApiFetchResult> {
        let resp = retry(ExponentialBackoff::default(), || async {
            let cut = self
                .http_client
                .get(&self.cut_url)
                .send()
                .await
                .context("Failed to send a request")?;
            let status = cut.status().as_u16();
            if status != 200 {
                let err = format!("Error! Got status {}", status);
                Err(backoff::Error::transient(anyhow::anyhow!(err)))
            } else {
                // dbg!("Got status {} for chain {}", status, self.chain_id);
                let cut_as_json: CurrentCut = cut
                    .json()
                    .await
                    .context("Failed to convert response to json.")?;
                Ok(cut_as_json)
            }
        })
        .await
        .context("Failed to fetch cut from chainweb node.")
        .map_err(ApiFetchResult::Failure)?;

        Ok(resp)
    }

    pub async fn listen_to_new_heads(&mut self) -> Result<(), ApiFetchResult> {
        let url = format!("{}/header/updates", self.root_url);
        let mut res = reqwest::get(url)
            .await
            .context("Failed to make request to fetch header updates")
            .map_err(ApiFetchResult::Failure)?;
        dbg!("{}", res.status().as_u16());

        while let Some(chunk) = res
            .chunk()
            .await
            .context("Failed to read response chunk")
            .map_err(ApiFetchResult::Failure)?
        {
            println!("Received a new head");

            // Remove non-json from received data
            let s = str::from_utf8(&chunk[23..]).unwrap();

            let _new_head: NewHead = serde_json::from_str(s).unwrap();
        }
        Ok(())
    }

    pub async fn blocks_headers(&self) -> Result<BlockHeaderItems, ApiFetchResult> {
        // let current_cut = self.current_cut().await?;
        // Should never panic because we have all available chain ids
        // let hh = current_cut.hashes.get(&self.chain_id).unwrap();

        let body = json!({
            "upper": [self.chain_head.hash],
            "lower": []
        });

        let url = format!(
            "{}/chain/{}{}",
            self.root_url,
            self.chain_id,
            format_endpoint_with_query_params(&self.qparams)
        );
        // println!("{}", url);
        let resp = retry(ExponentialBackoff::default(), || async {
            let resp = self
                .http_client
                .post(url.clone())
                .headers(req_header_content_type_with_accept())
                .json(&body)
                .send()
                .await
                .context("Failed to send a request")?;

            let status = resp.status().as_u16();
            if status != 200 {
                let detail = format!("Error while fetching block headers! Got status {}", status);
                let err = backoff::Error::transient(anyhow::anyhow!(detail));
                Err(err)
            } else {
                // dbg!("Got status {} for chain {}", status, self.chain_id);
                let block_headers_json: BlockHeaderItems = resp
                    .json()
                    .await
                    .context("Failed to convert response to json.")?;
                Ok(block_headers_json)
            }
        })
        .await
        .context("Failed to fetch block headers from chainweb node.")
        .map_err(ApiFetchResult::Failure)?;

        Ok(resp)
    }

    pub async fn blocks(&mut self) -> Result<(), ApiFetchResult> {
        let blocks_headers = self.blocks_headers().await?;
        println!("Block headers len: {}", blocks_headers.items.len());

        // headers are ordered by descending
        self.qparams.max_height = self.qparams.limit + blocks_headers.items[0].height;

        let payloads_hashes = &blocks_headers
            .items
            .iter()
            .map(|i| i.payload_hash.clone())
            .collect::<Vec<String>>();
        let body = json!(payloads_hashes);

        let blocks_payloads = retry(ExponentialBackoff::default(), || async {
            let url = format!(
                "{}/chain/{}/payload/outputs/batch",
                self.root_url, self.chain_id
            );
            let resp = self
                .http_client
                .post(url)
                .headers(req_header_content_type())
                .json(&body)
                .send()
                .await
                .context("Failed to send a request")?;
            let status = resp.status().as_u16();
            if status != 200 {
                let detail = format!(
                    "Error: Failed fetching block headers! Got http status {}",
                    status
                );
                dbg!(&detail);
                let err = backoff::Error::transient(anyhow::anyhow!(detail));
                Err(err)
            } else {
                let block_headers_json = resp
                    .json::<Vec<BlockPayload>>()
                    .await
                    .context("Failed to convert response to json.")?;
                Ok(block_headers_json)
            }
        })
        .await
        .context("Failed to fetch block headers from chainweb node.")
        .map_err(ApiFetchResult::Failure)?;

        let mut txs = vec![];
        for block_payload in blocks_payloads {
            if let Some(v) = block_payload.transactions {
                txs.push(self.process_block_transactions(v))
            }
        }

        for item in blocks_headers.items {
            let block = Block::new(item.chain_id, item.height);
            block
                .insert(&self.pool)
                .await
                .context("Failed to insert block to database")
                .map_err(ApiFetchResult::Failure)?;
        }

        Ok(())
    }

    fn process_block_transactions(&self, transactions: Vec<Vec<String>>) -> Vec<Transaction> {
        let mut txs = vec![];
        for transaction in transactions {
            let decoded_tx = decode_from_base64_url(&transaction[0]);
            let decoded_tx_output = decode_from_base64_url(&transaction[1]);

            // for debugging purpose to see what goes wrong with the next type conversation
            let _o: serde_json::Value = serde_json::from_slice(&decoded_tx_output).unwrap();
            // debug!("{}", _o);
            let _out: Output = serde_json::from_slice(&decoded_tx_output).unwrap();

            // for debugging purpose to see what goes wrong with the next type conversation
            let _t: serde_json::Value = serde_json::from_slice(&decoded_tx).unwrap();
            // debug!("{}", _t);
            let tx: TransactionWithCmdSigs = serde_json::from_slice(&decoded_tx).unwrap();

            // debug!("TX: {:?}", tx);
            // debug!("OUTPUT: {:?}", out);
            // let tx: TransactionWithCmdSigs = serde_json::from_value(_tx).unwrap();
            // cmd is string so to make serde works we need to use ::from_str then ::from_value
            let _cmd: Transaction =
                serde_json::from_value(serde_json::from_str(&tx.cmd).unwrap()).unwrap();
            txs.push(_cmd);
        }
        txs
    }
}
