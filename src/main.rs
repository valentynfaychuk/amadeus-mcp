use amadeus_mcp::{BlockchainClient, BlockchainMcpServer};
use rmcp::ServiceExt;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,amadeus_mcp=debug")),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    let blockchain_url = env::var("BLOCKCHAIN_URL")
        .unwrap_or_else(|_| "https://nodes.amadeus.bot".to_string());
    let api_key = env::var("BLOCKCHAIN_API_KEY").ok();

    info!(
        url = %blockchain_url,
        has_api_key = api_key.is_some(),
        "initializing blockchain client"
    );

    let client = BlockchainClient::new(blockchain_url, api_key)?;
    let server = BlockchainMcpServer::new(client);

    info!("starting MCP server on stdio transport");

    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("failed to initialize server: {}", e))?;

    info!("server initialized successfully");

    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("server error: {}", e))?;

    info!("server shutdown complete");

    Ok(())
}
