use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod args_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(args: &[Vec<u8>], ser: S) -> Result<S::Ok, S::Error> {
        let v: Vec<serde_bytes::ByteBuf> = args
            .iter()
            .map(|a| serde_bytes::ByteBuf::from(a.clone()))
            .collect();
        v.serialize(ser)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<Vec<u8>>, D::Error> {
        let v: Vec<serde_bytes::ByteBuf> = Deserialize::deserialize(de)?;
        Ok(v.into_iter().map(|b| b.into_vec()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAction {
    #[serde(with = "args_serde")]
    pub args: Vec<Vec<u8>>,
    pub contract: String,
    pub function: String,
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_symbol: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_amount: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub action: TxAction,
    pub nonce: i128,
    #[serde(with = "serde_bytes")]
    pub signer: Vec<u8>,
}

pub struct UnsignedTx {
    pub tx_blob: Vec<u8>,
    pub signing_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxU {
    #[serde(with = "serde_bytes")]
    hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    signature: Vec<u8>,
    tx: Tx,
}

pub struct FinalizedTx {
    pub packed: Vec<u8>,
    pub hash: [u8; 32],
}

pub fn finalize_transaction(tx_blob_b58: &str, signature_b58: &str) -> Result<FinalizedTx, &'static str> {
    let tx_encoded = bs58::decode(tx_blob_b58).into_vec().map_err(|_| "invalid blob base58")?;
    let signature = bs58::decode(signature_b58).into_vec().map_err(|_| "invalid signature base58")?;
    let tx: Tx = vecpak::from_slice(&tx_encoded).map_err(|_| "failed to decode tx")?;
    let hash: [u8; 32] = Sha256::digest(&tx_encoded).into();

    let txu = TxU {
        hash: hash.to_vec(),
        signature,
        tx,
    };
    let packed = vecpak::to_vec(&txu).map_err(|_| "failed to encode txu")?;
    Ok(FinalizedTx { packed, hash })
}

pub fn build_unsigned(
    signer_pk: &[u8],
    contract: &str,
    function: &str,
    args: &[Vec<u8>],
    attached_symbol: Option<&[u8]>,
    attached_amount: Option<&[u8]>,
    nonce: Option<i64>,
) -> Result<UnsignedTx, &'static str> {
    let nonce_val = nonce.map(|n| n as i128).unwrap_or_else(|| {
        #[cfg(target_arch = "wasm32")]
        { js_sys::Date::now() as i128 * 1_000_000 }
        #[cfg(not(target_arch = "wasm32"))]
        { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i128 }
    });

    let action = TxAction {
        op: "call".to_string(),
        contract: contract.to_string(),
        function: function.to_string(),
        args: args.to_vec(),
        attached_symbol: attached_symbol.map(|s| s.to_vec()),
        attached_amount: attached_amount.map(|a| a.to_vec()),
    };

    let tx = Tx {
        signer: signer_pk.to_vec(),
        nonce: nonce_val,
        action,
    };

    let tx_encoded = vecpak::to_vec(&tx).map_err(|_| "failed to encode tx")?;
    let hash: [u8; 32] = Sha256::digest(&tx_encoded).into();

    Ok(UnsignedTx {
        tx_blob: tx_encoded,
        signing_hash: hash,
    })
}

#[cfg(target_arch = "wasm32")]
pub struct BuiltTx {
    pub packed: Vec<u8>,
    pub hash: [u8; 32],
}

#[cfg(target_arch = "wasm32")]
pub fn build_transfer_tx(
    sk_bytes: &[u8],
    receiver: &[u8],
    symbol: &str,
    amount: i128,
) -> Result<BuiltTx, &'static str> {
    use bls12_381::Scalar;
    use group::Curve;

    if sk_bytes.len() != 64 {
        return Err("secret key must be 64 bytes");
    }
    let bytes_64: [u8; 64] = sk_bytes.try_into().map_err(|_| "invalid sk length")?;
    let sk_scalar = Scalar::from_bytes_wide(&bytes_64);
    let pk = (bls12_381::G1Projective::generator() * sk_scalar).to_affine().to_compressed().to_vec();

    let nonce = js_sys::Date::now() as i128 * 1_000_000;
    let action = TxAction {
        op: "call".to_string(),
        contract: "Coin".to_string(),
        function: "transfer".to_string(),
        args: vec![receiver.to_vec(), amount.to_string().as_bytes().to_vec(), symbol.as_bytes().to_vec()],
        attached_symbol: None,
        attached_amount: None,
    };

    let tx = Tx { signer: pk.clone(), nonce, action };
    let tx_encoded = vecpak::to_vec(&tx).map_err(|_| "failed to encode tx")?;
    let hash: [u8; 32] = Sha256::digest(&tx_encoded).into();

    let mut sk_be = sk_scalar.to_bytes();
    sk_be.reverse();
    let sk = blst::min_pk::SecretKey::from_bytes(&sk_be).map_err(|_| "invalid secret key")?;
    let signature = sk.sign(&hash, b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_TX_", &[]).to_bytes().to_vec();

    let txu = TxU { hash: hash.to_vec(), signature, tx };
    let packed = vecpak::to_vec(&txu).map_err(|_| "failed to encode txu")?;
    Ok(BuiltTx { packed, hash })
}
