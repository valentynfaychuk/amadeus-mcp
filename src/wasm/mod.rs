mod mint;
mod tx;

use crate::blockchain::*;
use crate::BlockchainClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use worker::*;

#[event(fetch)]
async fn main(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let blockchain_url = env
        .var("BLOCKCHAIN_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "https://nodes.amadeus.bot".to_string());

    let client = BlockchainClient::new(blockchain_url.clone())
        .map_err(|e| format!("failed to create client: {}", e))?;

    if req.method() == Method::Post {
        let client_ip = req.headers().get("CF-Connecting-IP").ok().flatten();
        let headers: HashMap<String, String> = req.headers().entries().collect();
        let body: Value = req.json().await?;
        Response::from_json(&handle_mcp_request(&client, &env, &blockchain_url, client_ip, headers, body).await)
    } else {
        Response::from_json(&json!({
            "name": "amadeus-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": ["tools"]
        }))
    }
}

async fn handle_mcp_request(
    client: &BlockchainClient, env: &Env, rpc: &str, client_ip: Option<String>,
    headers: HashMap<String, String>, request: Value,
) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request.get("id").cloned();
    let result: std::result::Result<Value, Value> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "amadeus-mcp", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(tools_list()),
        "tools/call" => handle_tool_call(client, env, rpc, client_ip, headers, &request["params"]).await,
        _ => Err(err("unknown method")),
    };

    match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
    }
}

async fn handle_tool_call(
    client: &BlockchainClient, env: &Env, rpc: &str, client_ip: Option<String>,
    headers: HashMap<String, String>, params: &Value,
) -> std::result::Result<Value, Value> {
    let tool = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];
    match tool {
        "create_transfer" => {
            let req: TransferRequest =
                serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            client.create_transfer_blob(req).await
                .map(|b| ok(&json!({ "blob": b.blob, "signing_payload": b.signing_payload, "transaction_hash": b.transaction_hash, "status": "unsigned" })))
                .map_err(|e| err(&e.to_string()))
        }
        "submit_transaction" => {
            let tx: SignedTransaction =
                serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            client
                .submit_signed_transaction(tx)
                .await
                .map(|r| ok(&r))
                .map_err(|e| err(&e.to_string()))
        }
        "get_account_balance" => {
            let addr = args["address"]
                .as_str()
                .ok_or_else(|| err("missing address"))?;
            client
                .get_account_balance(addr)
                .await
                .map(|b| ok(&b))
                .map_err(|e| err(&e.to_string()))
        }
        "get_chain_stats" => client
            .get_chain_stats()
            .await
            .map(|s| ok(&s))
            .map_err(|e| err(&e.to_string())),
        "get_block_by_height" => {
            let height = args["height"]
                .as_u64()
                .ok_or_else(|| err("missing height"))?;
            client
                .get_block_by_height(height)
                .await
                .map(|e| ok(&e))
                .map_err(|e| err(&e.to_string()))
        }
        "get_transaction" => {
            let hash = args["tx_hash"]
                .as_str()
                .ok_or_else(|| err("missing tx_hash"))?;
            client
                .get_transaction(hash)
                .await
                .map(|t| ok(&t))
                .map_err(|e| err(&e.to_string()))
        }
        "get_transaction_history" => {
            let addr = args["address"]
                .as_str()
                .ok_or_else(|| err("missing address"))?;
            let limit = args["limit"].as_u64().map(|v| v as u32);
            let offset = args["offset"].as_u64().map(|v| v as u32);
            let sort = args["sort"].as_str();
            client
                .get_transaction_history(addr, limit, offset, sort)
                .await
                .map(|t| ok(&t))
                .map_err(|e| err(&e.to_string()))
        }
        "get_validators" => client
            .get_validators()
            .await
            .map(|v| ok(&json!({ "validators": v, "count": v.len() })))
            .map_err(|e| err(&e.to_string())),
        "get_contract_state" => {
            let addr = args["contract_address"]
                .as_str()
                .ok_or_else(|| err("missing contract_address"))?;
            let key = args["key"].as_str().ok_or_else(|| err("missing key"))?;
            client
                .get_contract_state(addr, key)
                .await
                .map(|s| ok(&json!({ "contract_address": addr, "key": key, "value": s })))
                .map_err(|e| err(&e.to_string()))
        }
        "claim_testnet_ama" => claim_testnet_ama(env, client_ip, headers, args).await,
        "get_entry_tip" => fetch_json(&format!("{rpc}/api/chain/tip")).await,
        "get_entry_by_hash" => {
            let h = args["hash"].as_str().ok_or_else(|| err("missing hash"))?;
            fetch_json(&format!("{rpc}/api/chain/hash/{h}")).await
        }
        "get_block_with_txs" => {
            let h = args["height"].as_u64().ok_or_else(|| err("missing height"))?;
            fetch_json(&format!("{rpc}/api/chain/height_with_txs/{h}")).await
        }
        "get_txs_in_entry" => {
            let h = args["entry_hash"].as_str().ok_or_else(|| err("missing entry_hash"))?;
            fetch_json(&format!("{rpc}/api/chain/txs_in_entry/{h}")).await
        }
        "get_epoch_score" => {
            let url = match args["address"].as_str() {
                Some(pk) => format!("{rpc}/api/epoch/score/{pk}"),
                None => format!("{rpc}/api/epoch/score"),
            };
            fetch_json(&url).await
        }
        "get_emission_address" => {
            let pk = args["address"].as_str().ok_or_else(|| err("missing address"))?;
            fetch_json(&format!("{rpc}/api/epoch/get_emission_address/{pk}")).await
        }
        "get_richlist" => fetch_json(&format!("{rpc}/api/contract/richlist")).await,
        "get_nodes" => fetch_json(&format!("{rpc}/api/peer/nodes")).await,
        "get_removed_validators" => fetch_json(&format!("{rpc}/api/peer/removed_trainers")).await,
        _ => Err(err("unknown tool")),
    }
}

fn tools_list() -> Value {
    json!({ "tools": [
        tool("create_transfer", "Creates an unsigned transaction blob for transferring assets between accounts",
            json!({ "symbol": str_prop(), "source": str_prop(), "destination": str_prop(), "amount": str_prop(), "memo": str_prop() }),
            vec!["symbol", "source", "destination", "amount"]),
        tool("submit_transaction", "Submits a signed transaction to the blockchain network",
            json!({ "transaction": str_prop(), "signature": str_prop() }), vec!["transaction", "signature"]),
        tool("get_account_balance", "Queries the balance of an account across all supported assets",
            json!({ "address": str_prop() }), vec!["address"]),
        tool("get_chain_stats", "Retrieves current blockchain statistics", json!({}), vec![]),
        tool("get_block_by_height", "Retrieves blockchain entries at a specific height",
            json!({ "height": { "type": "number" } }), vec!["height"]),
        tool("get_transaction", "Retrieves a specific transaction by its hash",
            json!({ "tx_hash": str_prop() }), vec!["tx_hash"]),
        tool("get_transaction_history", "Retrieves transaction history for a specific account",
            json!({ "address": str_prop(), "limit": { "type": "number" }, "offset": { "type": "number" }, "sort": str_prop() }), vec!["address"]),
        tool("get_validators", "Retrieves the list of current validator nodes", json!({}), vec![]),
        tool("get_contract_state", "Retrieves a specific value from smart contract storage",
            json!({ "contract_address": str_prop(), "key": str_prop() }), vec!["contract_address", "key"]),
        tool("claim_testnet_ama", "Claims testnet AMA tokens to the specified address (once per 24 hours per IP)",
            json!({ "address": str_prop() }), vec!["address"]),
        tool("get_entry_tip", "Get the latest blockchain entry", json!({}), vec![]),
        tool("get_entry_by_hash", "Get entry by hash", json!({ "hash": str_prop() }), vec!["hash"]),
        tool("get_block_with_txs", "Get block at height with full transactions", json!({ "height": { "type": "number" } }), vec!["height"]),
        tool("get_txs_in_entry", "Get all transactions in an entry", json!({ "entry_hash": str_prop() }), vec!["entry_hash"]),
        tool("get_epoch_score", "Get validator mining scores (optionally for specific address)", json!({ "address": str_prop() }), vec![]),
        tool("get_emission_address", "Get emission address for a validator", json!({ "address": str_prop() }), vec!["address"]),
        tool("get_richlist", "Get top AMA token holders", json!({}), vec![]),
        tool("get_nodes", "Get connected peer nodes", json!({}), vec![]),
        tool("get_removed_validators", "Get validators removed this epoch", json!({}), vec![]),
    ]})
}

fn tool(name: &str, desc: &str, props: Value, required: Vec<&str>) -> Value {
    json!({ "name": name, "description": desc, "inputSchema": { "type": "object", "properties": props, "required": required }})
}

fn str_prop() -> Value {
    json!({ "type": "string" })
}
fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
fn ok<T: serde::Serialize>(data: &T) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(data).unwrap() }] })
}

async fn fetch_json(url: &str) -> std::result::Result<Value, Value> {
    let mut resp = worker::Fetch::Url(worker::Url::parse(url).map_err(|e| err(&e.to_string()))?)
        .send().await.map_err(|e| err(&e.to_string()))?;
    let json: Value = serde_json::from_str(&resp.text().await.map_err(|e| err(&e.to_string()))?)
        .map_err(|e| err(&e.to_string()))?;
    Ok(ok(&json))
}

const CLAIM_COOLDOWN_SECS: f64 = 86400.0;

async fn claim_testnet_ama(
    env: &Env,
    client_ip: Option<String>,
    headers: HashMap<String, String>,
    args: &Value,
) -> std::result::Result<Value, Value> {
    let ip = client_ip.ok_or_else(|| err("could not determine client IP"))?;
    let address = args["address"]
        .as_str()
        .ok_or_else(|| err("missing address"))?;
    let now = (Date::now().as_millis() / 1000) as f64;

    let db = env.d1("MCP_DATABASE").map_err(|e| err(&e.to_string()))?;

    let request_dump = serde_json::to_string(&headers).unwrap_or_default();
    let ts = Date::now().as_millis().to_string();
    let _ = db
        .prepare("INSERT INTO faucet_request_dumps (timestamp, request) VALUES (?1, ?2)")
        .bind(&[ts.into(), request_dump.into()])
        .map_err(|e| err(&e.to_string()))?
        .run()
        .await;
    let existing: Option<f64> = db
        .prepare("SELECT claimed_at FROM faucet_claims WHERE ip = ?1")
        .bind(&[ip.clone().into()])
        .map_err(|e| err(&e.to_string()))?
        .first(Some("claimed_at"))
        .await
        .map_err(|e| err(&e.to_string()))?;

    if let Some(claimed_at) = existing {
        let elapsed = now - claimed_at;
        if elapsed < CLAIM_COOLDOWN_SECS {
            let remaining = (CLAIM_COOLDOWN_SECS - elapsed) as i64;
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            return Err(err(&format!(
                "can only claim once per day, wait {}h {}m",
                hours, minutes
            )));
        }
    }

    let tx_hash = mint::transfer(env, address).await?;

    if existing.is_some() {
        db.prepare("UPDATE faucet_claims SET claimed_at = ?1, address = ?2 WHERE ip = ?3")
            .bind(&[now.into(), address.into(), ip.into()])
            .map_err(|e| err(&e.to_string()))?
            .run()
            .await
            .map_err(|e| err(&e.to_string()))?;
    } else {
        db.prepare("INSERT INTO faucet_claims (ip, address, claimed_at) VALUES (?1, ?2, ?3)")
            .bind(&[ip.into(), address.into(), now.into()])
            .map_err(|e| err(&e.to_string()))?
            .run()
            .await
            .map_err(|e| err(&e.to_string()))?;
    }

    Ok(ok(&json!({ "status": "success", "tx_hash": tx_hash })))
}
