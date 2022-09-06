use reqwest::header::HeaderMap;

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
