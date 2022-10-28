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
pub fn format_endpoint_with_query_params(params: &QueryParams) -> String {
    let mut endpoint = String::from("/header/branch");

    endpoint.push_str(format!("?limit={}", params.limit).as_str());
    endpoint.push_str(format!("&minheight={}", params.min_height).as_str());
    endpoint.push_str(format!("&maxheight={}", params.max_height).as_str());

    endpoint
}
