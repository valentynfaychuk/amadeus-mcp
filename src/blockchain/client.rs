use super::{
    error::{BlockchainError, Result},
    types::*,
};
use crate::wasm::tx;
use reqwest::{header, Client, Response};
use std::time::Duration;
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};
use tracing::warn;

#[derive(Clone)]
pub struct BlockchainClient {
    client: Client,
    base_url: String,
}

impl BlockchainClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent("amadeus-mcp/0.1.0")
            .build()
            .map_err(BlockchainError::HttpRequest)?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    #[tracing::instrument(skip(self), fields(contract=%req.contract, function=%req.function))]
    pub async fn create_transaction_blob(
        &self,
        req: TransactionRequest,
    ) -> Result<UnsignedTransactionBlob> {
        let signer_pk = bs58::decode(&req.signer)
            .into_vec()
            .map_err(|_| BlockchainError::ValidationFailed("invalid signer base58".into()))?;

        let args: Result<Vec<Vec<u8>>> = req.args.iter().map(|arg| match arg {
            Argument::String(s) => Ok(s.as_bytes().to_vec()),
            Argument::Number(n) => Ok(n.to_string().as_bytes().to_vec()),
            Argument::Base58 { b58 } => bs58::decode(b58)
                .into_vec()
                .map_err(|_| BlockchainError::ValidationFailed("invalid base58 arg".into())),
            Argument::Hex { hex } => hex::decode(hex.trim_start_matches("0x"))
                .map_err(|_| BlockchainError::ValidationFailed("invalid hex arg".into())),
            Argument::Utf8 { utf8 } => Ok(utf8.as_bytes().to_vec()),
        }).collect();
        let args = args?;

        let attached_symbol = req.attached_symbol.as_ref().map(|s| s.as_bytes());
        let attached_amount = req.attached_amount.as_ref().map(|s| s.as_bytes());

        let unsigned = tx::build_unsigned(
            &signer_pk,
            &req.contract,
            &req.function,
            &args,
            attached_symbol,
            attached_amount,
            req.nonce,
        ).map_err(|e| BlockchainError::ValidationFailed(e.into()))?;

        Ok(UnsignedTransactionBlob {
            blob: bs58::encode(&unsigned.tx_blob).into_string(),
            signing_payload: hex::encode(unsigned.signing_hash),
            transaction_hash: bs58::encode(unsigned.signing_hash).into_string(),
            tx_bytes: unsigned.tx_blob,
        })
    }

    #[tracing::instrument(skip(self, tx), fields(tx_hash))]
    pub async fn submit_signed_transaction(&self, tx: SignedTransaction, url: &str) -> Result<SubmitResponse> {
        let finalized = tx::finalize_transaction(&tx.transaction, &tx.signature)
            .map_err(|e| BlockchainError::ValidationFailed(e.into()))?;
        let tx_hash = bs58::encode(&finalized.hash).into_string();
        let txu_b58 = bs58::encode(&finalized.packed).into_string();
        let full_url = format!("{}/api/tx/submit", url);

        let response = self.client
            .post(&full_url)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(txu_b58)
            .send()
            .await
            .map_err(BlockchainError::HttpRequest)?;

        if !response.status().is_success() {
            return Err(BlockchainError::InvalidResponse(format!("HTTP {}", response.status())));
        }

        let api_response: serde_json::Value = self.parse_response(response).await?;
        let error = api_response.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");

        Ok(SubmitResponse {
            error: error.to_string(),
            tx_hash: if error == "ok" { Some(tx_hash) } else { None },
        })
    }

    #[tracing::instrument(skip(self), fields(address=%address))]
    pub async fn get_account_balance(&self, address: &str) -> Result<AccountBalance> {
        let path = format!("/api/wallet/balance_all/{}", address);
        let response = self.retry_request("GET", &path, None).await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::AccountNotFound {
                address: address.to_string(),
            });
        }

        let balances_data = api_response.get("balances").ok_or_else(|| {
            BlockchainError::InvalidResponse("missing balances field".to_string())
        })?;

        let balances: Vec<Balance> =
            serde_json::from_value(balances_data.clone()).map_err(|e| {
                BlockchainError::InvalidResponse(format!("failed to parse balances: {}", e))
            })?;

        Ok(AccountBalance {
            address: address.to_string(),
            balances,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_chain_stats(&self) -> Result<ChainStats> {
        let response = self.retry_request("GET", "/api/chain/stats", None).await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

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

    #[tracing::instrument(skip(self), fields(height=%height))]
    pub async fn get_block_by_height(&self, height: u64) -> Result<Vec<BlockEntry>> {
        let path = format!("/api/chain/height/{}", height);
        let response = self.retry_request("GET", &path, None).await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::InvalidResponse(
                "failed to get block entries".to_string(),
            ));
        }

        let entries = api_response
            .get("entries")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing entries field".to_string()))?;

        serde_json::from_value(entries.clone()).map_err(|e| {
            BlockchainError::InvalidResponse(format!("failed to parse entries: {}", e))
        })
    }

    #[tracing::instrument(skip(self), fields(tx_hash=%tx_hash))]
    pub async fn get_transaction(&self, tx_hash: &str) -> Result<Transaction> {
        let path = format!("/api/chain/tx/{}", tx_hash);
        let response = self.retry_request("GET", &path, None).await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

        if api_response.get("error").and_then(|e| e.as_str()) == Some("not_found") {
            return Err(BlockchainError::InvalidResponse(
                "transaction not found".to_string(),
            ));
        }

        let transaction = api_response.get("transaction").ok_or_else(|| {
            BlockchainError::InvalidResponse("missing transaction field".to_string())
        })?;

        serde_json::from_value(transaction.clone()).map_err(|e| {
            BlockchainError::InvalidResponse(format!("failed to parse transaction: {}", e))
        })
    }

    #[tracing::instrument(skip(self), fields(address=%address))]
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
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response = self.retry_request("GET", &path, None).await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

        let txs = api_response
            .get("txs")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing txs field".to_string()))?;

        serde_json::from_value(txs.clone())
            .map_err(|e| BlockchainError::InvalidResponse(format!("failed to parse txs: {}", e)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_validators(&self) -> Result<Vec<String>> {
        let response = self
            .retry_request("GET", "/api/peer/trainers", None)
            .await?;
        let api_response: serde_json::Value = self.parse_response(response).await?;

        if api_response.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::InvalidResponse(
                "failed to get validators".to_string(),
            ));
        }

        let trainers = api_response.get("trainers").ok_or_else(|| {
            BlockchainError::InvalidResponse("missing trainers field".to_string())
        })?;

        serde_json::from_value(trainers.clone()).map_err(|e| {
            BlockchainError::InvalidResponse(format!("failed to parse trainers: {}", e))
        })
    }

    #[tracing::instrument(skip(self), fields(contract=%contract_address, key=%key))]
    pub async fn get_contract_state(
        &self,
        contract_address: &str,
        key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!("/api/contract/get/{}/{}", contract_address, key);
        let response = self.retry_request("GET", &path, None).await?;
        self.parse_response(response).await
    }

    async fn retry_request(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response> {
        let retry_strategy = ExponentialBackoff::from_millis(100).map(jitter).take(3);

        let url = format!("{}{}", self.base_url, path);

        Retry::spawn(retry_strategy, || async {
            let mut request = match method {
                "GET" => self.client.get(&url),
                "POST" => self.client.post(&url),
                _ => {
                    return Err(BlockchainError::Configuration(format!(
                        "unsupported method: {}",
                        method
                    )))
                }
            };

            request = request.header(header::CONTENT_TYPE, "application/json");

            if let Some(json) = body {
                request = request.json(json);
            }

            request
                .send()
                .await
                .map_err(BlockchainError::HttpRequest)
                .and_then(|resp| {
                    if resp.status().is_success() {
                        Ok(resp)
                    } else {
                        Err(BlockchainError::InvalidResponse(format!(
                            "HTTP {}: request failed",
                            resp.status()
                        )))
                    }
                })
        })
        .await
        .map_err(|e| {
            warn!("retry exhausted: {}", e);
            BlockchainError::NetworkRetryExhausted { attempts: 3 }
        })
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(BlockchainError::HttpRequest)?;

        serde_json::from_str(&body).map_err(|e| {
            BlockchainError::InvalidResponse(format!(
                "failed to parse response (status {}): {}",
                status, e
            ))
        })
    }
}
