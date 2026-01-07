use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTransactionBlob {
    pub blob: String,
    pub signing_payload: String,
    pub transaction_hash: String,
    #[serde(skip)]
    pub tx_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct SignedTransaction {
    #[validate(length(min = 1))]
    pub transaction: String,
    #[validate(length(min = 1))]
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct AccountQuery {
    #[validate(length(min = 1))]
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub address: String,
    pub balances: Vec<Balance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub symbol: String,
    /// Balance in smallest unit (atoms)
    pub flat: u64,
    /// Human-readable balance
    pub float: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct HeightQuery {
    pub height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct TransactionQuery {
    #[validate(length(min = 1))]
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct TransactionHistoryQuery {
    #[validate(length(min = 1))]
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct ContractStateQuery {
    #[validate(length(min = 1))]
    pub contract_address: String,
    #[validate(length(min = 1))]
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Validate)]
pub struct TransactionRequest {
    #[validate(length(min = 1))]
    pub signer: String,
    #[validate(length(min = 1))]
    pub contract: String,
    #[validate(length(min = 1))]
    pub function: String,
    pub args: Vec<Argument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Argument {
    String(String),
    Number(i64),
    Base58 { b58: String },
    Hex { hex: String },
    Utf8 { utf8: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub height: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pflops: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burned: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circulating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_bits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_pool_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txs_per_sec: Option<f64>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEntry {
    pub hash: String,
    pub header_unpacked: HeaderUnpacked,
    pub tx_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus: Option<Consensus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderUnpacked {
    pub slot: u64,
    pub height: u64,
    pub dr: String,
    pub vr: String,
    pub prev_hash: String,
    pub signer: String,
    pub prev_slot: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consensus {
    pub score: f64,
    pub finality_reached: bool,
    pub mut_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub symbol: String,
    pub fee: String,
    pub nonce: u64,
    pub timestamp: u64,
    pub signature: String,
    #[serde(rename = "type")]
    pub tx_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub public_key: String,
    pub score: Option<f64>,
    pub epoch: Option<u64>,
    pub rank: Option<u32>,
}
