use super::tx;
use serde_json::{json, Value};
use worker::Env;

const FAUCET_AMOUNT: i128 = 100_000_000_000;
const FAUCET_SYMBOL: &str = "AMA";

pub async fn transfer(env: &Env, address: &str) -> Result<String, Value> {
    let rpc = env
        .var("AMADEUS_TESTNET_RPC")
        .map(|v| v.to_string())
        .map_err(|_| err("AMADEUS_TESTNET_RPC not configured"))?;
    let key_b58 = env
        .var("AMADEUS_TESTNET_SK")
        .map(|v| v.to_string())
        .map_err(|_| err("AMADEUS_TESTNET_SK not configured"))?;

    let sk = bs58::decode(&key_b58)
        .into_vec()
        .map_err(|_| err("invalid mint key encoding"))?;
    let receiver = bs58::decode(address)
        .into_vec()
        .map_err(|_| err("invalid address encoding"))?;

    if receiver.len() != 48 {
        return Err(err("address must be 48 bytes"));
    }

    let built = tx::build_transfer_tx(&sk, &receiver, FAUCET_SYMBOL, FAUCET_AMOUNT).map_err(err)?;
    let tx_b58 = bs58::encode(&built.packed).into_string();
    let tx_hash = bs58::encode(&built.hash).into_string();

    let url = format!("{}/api/tx/submit/{}", rpc.trim_end_matches('/'), tx_b58);
    let mut resp = worker::Fetch::Url(worker::Url::parse(&url).map_err(|e| err(&e.to_string()))?)
        .send()
        .await
        .map_err(|e| err(&e.to_string()))?;

    let body = resp.text().await.map_err(|e| err(&e.to_string()))?;
    Ok(format!(
        "status={} tx_hash={} body={}",
        resp.status_code(),
        tx_hash,
        body
    ))
}

fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
