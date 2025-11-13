use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransferRequest {
    pub symbol: String,
    pub source: String,
    pub destination: String,
    pub amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTransactionBlob {
    pub blob: String,
    pub signing_payload: String,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SignedTransaction {
    pub transaction: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub transaction_hash: String,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AccountQuery {
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

// View tool types for querying blockchain data

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HeightQuery {
    pub height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransactionQuery {
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransactionHistoryQuery {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractStateQuery {
    pub contract_address: String,
    pub key: String,
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
