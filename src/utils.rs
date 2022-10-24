use reqwest::header::HeaderMap;

use crate::ingest::QueryParams;

///
/// Create chain URL for a specific chain id
pub fn chain_url(base_url: &str, chain_id: u64, endpoint: &str) -> String {
    format!("{}/chain/{}{}", base_url, chain_id, endpoint)
}

pub fn req_header_content_type() -> HeaderMap {
    let mut map = HeaderMap::new();
    map.insert("Content-Type", "application/json".parse().unwrap());
    map
}

pub fn req_header_content_type_with_accept() -> HeaderMap {
    let mut map = req_header_content_type();
    map.insert(
        "Accept",
        "application/json;blockheader-encoding=object"
            .parse()
            .unwrap(),
    );
    map
}

pub fn decode_from_base64_url(input: &str) -> Vec<u8> {
    base64_url::decode(input).unwrap()
}

// Create url endpoint with query parameters
pub fn format_endpoint_with_query_params(params: Option<QueryParams>) -> String {
    let mut endpoint = String::from("/header/branch");

    let mut lmt = 1;
    if let Some(query_params) = params {
        if let Some(l) = query_params.limit {
            lmt = l
        };
        endpoint.push_str(format!("?limit={}", lmt).as_str());

        if let Some(h) = query_params.min_height {
            endpoint.push_str(format!("&minheight={}", h).as_str());
        }
        if let Some(h) = query_params.max_height {
            endpoint.push_str(format!("&maxheight={}", h).as_str());
        }
    } else {
        endpoint.push_str(format!("?limit={}", lmt).as_str());
    }

    endpoint
}
