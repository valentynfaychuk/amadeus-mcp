use super::{
    error::{BlockchainError, Result},
    types::*,
};
use crate::wasm::tx;
use worker::{Fetch, Method, Request, RequestInit};

#[derive(Clone)]
pub struct BlockchainClient {
    base_url: String,
}

impl BlockchainClient {
    pub fn new(base_url: String) -> Result<Self> {
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

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

    pub async fn submit_signed_transaction(&self, tx: SignedTransaction, url: &str) -> Result<SubmitResponse> {
        let finalized = tx::finalize_transaction(&tx.transaction, &tx.signature)
            .map_err(|e| BlockchainError::ValidationFailed(e.into()))?;
        let tx_hash = bs58::encode(&finalized.hash).into_string();
        let txu_b58 = bs58::encode(&finalized.packed).into_string();
        let full_url = format!("{}/api/tx/submit", url);

        let mut init = RequestInit::new();
        init.with_method(Method::Post);

        let mut headers = worker::Headers::new();
        headers.set("Content-Type", "text/plain")
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;
        init.with_headers(headers);
        init.with_body(Some(txu_b58.into()));

        let request = Request::new_with_init(&full_url, &init)
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let mut response = Fetch::Request(request).send().await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            return Err(BlockchainError::InvalidResponse(format!("HTTP {}", status)));
        }

        let text = response.text().await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let api_response: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))?;
        let error = api_response.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");

        Ok(SubmitResponse {
            error: error.to_string(),
            tx_hash: if error == "ok" { Some(tx_hash) } else { None },
        })
    }

    pub async fn get_account_balance(&self, address: &str) -> Result<AccountBalance> {
        let path = format!("/api/wallet/balance_all/{}", address);
        let resp: serde_json::Value = self.request("GET", &path, None).await?;

        if resp.get("error").and_then(|e| e.as_str()) != Some("ok") {
            return Err(BlockchainError::AccountNotFound {
                address: address.to_string(),
            });
        }

        let balances = resp
            .get("balances")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing balances".into()))?;

        Ok(AccountBalance {
            address: address.to_string(),
            balances: serde_json::from_value(balances.clone())
                .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))?,
        })
    }

    pub async fn get_chain_stats(&self) -> Result<ChainStats> {
        let resp: serde_json::Value = self.request("GET", "/api/chain/stats", None).await?;

        let stats = resp
            .get("stats")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing stats".into()))?;

        serde_json::from_value(stats.clone())
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
    }

    pub async fn get_block_by_height(&self, height: u64) -> Result<Vec<BlockEntry>> {
        let path = format!("/api/chain/height/{}", height);
        let resp: serde_json::Value = self.request("GET", &path, None).await?;

        let entries = resp
            .get("entries")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing entries".into()))?;

        serde_json::from_value(entries.clone())
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
    }

    pub async fn get_transaction(&self, tx_hash: &str) -> Result<Transaction> {
        let path = format!("/api/chain/tx/{}", tx_hash);
        let resp: serde_json::Value = self.request("GET", &path, None).await?;

        if resp.get("error").and_then(|e| e.as_str()) == Some("not_found") {
            return Err(BlockchainError::InvalidResponse(
                "transaction not found".into(),
            ));
        }

        let tx = resp
            .get("transaction")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing transaction".into()))?;

        serde_json::from_value(tx.clone())
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
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
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let resp: serde_json::Value = self.request("GET", &path, None).await?;
        let txs = resp
            .get("txs")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing txs".into()))?;

        serde_json::from_value(txs.clone())
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
    }

    pub async fn get_validators(&self) -> Result<Vec<String>> {
        let resp: serde_json::Value = self.request("GET", "/api/peer/trainers", None).await?;

        let trainers = resp
            .get("trainers")
            .ok_or_else(|| BlockchainError::InvalidResponse("missing trainers".into()))?;

        serde_json::from_value(trainers.clone())
            .map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
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
        init.with_method(if method == "GET" {
            Method::Get
        } else {
            Method::Post
        });

        let mut headers = worker::Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;
        init.with_headers(headers);

        if let Some(json) = body {
            init.with_body(Some(
                serde_json::to_string(json)
                    .map_err(BlockchainError::Serialization)?
                    .into(),
            ));
        }

        let request = Request::new_with_init(&url, &init)
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        let status = response.status_code();
        if !(200..300).contains(&status) {
            return Err(BlockchainError::InvalidResponse(format!("HTTP {}", status)));
        }

        let text = response
            .text()
            .await
            .map_err(|e| BlockchainError::HttpRequestWasm(e.to_string()))?;

        serde_json::from_str(&text).map_err(|e| BlockchainError::InvalidResponse(e.to_string()))
    }
}
