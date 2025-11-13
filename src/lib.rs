pub mod blockchain;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use blockchain::{BlockchainClient, BlockchainError};

#[cfg(not(target_arch = "wasm32"))]
pub use server::BlockchainMcpServer;
