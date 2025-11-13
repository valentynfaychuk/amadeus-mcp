#[cfg(not(target_arch = "wasm32"))]
pub mod client;
#[cfg(target_arch = "wasm32")]
pub mod client_wasm;

pub mod error;
pub mod types;

#[cfg(not(target_arch = "wasm32"))]
pub use client::BlockchainClient;
#[cfg(target_arch = "wasm32")]
pub use client_wasm::BlockchainClient;

pub use error::BlockchainError;
pub use types::*;
