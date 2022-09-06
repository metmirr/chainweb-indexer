use crate::utils::{
    chain_url, decode_from_base64_url, req_header_content_type, req_header_content_type_with_accept,
};

use backoff::future::retry;
use backoff::ExponentialBackoff;
use log::debug;
use reqwest::header::HeaderMap;
use std::str;
use std::time::Instant;

use crate::types::{
    BlockHeader, BlockHeaderItems, BlockPayload, CurrentCut, Output, Transaction,
    TransactionWithCmdSigs,
};

pub enum HttpMethod {
    GET,
    POST,
}

pub async fn api_call_with_retry(
    method: HttpMethod,
    url: String,
    body: Option<String>,
    headers: Option<HeaderMap>,
) -> Result<reqwest::Response, reqwest::Error> {
    retry(ExponentialBackoff::default(), || async {
        // debug!("Fetching {}", url);
        let client = reqwest::Client::new();
        let mut req = match method {
            HttpMethod::GET => client.get(url.clone()),
            HttpMethod::POST => client.post(url.clone()),
        };

        if let Some(b) = body.clone() {
            req = req.body(b);
        }
        if let Some(h) = headers.clone() {
            req = req.headers(h);
        }
        Ok(req.send().await?)
    })
    .await

    // let client = reqwest::Client::new();
    // let mut req = match method {
    //     HttpMethod::GET => client.get(url),
    //     HttpMethod::POST => client.post(url),
    // };

    // if let Some(b) = body {
    //     req = req.body(b);
    // }
    // if let Some(h) = headers {
    //     req = req.headers(h);
    // }
    // req.send().await
}

// pub async fn api_call_with_retry(
//     method: HttpMethod,
//     url: String,
//     body: Option<String>,
//     headers: Option<HeaderMap>,
// ) -> Result<reqwest::Response, reqwest::Error> {
//     retry(ExponentialBackoff::default(), || async {
//         println!("Fetching {}", url);
//         Ok(api_call(method, url, body, headers).await?)
//     })
//     .await
// }

pub async fn cut(url: &str) -> Result<CurrentCut, reqwest::Error> {
    let now = Instant::now();
    let c = api_call_with_retry(HttpMethod::GET, format!("{}{}", url, "/cut"), None, None)
        .await
        .unwrap();
    let c = c.json().await?;
    debug!("Took {}ms to fetch cut data", now.elapsed().as_millis());
    Ok(c)
}

pub async fn listen_to_new_heads(url: &str) -> Result<(), reqwest::Error> {
    let mut res = reqwest::get(format!("{}{}", &url, "/header/updates")).await?;

    while let Some(chunk) = res.chunk().await? {
        debug!("Received a new head");
        // println!("Chunk: {:?}", &chunk[10..]);
        // let r = serde_json::from_slice(&chunk).unwrap();

        // Remove non-json from data
        let s = str::from_utf8(&chunk[23..]).unwrap();
        // println!("{:#?}", s);

        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        // println!("{}", v);

        // println!("{}", serde_json::from_slice(&chunk).unwrap())
    }
    Ok(())
}

pub async fn headers(
    url: &'static str,
    min_h: Option<u64>,
    max_h: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<BlockHeader>, reqwest::Error> {
    let start = Instant::now();

    let mut endpoint = String::from("/header/branch");
    let mut lmt = 1;

    if let Some(l) = limit {
        lmt = l;
    }
    endpoint.push_str(format!("?limit={}", lmt).as_str());

    if let Some(h) = min_h {
        endpoint.push_str(format!("&minheight={}", h).as_str());
    }
    if let Some(h) = max_h {
        endpoint.push_str(format!("&maxheight={}", h).as_str());
    }

    let current_cut = cut(url).await?;
    let hashes = current_cut.hashes.clone();
    let req_body = |upper: &str| format!("{{\"upper\": [\"{}\"], \"lower\": []}}", upper);

    let mut blocks_headers: Vec<BlockHeader> = vec![];
    let mut handles = vec![];

    let ids = hashes.keys().copied().collect::<Vec<u64>>();

    for chain_id in ids {
        let upper = hashes.get(&chain_id).unwrap().clone();
        let e = endpoint.clone();

        let s = tokio::spawn(async move {
            api_call_with_retry(
                HttpMethod::POST,
                chain_url(url, chain_id, &e),
                Some(req_body(&upper.hash)),
                Some(req_header_content_type_with_accept()),
            )
            .await
        });
        handles.push(s);
    }
    for handle in handles {
        let mut a: BlockHeaderItems = handle.await.unwrap().unwrap().json().await?;
        blocks_headers.append(&mut a.items);
    }

    debug!(
        "Took {}s to fetch {} blocks headers",
        start.elapsed().as_secs(),
        blocks_headers.len()
    );
    Ok(blocks_headers)
}

pub async fn blocks(url: &'static str) -> Result<Vec<BlockPayload>, reqwest::Error> {
    let start = Instant::now();
    let mut blocks_payloads: Vec<BlockPayload> = vec![];
    let blocks_headers = headers(url, None, None, Some(1)).await?;
    let mut handles = vec![];

    for header in blocks_headers {
        let handle = tokio::spawn(async move {
            api_call_with_retry(
                HttpMethod::POST,
                chain_url(url, header.chain_id, "/payload/outputs/batch"),
                Some(format!("[\"{}\"]", header.payload_hash)),
                Some(req_header_content_type()),
            )
            .await
        });
        handles.push(handle);
    }

    for handle in handles {
        let out = handle.await.unwrap();
        let full = out.unwrap().bytes().await?;
        let v: serde_json::Value = serde_json::from_slice(&full).unwrap();
        println!("{:?}", v);
        let mut a: Vec<BlockPayload> = serde_json::from_slice(&full).unwrap();
        blocks_payloads.append(&mut a);

        // match out {
        //     Ok(result) => {
        //         let full = result.bytes().await?;

        //         let v: serde_json::Value = serde_json::from_slice(&full).unwrap();
        //         println!("{:?}", v);
        //         let mut a: Vec<BlockPayload> = serde_json::from_slice(&full).unwrap();
        //         blocks_payloads.append(&mut a);
        //     }
        //     Err(err) => panic!("Error2{}", err),
        // }
    }

    for payload in &blocks_payloads {
        let block_txs = &payload.transactions;
        if block_txs.is_some() {
            let _txs = block_transactions(block_txs.as_ref().unwrap());
        }
    }

    debug!(
        "Took {}s to fetch {} blocks",
        start.elapsed().as_secs(),
        blocks_payloads.len()
    );
    Ok(blocks_payloads)
}

pub fn block_transactions(transactions: &Vec<Vec<String>>) -> Vec<Transaction> {
    let txs = vec![];
    for transaction in transactions {
        let decoded_tx = decode_from_base64_url(&transaction[0]);
        let decoded_tx_output = decode_from_base64_url(&transaction[1]);

        // for debugging purpose to see what goes wrong with the next type conversation
        let _o: serde_json::Value = serde_json::from_slice(&decoded_tx_output).unwrap();
        debug!("{}", _o);
        let _out: Output = serde_json::from_slice(&decoded_tx_output).unwrap();

        // for debugging purpose to see what goes wrong with the next type conversation
        let _t: serde_json::Value = serde_json::from_slice(&decoded_tx).unwrap();
        debug!("{}", _t);
        let tx: TransactionWithCmdSigs = serde_json::from_slice(&decoded_tx).unwrap();

        // debug!("TX: {:?}", tx);
        // debug!("OUTPUT: {:?}", out);
        // let tx: TransactionWithCmdSigs = serde_json::from_value(_tx).unwrap();
        // cmd is string so to make serde works we need to use ::from_str then ::from_value
        let _cmd: Transaction =
            serde_json::from_value(serde_json::from_str(&tx.cmd).unwrap()).unwrap();
    }
    txs
}
