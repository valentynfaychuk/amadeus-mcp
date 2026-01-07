use crate::blockchain::{
    AccountQuery, BlockchainClient, BlockchainError, ContractStateQuery, HeightQuery,
    SignedTransaction, TransactionHistoryQuery, TransactionQuery, TransactionRequest,
};
use rmcp::{
    handler::server::tool::{Parameters, ToolRouter},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, Json, RoleServer, ServerHandler,
};
use std::{future::Future, sync::Arc};
use tracing::error;
use validator::Validate;

#[derive(Clone)]
pub struct BlockchainMcpServer {
    blockchain: Arc<BlockchainClient>,
    mainnet_url: String,
    testnet_url: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BlockchainMcpServer {
    pub fn new(blockchain: BlockchainClient, mainnet_url: String, testnet_url: String) -> Self {
        Self {
            blockchain: Arc::new(blockchain),
            mainnet_url,
            testnet_url,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "create_transaction",
        description = "Creates an unsigned transaction for any contract call. Takes signer public key, contract name, function name, and arguments. Returns transaction blob that only needs signing."
    )]
    async fn create_transaction(
        &self,
        params: Parameters<TransactionRequest>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let req = params.0;
        req.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let blob = self
            .blockchain
            .create_transaction_blob(req)
            .await
            .map_err(|e| Self::blockchain_error("create_transaction", e))?;

        Ok(Json(serde_json::json!({
            "blob": blob.blob,
            "signing_payload": blob.signing_payload,
            "transaction_hash": blob.transaction_hash,
            "status": "unsigned",
            "next_step": "Sign the signing_payload with BLS12-381 and call submit_transaction"
        })))
    }

    #[tool(
        name = "submit_transaction",
        description = "Submits a signed transaction to the blockchain network. Requires the transaction blob and signature from the signing process. Optional network parameter: 'mainnet' (default) or 'testnet'."
    )]
    async fn submit_transaction(
        &self,
        params: Parameters<SignedTransaction>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let tx = params.0;
        tx.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let url = match tx.network.as_deref() {
            Some("testnet") => &self.testnet_url,
            _ => &self.mainnet_url,
        };

        let response = self
            .blockchain
            .submit_signed_transaction(tx, url)
            .await
            .map_err(|e| Self::blockchain_error("submit_transaction", e))?;

        if response.error == "ok" {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Transaction submitted successfully",
                "tx_hash": response.tx_hash
            })))
        } else {
            Err(McpError::internal_error(
                "submission_failed",
                Some(serde_json::json!({ "error": response.error })),
            ))
        }
    }

    #[tool(
        name = "get_account_balance",
        description = "Queries the balance of an account across all supported assets."
    )]
    async fn get_account_balance(
        &self,
        params: Parameters<AccountQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let balance = self
            .blockchain
            .get_account_balance(&query.address)
            .await
            .map_err(|e| Self::blockchain_error("get_account_balance", e))?;

        Self::to_json(balance)
    }

    #[tool(
        name = "get_chain_stats",
        description = "Retrieves current blockchain statistics including height, total transactions, and total accounts."
    )]
    async fn get_chain_stats(&self) -> Result<Json<serde_json::Value>, McpError> {
        let stats = self
            .blockchain
            .get_chain_stats()
            .await
            .map_err(|e| Self::blockchain_error("get_chain_stats", e))?;

        Self::to_json(stats)
    }

    #[tool(
        name = "get_block_by_height",
        description = "Retrieves blockchain entries at a specific height. Returns all entries for that height."
    )]
    async fn get_block_by_height(
        &self,
        params: Parameters<HeightQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let entries = self
            .blockchain
            .get_block_by_height(query.height)
            .await
            .map_err(|e| Self::blockchain_error("get_block_by_height", e))?;

        Self::to_json(entries)
    }

    #[tool(
        name = "get_transaction",
        description = "Retrieves a specific transaction by its hash. Returns detailed transaction information."
    )]
    async fn get_transaction(
        &self,
        params: Parameters<TransactionQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let transaction = self
            .blockchain
            .get_transaction(&query.tx_hash)
            .await
            .map_err(|e| Self::blockchain_error("get_transaction", e))?;

        Self::to_json(transaction)
    }

    #[tool(
        name = "get_transaction_history",
        description = "Retrieves transaction history for a specific account. Supports pagination with limit, offset, and sort parameters."
    )]
    async fn get_transaction_history(
        &self,
        params: Parameters<TransactionHistoryQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let transactions = self
            .blockchain
            .get_transaction_history(
                &query.address,
                query.limit,
                query.offset,
                query.sort.as_deref(),
            )
            .await
            .map_err(|e| Self::blockchain_error("get_transaction_history", e))?;

        Self::to_json(transactions)
    }

    #[tool(
        name = "get_validators",
        description = "Retrieves the list of current validator nodes (trainers) in the network."
    )]
    async fn get_validators(&self) -> Result<Json<serde_json::Value>, McpError> {
        let validators = self
            .blockchain
            .get_validators()
            .await
            .map_err(|e| Self::blockchain_error("get_validators", e))?;

        Ok(Json(serde_json::json!({
            "validators": validators,
            "count": validators.len()
        })))
    }

    #[tool(
        name = "get_contract_state",
        description = "Retrieves a specific value from smart contract storage by contract address and key."
    )]
    async fn get_contract_state(
        &self,
        params: Parameters<ContractStateQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let state = self
            .blockchain
            .get_contract_state(&query.contract_address, &query.key)
            .await
            .map_err(|e| Self::blockchain_error("get_contract_state", e))?;

        Ok(Json(serde_json::json!({
            "contract_address": query.contract_address,
            "key": query.key,
            "value": state
        })))
    }

    #[tool(
        name = "get_amadeus_docs",
        description = "Returns comprehensive documentation about the Amadeus blockchain, including overview, key concepts, RPC API endpoints, wallet operations, and ecosystem information."
    )]
    async fn get_amadeus_docs(&self) -> Result<Json<serde_json::Value>, McpError> {
        Ok(Json(serde_json::json!({
            "overview": {
                "title": "Amadeus Blockchain",
                "description": "Amadeus is a high-performance Layer 1 blockchain purpose-built to create, deploy, and monetize AI agents. It combines verifiable agent generation (no code required) through Nova AI with real compute-mining via Useful Proof of Work (uPoW), making it the first blockchain where AI agents evolve transparently, execute deterministically, and serve real users — all on-chain.",
                "key_features": [
                    "Verifiable AI agent generation via Nova AI Compiler",
                    "Useful Proof of Work (uPoW) - real Tensorcore matrix computations for AI workloads",
                    "WASM-based smart contract runtime",
                    "BLS-based consensus and block finalization",
                    "Sub-second transaction confirmation (1-3 seconds typical)",
                    "AI Agent Oracle Streams for data integration"
                ],
                "links": {
                    "explorer": "https://explorer.ama.one/",
                    "wallet": "https://wallet.ama.one/",
                    "docs": "https://docs.ama.one/",
                    "github": "https://github.com/amadeus-robot/node"
                }
            },
            "useful_proof_of_work": {
                "description": "Instead of wasting energy on traditional hashing, Amadeus uses Useful Proof of Work (uPoW), where validators perform real Tensorcore matrix computations that provide decentralized compute powering AI workloads.",
                "benefits": [
                    "Real AI training and inference computations",
                    "Verifiable through zk-proofs (via zkVerify integration)",
                    "Validators earn $AMA through block rewards and execution fees",
                    "Supports real-time agent activity"
                ]
            },
            "ecosystem_participants": {
                "builders": {
                    "role": "Create demand through agents and applications",
                    "activities": [
                        "Deploy smart contracts and dApps on WASM runtime",
                        "Create AI agents using Nova AI Compiler",
                        "Design agent swarms for perception, planning, and reasoning",
                        "Integrate data feeds via AI Agent Oracle Streams",
                        "Build modules for agent memory and identity management"
                    ]
                },
                "validators": {
                    "role": "Supply secure compute and finalize blocks",
                    "activities": [
                        "Participate in BLS-based consensus",
                        "Perform uPoW computations for AI workloads",
                        "Earn $AMA rewards for block production"
                    ]
                },
                "partners": {
                    "role": "Extend ecosystem with storage, verification, and tooling",
                    "integrations": ["Arweave (permanent storage)", "Crust Network (decentralized storage)", "zkVerify (zk-proof verification)"]
                }
            },
            "token": {
                "symbol": "AMA",
                "decimals": 9,
                "note": "1.0 AMA = 1,000,000,000 atomic units",
                "utility": [
                    "Agent execution fees",
                    "Smart contract deployment gas",
                    "Validator staking and rewards",
                    "Network governance"
                ]
            },
            "rpc_api": {
                "primary_endpoint": "https://nodes.amadeus.bot",
                "secondary_endpoints": [
                    "http://167.235.169.185",
                    "http://37.27.238.30"
                ],
                "endpoints": {
                    "chain": {
                        "GET /api/chain/stats": "Get blockchain statistics (height, tip hash, pflops, circulating supply)",
                        "GET /api/chain/height/{height}": "Get all entries at a specific block height",
                        "GET /api/chain/tx/{txId}": "Get transaction by hash",
                        "GET /api/chain/tx_events_by_account/{account}": "Get transaction history for account (supports limit, offset, sort, cursor)"
                    },
                    "wallet": {
                        "GET /api/wallet/balance_all/{publicKey}": "Get all token balances for a public key"
                    },
                    "transaction": {
                        "GET /api/tx/submit/{txPackedBase58}": "Submit Base58-encoded transaction via URL",
                        "POST /api/tx/submit": "Submit transaction via POST body (binary or Base58)"
                    },
                    "peer": {
                        "GET /api/peer/trainers": "Get all validator/trainer nodes",
                        "GET /api/peer/removed_trainers": "Get removed validators",
                        "GET /api/peer/nodes": "Get connected peer nodes"
                    },
                    "epoch": {
                        "GET /api/epoch/score": "Get validator mining scores for current epoch",
                        "GET /api/epoch/get_emission_address/{publicKey}": "Get emission address for validator"
                    }
                }
            },
            "transactions_technical": {
                "transaction_structure": {
                    "hash": "Transaction hash (Base58)",
                    "from": "Sender public key (Base58)",
                    "to": "Recipient address (Base58)",
                    "amount": "Transaction amount (string, in atomic units)",
                    "symbol": "Token symbol (e.g., 'AMA', 'NEURAL')",
                    "fee": "Transaction fee",
                    "nonce": "Transaction nonce (integer)",
                    "timestamp": "Transaction timestamp (integer)",
                    "signature": "Transaction signature (Base58)",
                    "type": "Transaction type: 'transfer', 'contract_call', or 'contract_deploy'"
                },
                "creating_transactions": {
                    "method_1_cli": {
                        "description": "Build transaction using amadeusd CLI",
                        "command": "SEED64=<sender_seed> ./amadeusd --bw-command start buildtx Coin transfer [\"<receiver>\", \"<amount>\", \"AMA\"]",
                        "example": "SEED64=2k3LidUYf... ./amadeusd --bw-command start buildtx Coin transfer [\"69TDon8KJp...\", \"1000000000\", \"AMA\"]",
                        "output": {
                            "signature": "Base58 signature",
                            "hash": "Transaction hash",
                            "tx_encoded": "Base64 encoded transaction for submission",
                            "tx": {
                                "nonce": "integer",
                                "signer": "public key",
                                "actions": [{"contract": "Coin", "function": "transfer", "op": "call", "args": ["receiver", "amount", "symbol"]}]
                            }
                        }
                    },
                    "method_2_cli_broadcast": {
                        "description": "Build and immediately broadcast transaction",
                        "command": "SEED64=<sender_seed> ./amadeusd --bw-command start build_and_broadcasttx Coin transfer [\"<receiver>\", \"<amount>\", \"AMA\"]",
                        "returns": "Transaction hash on success"
                    },
                    "method_3_elixir_api": {
                        "description": "Using Elixir RPC API (in node REPL)",
                        "single_transfer": "RPC.API.Wallet.transfer(seed64, receiver, amount, symbol)",
                        "bulk_transfer": "RPC.API.Wallet.transfer_bulk(seed64, [{receiver1, amount1}, {receiver2, amount2, \"USDT\"}])",
                        "note": "Amount as float (1.0) = 1,000,000,000 atomic units. Amount as integer (1) = 1 atomic unit"
                    }
                },
                "submitting_transactions": {
                    "http_post": {
                        "endpoint": "POST /api/tx/submit",
                        "content_types": ["application/octet-stream (binary)", "text/plain (Base58 string)"],
                        "response": {"error": "ok|invalid_signature|insufficient_funds|network_error", "tx_hash": "hash on success"}
                    },
                    "http_get": {
                        "endpoint": "GET /api/tx/submit/{txPackedBase58}",
                        "description": "Submit Base58-encoded transaction via URL parameter",
                        "response": {"error": "ok|invalid_signature|insufficient_funds|network_error", "tx_hash": "hash on success"}
                    }
                },
                "checking_transaction_status": {
                    "endpoint": "GET /api/chain/tx/{txHash}",
                    "response_fields": ["metadata.entry_hash", "metadata.entry_slot", "signature", "result.error", "hash", "tx.nonce", "tx.signer", "tx.actions"]
                },
                "amount_handling": {
                    "decimals": 9,
                    "examples": {
                        "1.0_float": "1,000,000,000 atomic units (1 AMA)",
                        "1_integer": "1 atomic unit (0.000000001 AMA)",
                        "1000000000_string": "1 AMA when passed as string in CLI"
                    }
                }
            },
            "wallet_operations": {
                "creating_wallet": {
                    "url": "https://wallet.ama.one/",
                    "steps": [
                        "Navigate to wallet.ama.one and click 'Create New Wallet'",
                        "Enter wallet name",
                        "Configure seed (64-byte master secret)",
                        "Create vault (encrypted storage)",
                        "Download vault file for backup"
                    ],
                    "security_notes": [
                        "Vault file contains encrypted wallet data, salt, IV, timestamp",
                        "No plaintext sensitive information stored",
                        "Follow 3-2-1 backup rule: 3 copies, 2 media types, 1 offsite"
                    ]
                },
                "sending_tokens": {
                    "requirements": ["Wallet must be unlocked", "Valid recipient Base58 address", "Sufficient balance"],
                    "process": "Transaction signed locally, submitted to network, confirms in 1-3 seconds"
                }
            },
            "running_a_node": {
                "download": "Get latest amadeusd release from GitHub",
                "run_command": "./amadeusd",
                "environment_variables": {
                    "WORKFOLDER": "Directory for blockchain data storage",
                    "OFFLINE": "Control peer connection (true for utility mode)",
                    "UDP_IPV4": "Network interface for UDP",
                    "UDP_PORT": "UDP port for P2P",
                    "PUBLIC_UDP_IPV4": "Public IP for NAT traversal",
                    "ANR_NAME": "Validator display name",
                    "ANR_DESC": "Validator description",
                    "HTTP_IPV4": "RPC API interface",
                    "HTTP_PORT": "RPC API port",
                    "ARCHIVALNODE": "Enable full chainstate storage",
                    "COMPUTOR": "Disable solver functionality"
                },
                "notes": [
                    "Seed stored in $WORKFOLDER/sk - keep secure",
                    "Full sync requires 170GB+ disk space",
                    "Recommended: stable 1gbps connection"
                ]
            },
            "mcp_tools_available": [
                "create_transaction - Create unsigned transaction",
                "submit_transaction - Submit signed transaction",
                "get_account_balance - Query account balances",
                "get_chain_stats - Get blockchain statistics",
                "get_transaction - Get transaction by hash",
                "get_transaction_history - Get account history",
                "get_validators - List validators",
                "claim_testnet_ama - Claim testnet tokens"
            ]
        })))
    }

    fn blockchain_error(tool: &str, error: BlockchainError) -> McpError {
        error!(%error, tool, "blockchain operation failed");
        match error {
            BlockchainError::AccountNotFound { address } => McpError::resource_not_found(
                "account_not_found",
                Some(serde_json::json!({ "address": address })),
            ),
            BlockchainError::InsufficientBalance {
                required,
                available,
            } => McpError::invalid_request(
                "insufficient_balance",
                Some(serde_json::json!({ "required": required, "available": available })),
            ),
            BlockchainError::ValidationFailed(msg) => McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "message": msg })),
            ),
            e => McpError::internal_error(
                "blockchain_error",
                Some(serde_json::json!({ "error": e.to_string() })),
            ),
        }
    }

    fn to_json<T: serde::Serialize>(value: T) -> Result<Json<serde_json::Value>, McpError> {
        Ok(Json(serde_json::to_value(value).map_err(|e| {
            McpError::internal_error(
                "serialization_error",
                Some(serde_json::json!({ "error": e.to_string() })),
            )
        })?))
    }
}

#[tool_handler]
impl ServerHandler for BlockchainMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            instructions: Some(
                "Blockchain MCP server for Amadeus. \
                Use create_transaction to build unsigned transactions, sign externally with BLS12-381, \
                then submit_transaction to broadcast."
                    .into(),
            ),
            protocol_version: Default::default(),
            server_info: Implementation {
                name: "amadeus-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let prompt_name = request.name.as_str();

        Err(McpError::invalid_params(
            "unknown_prompt",
            Some(serde_json::json!({ "name": prompt_name })),
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        Err(McpError::invalid_params(
            "invalid_uri",
            Some(serde_json::json!({ "message": format!("Unknown resource URI: {}", uri) })),
        ))
    }
}
