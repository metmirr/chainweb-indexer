use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BlockHeaderItems {
    pub items: Vec<BlockHeader>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewHead {
    pub header: BlockHeader,
    pub pow_hash: String,
    pub target: String,
    pub tx_count: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub chain_id: u16,
    pub chainweb_version: String,
    pub creation_time: u64,
    pub epoch_start: u64,
    pub feature_flags: u64,
    pub hash: String,
    pub height: u64,
    pub nonce: String,
    pub parent: String,
    pub payload_hash: String,
    pub target: String,
    pub weight: String,
    pub adjacents: HashMap<u32, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HashHeight {
    pub hash: String,
    pub height: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentCut {
    pub hashes: HashMap<i16, HashHeight>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayload {
    pub coinbase: String,
    pub miner_data: String,
    pub outputs_hash: String,
    pub payload_hash: String,
    pub transactions_hash: String,
    pub transactions: Option<Vec<Vec<String>>>, // pub transactions: Vec<Transaction>
}

#[derive(Deserialize, Debug, Clone)]
pub struct TransactionWithCmdSigs {
    pub cmd: String,
    pub sigs: Vec<Sig>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Sig {
    pub sig: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub network_id: String,
    pub payload: PayloadExec,
    pub signers: Option<Vec<Signer>>,
    pub meta: Meta,
    pub nonce: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PayloadExec {
    pub exec: Option<PayloadExecDataCode>,
    pub cont: Option<Continuation>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Continuation {
    pub proof: String,
    pub pact_id: String,
    pub rollback: bool,
    pub step: u8,
    pub data: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PayloadExecDataCode {
    pub data: serde_json::Value,
    pub code: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Signer {
    pub pub_key: String,
    pub clist: Option<Vec<Clist>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Clist {
    pub args: Vec<serde_json::Value>,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub creation_time: u64,
    pub ttl: f64,
    pub gas_limit: u64,
    pub chain_id: String,
    pub gas_price: f64,
    pub sender: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub continuation: serde_json::Value,
    pub events: Vec<Event>,
    pub gas: u64,
    pub logs: String,
    pub meta_data: Option<serde_json::Value>,
    pub req_key: String,
    pub result: OutputResult,
    pub tx_id: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub module: Module,
    pub module_hash: String,
    pub name: String,
    pub params: Vec<serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Module {
    pub name: String,
    pub namespace: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutputResult {
    pub error: Option<ResultError>,
    pub data: Option<serde_json::Value>,
    pub status: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResultError {
    pub call_stack: Vec<String>,
    pub info: String,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}
