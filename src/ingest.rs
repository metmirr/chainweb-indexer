use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};

use crate::{
    types::{
        BlockHeaderItems, BlockPayload, CurrentCut, Output, Transaction, TransactionWithCmdSigs,
    },
    utils::{
        decode_from_base64_url, format_endpoint_with_query_params,
        req_header_content_type_with_accept,
    },
};

#[derive(Debug, Clone)]
pub struct Ingest {
    pub chain_id: u64,
    pub base_url: String,
    pub http_client: reqwest::Client,
    pub cut_url: String,
    pub qparams: Option<QueryParams>,
    pub root_url: String,
    pub start_height: u64,
    pub next_height: u64,
}

/// Query paramaters for endpoint
#[derive(Debug, Clone)]
pub struct QueryParams {
    pub min_height: Option<u64>,
    pub max_height: Option<u64>,
    pub limit: Option<u16>,
}

#[derive(thiserror::Error, Debug)]
pub enum ApiFetchResult {
    #[error("{0}")]
    Success(String),
    #[error("{0}")]
    Failure(#[from] anyhow::Error),
}

impl Ingest {
    pub fn new(
        chain_id: u64,
        base_url: String,
        start_height: u64,
        params: Option<QueryParams>,
    ) -> Self {
        let http_client = reqwest::Client::new();
        let cut_url = format!("{}/cut", base_url);
        let root_url = base_url.clone();
        let base_url = format!(
            "{}/chain/{}{}",
            base_url,
            chain_id,
            format_endpoint_with_query_params(params.clone())
        );
        Self {
            chain_id,
            base_url,
            http_client,
            cut_url,
            qparams: params,
            root_url,
            start_height,
            next_height: start_height,
        }
    }

    pub async fn loop_(&mut self) -> Result<(), anyhow::Error> {
        let mut i = 0;
        loop {
            match self.blocks().await {
                Ok(_) => {
                    self.next_height = self.start_height + 1;

                    i += 1;
                    if i == 10 {
                        return Ok(());
                    }
                }
                Err(e) => println!("{}", e),
            }
            self.listen_to_new_heads().await?;
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

    pub async fn listen_to_new_heads(&self) -> Result<(), ApiFetchResult> {
        let url = format!("{}/header/updates", self.root_url);
        println!("{}", url);
        let mut res = reqwest::get(url)
            .await
            .context("Failed to make request to fetch header updates")
            .map_err(ApiFetchResult::Failure)?;
        println!("{}", res.status().as_u16());

        while let Some(chunk) = res
            .chunk()
            .await
            .context("Failed to read response chunk")
            .map_err(ApiFetchResult::Failure)?
        {
            println!("Received a new head");

            // Remove non-json from received data
            let s = str::from_utf8(&chunk[23..]).unwrap();

            let new_head: NewHead = serde_json::from_str(s).unwrap();
            println!("{:?}", new_head);
        }
        Ok(())
    }

    pub async fn block_headers(&self) -> Result<BlockHeaderItems, ApiFetchResult> {
        let current_cut = self.current_cut().await?;
        // Should never panic since we have all available chain ids
        let hh = current_cut.hashes.get(&self.chain_id).unwrap();

        let resp = retry(ExponentialBackoff::default(), || async {
            let resp = self
                .http_client
                .post(self.base_url.clone())
                .headers(req_header_content_type_with_accept())
                .body(format!(
                    "{{\"upper\": [\"{}\"], \"lower\": []}}",
                    hh.hash.clone()
                ))
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

    pub async fn blocks(&self) -> Result<(), ApiFetchResult> {
        let block_headers = self.block_headers().await?;
        let block_header = &block_headers.items[0];

        let resp = retry(ExponentialBackoff::default(), || async {
            let url = format!(
                "{}/chain/{}/payload/{}/outputs",
                self.root_url, self.chain_id, block_headers.items[0].payload_hash
            );
            let resp = self
                .http_client
                .get(url)
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
                // dbg!("Fetched block payload");
                let block_headers_json = resp
                    .json::<BlockPayload>()
                    .await
                    .context("Failed to convert response to json.")?;
                Ok(block_headers_json)
            }
        })
        .await
        .context("Failed to fetch block headers from chainweb node.")
        .map_err(ApiFetchResult::Failure)?;

        let txs = if let Some(v) = resp.transactions {
            self.process_block_transactions(v)
        } else {
            vec![]
        };
        println!(
            "chain id: {} - block height: {} - txs: {}",
            self.chain_id,
            block_header.height,
            txs.len()
        );
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
