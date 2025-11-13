use super::{
    error::{BlockchainError, Result},
    types::*,
};
use serde_json::json;
use worker::{Fetch, Method, Request, RequestInit};

#[derive(Clone)]
pub struct BlockchainClient {
    base_url: String,
    api_key: Option<String>,
}

impl BlockchainClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Result<Self> {
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub async fn create_transfer_blob(&self, req: TransferRequest) -> Result<UnsignedTransactionBlob> {
        let payload = json!({
            "type": "transfer",
            "from": req.source,
            "to": req.destination,
            "asset": req.symbol,
            "amount": req.amount,
            "memo": req.memo,
        });

        self.request("POST", "/api/v1/tx/build", Some(&payload)).await
    }

    pub async fn submit_signed_transaction(&self, tx: SignedTransaction) -> Result<SubmitResponse> {
        let payload = json!({
            "transaction": tx.transaction,
            "signature": tx.signature,
        });

        self.request("POST", "/api/v1/tx/submit", Some(&payload)).await
    }

    pub async fn get_account_balance(&self, address: &str) -> Result<AccountBalance> {
        let path = format!("/api/wallet/balance_all/{}", address);
        let api_response: serde_json::Value = self.request("GET", &path, None).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::AccountNotFound {
                address: address.to_string(),
            });
        }

        let balances_data = api_response
            .get("balances")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing balances field".to_string()))?;

        let balances: Vec<Balance> = serde_json::from_value(balances_data.clone())
            .map_err(|e| BlockchainError::InvalidResponse(format!("failed to parse balances: {}", e)))?;

        Ok(AccountBalance {
            address: address.to_string(),
            balances,
        })
    }

    pub async fn get_chain_stats(&self) -> Result<ChainStats> {
        let api_response: serde_json::Value = self.request("GET", "/api/chain/stats", None).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::InvalidResponse(
                "failed to get chain stats".to_string(),
            ));
        }

        let stats = api_response
            .get("stats")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing stats field".to_string()))?;

        serde_json::from_value(stats.clone())
            .map_err(|e| BlockchainError::InvalidResponse(format!("failed to parse stats: {}", e)))
    }

    pub async fn get_block_by_height(&self, height: u64) -> Result<Vec<BlockEntry>> {
        let path = format!("/api/chain/height/{}", height);
        let api_response: serde_json::Value = self.request("GET", &path, None).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::InvalidResponse(
                "failed to get block entries".to_string(),
            ));
        }

        let entries = api_response
            .get("entries")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing entries field".to_string()))?;

        serde_json::from_value(entries.clone())
            .map_err(|e| BlockchainError::InvalidResponse(format!("failed to parse entries: {}", e)))
    }

    pub async fn get_transaction(&self, tx_hash: &str) -> Result<Transaction> {
        let path = format!("/api/chain/tx/{}", tx_hash);
        let api_response: serde_json::Value = self.request("GET", &path, None).await?;

        if api_response.get("error").and_then(|e| e.as_str()) == Some("not_found") {
            return Err(BlockchainError::InvalidResponse(
                "transaction not found".to_string(),
            ));
        }

        let transaction = api_response
            .get("transaction")
            .ok_or_else(|| {
                BlockchainError::InvalidResponse("missing transaction field".to_string())
            })?;

        serde_json::from_value(transaction.clone()).map_err(|e| {
            BlockchainError::InvalidResponse(format!("failed to parse transaction: {}", e))
        })
    }

    pub async fn get_transaction_history(
        &self,
        address: &str,
        limit: Option<u32>,
        offset: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<Transaction>> {
        let mut path = format!("/api/chain/tx_events_by_account/{}", address);
        let mut params = vec![];

        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }

        if !params.is_empty() {
            path.push_str("?");
            path.push_str(&params.join("&"));
        }

        let api_response: serde_json::Value = self.request("GET", &path, None).await?;

        let txs = api_response
            .get("txs")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing txs field".to_string()))?;

        serde_json::from_value(txs.clone())
            .map_err(|e| BlockchainError::InvalidResponse(format!("failed to parse txs: {}", e)))
    }

    pub async fn get_validators(&self) -> Result<Vec<String>> {
        let api_response: serde_json::Value = self.request("GET", "/api/peer/trainers", None).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::InvalidResponse(
                "failed to get validators".to_string(),
            ));
        }

        let trainers = api_response
            .get("trainers")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing trainers field".to_string()))?;

        serde_json::from_value(trainers.clone()).map_err(|e| {
            BlockchainError::InvalidResponse(format!("failed to parse trainers: {}", e))
        })
    }

    pub async fn get_contract_state(
        &self,
        contract_address: &str,
        key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!("/api/contract/get/{}/{}", contract_address, key);
        self.request("GET", &path, None).await
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let mut init = RequestInit::new();
        init.with_method(if method == "GET" { Method::Get } else { Method::Post });

        let mut headers = worker::Headers::new();
        headers.set("Content-Type", "application/json")
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        if let Some(key) = &self.api_key {
            headers.set("Authorization", &format!("Bearer {}", key))
                .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;
        }

        init.with_headers(headers);

        if let Some(json) = body {
            init.with_body(Some(serde_json::to_string(json)
                .map_err(|e| BlockchainError::Serialization(e))?
                .into()));
        }

        let request = Request::new_with_init(&url, &init)
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let mut response = Fetch::Request(request).send().await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            return Err(BlockchainError::InvalidResponse(format!(
                "HTTP {}: request failed",
                status
            )));
        }

        let text = response.text().await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        serde_json::from_str(&text).map_err(|e| {
            BlockchainError::InvalidResponse(format!(
                "failed to parse response: {}",
                e
            ))
        })
    }
}
