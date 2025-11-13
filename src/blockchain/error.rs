use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[cfg(target_arch = "wasm32")]
    #[error("HTTP request failed: {0}")]
    HttpRequestWasm(String),

    #[error("Invalid response from blockchain: {0}")]
    InvalidResponse(String),

    #[error("Transaction validation failed: {0}")]
    ValidationFailed(String),

    #[error("Account not found: {address}")]
    AccountNotFound { address: String },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Network error after {attempts} retries")]
    NetworkRetryExhausted { attempts: usize },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, BlockchainError>;
