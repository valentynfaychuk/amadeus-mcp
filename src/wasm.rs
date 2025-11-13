use crate::BlockchainClient;
use crate::blockchain::*;
use serde_json::{json, Value};
use worker::*;

#[event(fetch)]
async fn main(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let blockchain_url = env
        .var("BLOCKCHAIN_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "https://nodes.amadeus.bot".to_string());

    let api_key = env.var("BLOCKCHAIN_API_KEY").ok().map(|v| v.to_string());

    let client = BlockchainClient::new(blockchain_url, api_key)
        .map_err(|e| format!("failed to create client: {}", e))?;

    if req.method() == Method::Post {
        let body: Value = req.json().await?;
        let response = handle_mcp_request(&client, body).await;
        Response::from_json(&response)
    } else {
        Response::from_json(&json!({
            "name": "amadeus-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": ["tools"]
        }))
    }
}

async fn handle_mcp_request(client: &BlockchainClient, request: Value) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request.get("id").cloned();

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "amadeus-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "create_transfer",
                    "description": "Creates an unsigned transaction blob for transferring assets between accounts",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "symbol": { "type": "string" },
                            "source": { "type": "string" },
                            "destination": { "type": "string" },
                            "amount": { "type": "string" },
                            "memo": { "type": "string" }
                        },
                        "required": ["symbol", "source", "destination", "amount"]
                    }
                },
                {
                    "name": "submit_transaction",
                    "description": "Submits a signed transaction to the blockchain network",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "transaction": { "type": "string" },
                            "signature": { "type": "string" }
                        },
                        "required": ["transaction", "signature"]
                    }
                },
                {
                    "name": "get_account_balance",
                    "description": "Queries the balance of an account across all supported assets",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "address": { "type": "string" }
                        },
                        "required": ["address"]
                    }
                },
                {
                    "name": "get_chain_stats",
                    "description": "Retrieves current blockchain statistics",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_block_by_height",
                    "description": "Retrieves blockchain entries at a specific height",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "height": { "type": "number" }
                        },
                        "required": ["height"]
                    }
                },
                {
                    "name": "get_transaction",
                    "description": "Retrieves a specific transaction by its hash",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "tx_hash": { "type": "string" }
                        },
                        "required": ["tx_hash"]
                    }
                },
                {
                    "name": "get_transaction_history",
                    "description": "Retrieves transaction history for a specific account",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "address": { "type": "string" },
                            "limit": { "type": "number" },
                            "offset": { "type": "number" },
                            "sort": { "type": "string" }
                        },
                        "required": ["address"]
                    }
                },
                {
                    "name": "get_validators",
                    "description": "Retrieves the list of current validator nodes",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_contract_state",
                    "description": "Retrieves a specific value from smart contract storage",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "contract_address": { "type": "string" },
                            "key": { "type": "string" }
                        },
                        "required": ["contract_address", "key"]
                    }
                }
            ]
        })),
        "tools/call" => {
            let tool_name = request["params"]["name"].as_str().unwrap_or("");
            let arguments = &request["params"]["arguments"];

            match tool_name {
                "create_transfer" => {
                    match serde_json::from_value::<TransferRequest>(arguments.clone()) {
                        Ok(req) => match client.create_transfer_blob(req).await {
                            Ok(blob) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&json!({
                                        "blob": blob.blob,
                                        "signing_payload": blob.signing_payload,
                                        "transaction_hash": blob.transaction_hash,
                                        "status": "unsigned"
                                    })).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        Err(e) => Err(error_response(&format!("invalid arguments: {}", e)))
                    }
                }
                "submit_transaction" => {
                    match serde_json::from_value::<SignedTransaction>(arguments.clone()) {
                        Ok(tx) => match client.submit_signed_transaction(tx).await {
                            Ok(resp) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&resp).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        Err(e) => Err(error_response(&format!("invalid arguments: {}", e)))
                    }
                }
                "get_account_balance" => {
                    match arguments["address"].as_str() {
                        Some(address) => match client.get_account_balance(address).await {
                            Ok(balance) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&balance).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        None => Err(error_response("missing address parameter"))
                    }
                }
                "get_chain_stats" => {
                    match client.get_chain_stats().await {
                        Ok(stats) => Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&stats).unwrap()
                            }]
                        })),
                        Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                    }
                }
                "get_block_by_height" => {
                    match arguments["height"].as_u64() {
                        Some(height) => match client.get_block_by_height(height).await {
                            Ok(entries) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&entries).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        None => Err(error_response("missing or invalid height parameter"))
                    }
                }
                "get_transaction" => {
                    match arguments["tx_hash"].as_str() {
                        Some(hash) => match client.get_transaction(hash).await {
                            Ok(tx) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&tx).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        None => Err(error_response("missing tx_hash parameter"))
                    }
                }
                "get_transaction_history" => {
                    match arguments["address"].as_str() {
                        Some(address) => {
                            let limit = arguments["limit"].as_u64().map(|v| v as u32);
                            let offset = arguments["offset"].as_u64().map(|v| v as u32);
                            let sort = arguments["sort"].as_str();

                            match client.get_transaction_history(address, limit, offset, sort).await {
                                Ok(txs) => Ok(json!({
                                    "content": [{
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&txs).unwrap()
                                    }]
                                })),
                                Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                            }
                        },
                        None => Err(error_response("missing address parameter"))
                    }
                }
                "get_validators" => {
                    match client.get_validators().await {
                        Ok(validators) => Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&json!({
                                    "validators": validators,
                                    "count": validators.len()
                                })).unwrap()
                            }]
                        })),
                        Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                    }
                }
                "get_contract_state" => {
                    match (arguments["contract_address"].as_str(), arguments["key"].as_str()) {
                        (Some(addr), Some(key)) => match client.get_contract_state(addr, key).await {
                            Ok(state) => Ok(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&json!({
                                        "contract_address": addr,
                                        "key": key,
                                        "value": state
                                    })).unwrap()
                                }]
                            })),
                            Err(e) => Err(error_response(&format!("blockchain error: {}", e)))
                        },
                        _ => Err(error_response("missing contract_address or key parameter"))
                    }
                }
                _ => Err(error_response("unknown tool"))
            }
        }
        _ => Err(error_response("unknown method"))
    };

    match result {
        Ok(r) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": r
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": e
        })
    }
}

fn error_response(message: &str) -> Value {
    json!({
        "code": -32603,
        "message": message
    })
}
