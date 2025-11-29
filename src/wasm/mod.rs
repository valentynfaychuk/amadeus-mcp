mod mint;

use crate::blockchain::*;
use crate::BlockchainClient;
use serde_json::{json, Value};
use worker::*;

#[event(fetch)]
async fn main(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let blockchain_url = env
        .var("BLOCKCHAIN_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "https://nodes.amadeus.bot".to_string());

    let client = BlockchainClient::new(blockchain_url)
        .map_err(|e| format!("failed to create client: {}", e))?;

    if req.method() == Method::Post {
        let body: Value = req.json().await?;
        Response::from_json(&handle_mcp_request(&client, &env, body).await)
    } else {
        Response::from_json(&json!({
            "name": "amadeus-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": ["tools"]
        }))
    }
}

async fn handle_mcp_request(client: &BlockchainClient, env: &Env, request: Value) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request.get("id").cloned();

    let result: std::result::Result<Value, Value> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "amadeus-mcp", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(tools_list()),
        "tools/call" => handle_tool_call(client, env, &request["params"]).await,
        _ => Err(err("unknown method")),
    };

    match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
    }
}

async fn handle_tool_call(client: &BlockchainClient, env: &Env, params: &Value) -> std::result::Result<Value, Value> {
    let tool = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    match tool {
        "create_transfer" => {
            let req: TransferRequest = serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            client.create_transfer_blob(req).await
                .map(|b| ok(&json!({ "blob": b.blob, "signing_payload": b.signing_payload, "transaction_hash": b.transaction_hash, "status": "unsigned" })))
                .map_err(|e| err(&e.to_string()))
        }
        "submit_transaction" => {
            let tx: SignedTransaction = serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            client.submit_signed_transaction(tx).await.map(|r| ok(&r)).map_err(|e| err(&e.to_string()))
        }
        "get_account_balance" => {
            let addr = args["address"].as_str().ok_or_else(|| err("missing address"))?;
            client.get_account_balance(addr).await.map(|b| ok(&b)).map_err(|e| err(&e.to_string()))
        }
        "get_chain_stats" => {
            client.get_chain_stats().await.map(|s| ok(&s)).map_err(|e| err(&e.to_string()))
        }
        "get_block_by_height" => {
            let height = args["height"].as_u64().ok_or_else(|| err("missing height"))?;
            client.get_block_by_height(height).await.map(|e| ok(&e)).map_err(|e| err(&e.to_string()))
        }
        "get_transaction" => {
            let hash = args["tx_hash"].as_str().ok_or_else(|| err("missing tx_hash"))?;
            client.get_transaction(hash).await.map(|t| ok(&t)).map_err(|e| err(&e.to_string()))
        }
        "get_transaction_history" => {
            let addr = args["address"].as_str().ok_or_else(|| err("missing address"))?;
            let limit = args["limit"].as_u64().map(|v| v as u32);
            let offset = args["offset"].as_u64().map(|v| v as u32);
            let sort = args["sort"].as_str();
            client.get_transaction_history(addr, limit, offset, sort).await.map(|t| ok(&t)).map_err(|e| err(&e.to_string()))
        }
        "get_validators" => {
            client.get_validators().await
                .map(|v| ok(&json!({ "validators": v, "count": v.len() })))
                .map_err(|e| err(&e.to_string()))
        }
        "get_contract_state" => {
            let addr = args["contract_address"].as_str().ok_or_else(|| err("missing contract_address"))?;
            let key = args["key"].as_str().ok_or_else(|| err("missing key"))?;
            client.get_contract_state(addr, key).await
                .map(|s| ok(&json!({ "contract_address": addr, "key": key, "value": s })))
                .map_err(|e| err(&e.to_string()))
        }
        "faucet_create_order" => faucet_create_order(env, args).await,
        "faucet_complete_order" => faucet_complete_order(client, env, args).await,
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
        tool("faucet_create_order", "Creates a faucet order by sending OTP to phone number for verification",
            json!({ "phone": str_prop() }), vec!["phone"]),
        tool("faucet_complete_order", "Completes a faucet order by verifying OTP and minting testnet tokens",
            json!({ "phone": str_prop(), "otp": str_prop(), "address": str_prop() }), vec!["phone", "otp", "address"]),
    ]})
}

fn tool(name: &str, desc: &str, props: Value, required: Vec<&str>) -> Value {
    json!({ "name": name, "description": desc, "inputSchema": { "type": "object", "properties": props, "required": required }})
}

fn str_prop() -> Value { json!({ "type": "string" }) }
fn err(msg: &str) -> Value { json!({ "code": -32603, "message": msg }) }
fn ok<T: serde::Serialize>(data: &T) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(data).unwrap() }] })
}

async fn faucet_create_order(env: &Env, args: &Value) -> std::result::Result<Value, Value> {
    let phone = args["phone"].as_str().ok_or_else(|| err("missing phone"))?;

    // Check if phone already claimed faucet
    let db = env.d1("MCP_DATABASE").map_err(|e| err(&e.to_string()))?;
    let existing = db.prepare("SELECT 1 FROM faucet_orders WHERE phone = ?1")
        .bind(&[phone.into()]).map_err(|e| err(&e.to_string()))?
        .first::<i32>(None).await.map_err(|e| err(&e.to_string()))?;
    if existing.is_some() { return Err(err("phone already used faucet")); }

    // Send OTP via Twilio Verify
    twilio_verify_send(env, phone).await?;
    Ok(ok(&json!({ "status": "success", "message": "OTP sent" })))
}

async fn faucet_complete_order(_client: &BlockchainClient, env: &Env, args: &Value) -> std::result::Result<Value, Value> {
    let phone = args["phone"].as_str().ok_or_else(|| err("missing phone"))?;
    let code = args["otp"].as_str().ok_or_else(|| err("missing otp"))?;
    let address = args["address"].as_str().ok_or_else(|| err("missing address"))?;

    // Check if phone already claimed
    let db = env.d1("MCP_DATABASE").map_err(|e| err(&e.to_string()))?;
    let existing = db.prepare("SELECT 1 FROM faucet_orders WHERE phone = ?1")
        .bind(&[phone.into()]).map_err(|e| err(&e.to_string()))?
        .first::<i32>(None).await.map_err(|e| err(&e.to_string()))?;
    if existing.is_some() { return Err(err("phone already used faucet")); }

    // Verify OTP via Twilio Verify
    let status = twilio_verify_check(env, phone, code).await?;
    if status != "approved" { return Err(err("invalid or expired OTP")); }

    // Mint tokens
    let tx_hash = mint::mint_tokens(env, address).await?;

    // Log successful claim
    db.prepare("INSERT INTO faucet_orders (phone, address) VALUES (?1, ?2)")
        .bind(&[phone.into(), address.into()]).map_err(|e| err(&e.to_string()))?
        .run().await.map_err(|e| err(&e.to_string()))?;

    Ok(ok(&json!({ "status": "success", "message": "tokens minted", "tx_hash": tx_hash })))
}

fn twilio_auth(env: &Env) -> std::result::Result<(String, String), Value> {
    let api_key = env.var("TWILIO_API_KEY_SID").map(|v| v.to_string()).map_err(|_| err("TWILIO_API_KEY_SID not set"))?;
    let api_secret = env.var("TWILIO_API_KEY_SECRET").map(|v| v.to_string()).map_err(|_| err("TWILIO_API_KEY_SECRET not set"))?;
    let service_sid = env.var("TWILIO_VERIFY_SERVICE_SID").map(|v| v.to_string()).map_err(|_| err("TWILIO_VERIFY_SERVICE_SID not set"))?;
    Ok((base64_encode(&format!("{}:{}", api_key, api_secret)), service_sid))
}

async fn twilio_verify_send(env: &Env, to: &str) -> std::result::Result<(), Value> {
    let (auth, service_sid) = twilio_auth(env)?;
    let url = format!("https://verify.twilio.com/v2/Services/{}/Verifications", service_sid);

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Basic {}", auth)).map_err(|e| err(&e.to_string()))?;
    headers.set("Content-Type", "application/x-www-form-urlencoded").map_err(|e| err(&e.to_string()))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers)
        .with_body(Some(format!("To={}&Channel=sms", urlenc(to)).into()));

    let req = Request::new_with_init(&url, &init).map_err(|e| err(&e.to_string()))?;
    let resp = Fetch::Request(req).send().await.map_err(|e| err(&e.to_string()))?;
    if resp.status_code() >= 400 { return Err(err("failed to send verification")); }
    Ok(())
}

async fn twilio_verify_check(env: &Env, to: &str, code: &str) -> std::result::Result<String, Value> {
    let (auth, service_sid) = twilio_auth(env)?;
    let url = format!("https://verify.twilio.com/v2/Services/{}/VerificationChecks", service_sid);

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Basic {}", auth)).map_err(|e| err(&e.to_string()))?;
    headers.set("Content-Type", "application/x-www-form-urlencoded").map_err(|e| err(&e.to_string()))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers)
        .with_body(Some(format!("To={}&Code={}", urlenc(to), urlenc(code)).into()));

    let req = Request::new_with_init(&url, &init).map_err(|e| err(&e.to_string()))?;
    let mut resp = Fetch::Request(req).send().await.map_err(|e| err(&e.to_string()))?;
    if resp.status_code() >= 400 { return Err(err("verification check failed")); }

    let body: Value = resp.json().await.map_err(|e| err(&e.to_string()))?;
    Ok(body["status"].as_str().unwrap_or("").to_string())
}

fn base64_encode(s: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk.get(0).copied().unwrap_or(0), chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        out.push(CHARS[(b[0] >> 2) as usize] as char);
        out.push(CHARS[((b[0] & 3) << 4 | b[1] >> 4) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((b[1] & 15) << 2 | b[2] >> 6) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(b[2] & 63) as usize] as char } else { '=' });
    }
    out
}

fn urlenc(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}
