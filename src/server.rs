use crate::blockchain::{
    BlockchainClient, BlockchainError, SignedTransaction, TransferRequest, AccountQuery,
    HeightQuery, TransactionQuery, TransactionHistoryQuery, ContractStateQuery,
};
use rmcp::{
    handler::server::tool::{ToolRouter, Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, Json, RoleServer, ServerHandler,
};
use std::{future::Future, sync::Arc};
use tracing::error;

#[derive(Clone)]
pub struct BlockchainMcpServer {
    blockchain: Arc<BlockchainClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BlockchainMcpServer {
    pub fn new(blockchain: BlockchainClient) -> Self {
        Self {
            blockchain: Arc::new(blockchain),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "create_transfer",
        description = "Creates an unsigned transaction blob for transferring assets between accounts. Returns the blob and signing payload for the agent to sign."
    )]
    async fn create_transfer(
        &self,
        params: Parameters<TransferRequest>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let req = params.0;

        let blob = self
            .blockchain
            .create_transfer_blob(req)
            .await
            .map_err(|e| Self::blockchain_error("create_transfer", e))?;

        Ok(Json(serde_json::json!({
            "blob": blob.blob,
            "signing_payload": blob.signing_payload,
            "transaction_hash": blob.transaction_hash,
            "status": "unsigned",
            "next_step": "Sign the signing_payload and call submit_transaction with the signature"
        })))
    }

    #[tool(
        name = "submit_transaction",
        description = "Submits a signed transaction to the blockchain network. Requires the transaction blob and signature from the signing process."
    )]
    async fn submit_transaction(
        &self,
        params: Parameters<SignedTransaction>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let tx = params.0;

        let response = self
            .blockchain
            .submit_signed_transaction(tx)
            .await
            .map_err(|e| Self::blockchain_error("submit_transaction", e))?;

        Ok(Json(serde_json::json!({
            "transaction_hash": response.transaction_hash,
            "status": response.status,
            "message": "Transaction submitted successfully"
        })))
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

        let balance = self
            .blockchain
            .get_account_balance(&query.address)
            .await
            .map_err(|e| Self::blockchain_error("get_account_balance", e))?;

        Ok(Json(serde_json::to_value(balance).map_err(|e| {
            McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() })))
        })?))
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

        Ok(Json(serde_json::to_value(stats).map_err(|e| {
            McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() })))
        })?))
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

        let entries = self
            .blockchain
            .get_block_by_height(query.height)
            .await
            .map_err(|e| Self::blockchain_error("get_block_by_height", e))?;

        Ok(Json(serde_json::to_value(entries).map_err(|e| {
            McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() })))
        })?))
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

        let transaction = self
            .blockchain
            .get_transaction(&query.tx_hash)
            .await
            .map_err(|e| Self::blockchain_error("get_transaction", e))?;

        Ok(Json(serde_json::to_value(transaction).map_err(|e| {
            McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() })))
        })?))
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

        Ok(Json(serde_json::to_value(transactions).map_err(|e| {
            McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() })))
        })?))
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

    fn blockchain_error(tool: &str, error: BlockchainError) -> McpError {
        error!(%error, tool, "blockchain operation failed");

        match error {
            BlockchainError::AccountNotFound { address } => {
                McpError::resource_not_found("account_not_found", Some(serde_json::json!({ "address": address })))
            }
            BlockchainError::InsufficientBalance { required, available } => {
                McpError::invalid_request(
                    "insufficient_balance",
                    Some(serde_json::json!({ "required": required, "available": available })),
                )
            }
            BlockchainError::ValidationFailed(msg) => {
                McpError::invalid_params("validation_failed", Some(serde_json::json!({ "message": msg })))
            }
            e => McpError::internal_error("blockchain_error", Some(serde_json::json!({ "error": e.to_string() }))),
        }
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
                "Blockchain MCP server for creating and submitting transactions. \
                Use create_transfer to build an unsigned transaction, sign it externally, \
                then use submit_transaction to broadcast it to the network."
                    .into(),
            ),
            protocol_version: Default::default(),
            server_info: Implementation {
                name: "amadeus-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        async move {
            Ok(ListPromptsResult {
                prompts: vec![
                    Prompt {
                        name: "check_balance".into(),
                        description: Some("Check account balance for a specific address".into()),
                        arguments: Some(vec![
                            PromptArgument {
                                name: "address".into(),
                                description: Some("The account address to check".into()),
                                required: Some(true),
                            },
                        ]),
                    },
                    Prompt {
                        name: "view_transaction".into(),
                        description: Some("View transaction details by hash".into()),
                        arguments: Some(vec![
                            PromptArgument {
                                name: "hash".into(),
                                description: Some("The transaction hash".into()),
                                required: Some(true),
                            },
                        ]),
                    },
                    Prompt {
                        name: "view_block".into(),
                        description: Some("View block details by height".into()),
                        arguments: Some(vec![
                            PromptArgument {
                                name: "height".into(),
                                description: Some("The block height".into()),
                                required: Some(true),
                            },
                        ]),
                    },
                    Prompt {
                        name: "blockchain_stats".into(),
                        description: Some("View current blockchain statistics".into()),
                        arguments: None,
                    },
                ],
                next_cursor: None,
            })
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        async move {
            let prompt_name = request.name.as_str();

            match prompt_name {
                "check_balance" => {
                    let address = request.arguments.as_ref()
                        .and_then(|args| args.get("address"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_params("missing_address", None))?;

                    Ok(GetPromptResult {
                        description: Some(format!("Check balance for address: {}", address)),
                        messages: vec![
                            PromptMessage {
                                role: PromptMessageRole::User,
                                content: PromptMessageContent::Text {
                                    text: format!("Please check the balance for address {} using the get_account_balance tool or the amadeus://account/{}/balance resource", address, address),
                                },
                            },
                        ],
                    })
                }
                "view_transaction" => {
                    let hash = request.arguments.as_ref()
                        .and_then(|args| args.get("hash"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_params("missing_hash", None))?;

                    Ok(GetPromptResult {
                        description: Some(format!("View transaction: {}", hash)),
                        messages: vec![
                            PromptMessage {
                                role: PromptMessageRole::User,
                                content: PromptMessageContent::Text {
                                    text: format!("Please show me the details of transaction {} using the get_transaction tool or the amadeus://transaction/{} resource", hash, hash),
                                },
                            },
                        ],
                    })
                }
                "view_block" => {
                    let height = request.arguments.as_ref()
                        .and_then(|args| args.get("height"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_params("missing_height", None))?;

                    Ok(GetPromptResult {
                        description: Some(format!("View block at height: {}", height)),
                        messages: vec![
                            PromptMessage {
                                role: PromptMessageRole::User,
                                content: PromptMessageContent::Text {
                                    text: format!("Please show me the block at height {} using the get_block_by_height tool or the amadeus://block/{} resource", height, height),
                                },
                            },
                        ],
                    })
                }
                "blockchain_stats" => {
                    Ok(GetPromptResult {
                        description: Some("View current blockchain statistics".into()),
                        messages: vec![
                            PromptMessage {
                                role: PromptMessageRole::User,
                                content: PromptMessageContent::Text {
                                    text: "Please show me the current blockchain statistics using the get_chain_stats tool or the amadeus://chain/stats resource".into(),
                                },
                            },
                        ],
                    })
                }
                _ => Err(McpError::invalid_params("unknown_prompt", Some(serde_json::json!({ "name": prompt_name }))))
            }
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            Ok(ListResourcesResult {
                resources: vec![
                    Resource {
                        raw: RawResource {
                            uri: "amadeus://chain/stats".into(),
                            name: "Blockchain Statistics".into(),
                            description: Some("Current blockchain statistics including height, total transactions, and total accounts".into()),
                            mime_type: Some("application/json".into()),
                            size: None,
                        },
                        annotations: None,
                    },
                    Resource {
                        raw: RawResource {
                            uri: "amadeus://validators".into(),
                            name: "Validator List".into(),
                            description: Some("List of current validator nodes (trainers) in the network".into()),
                            mime_type: Some("application/json".into()),
                            size: None,
                        },
                        annotations: None,
                    },
                ],
                next_cursor: None,
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        async move {
            Ok(ListResourceTemplatesResult {
                resource_templates: vec![
                    ResourceTemplate {
                        raw: RawResourceTemplate {
                            uri_template: "amadeus://block/{height}".into(),
                            name: "Block by Height".into(),
                            description: Some("Retrieve blockchain entries at a specific height".into()),
                            mime_type: Some("application/json".into()),
                        },
                        annotations: None,
                    },
                    ResourceTemplate {
                        raw: RawResourceTemplate {
                            uri_template: "amadeus://transaction/{hash}".into(),
                            name: "Transaction".into(),
                            description: Some("Get detailed transaction information by hash".into()),
                            mime_type: Some("application/json".into()),
                        },
                        annotations: None,
                    },
                    ResourceTemplate {
                        raw: RawResourceTemplate {
                            uri_template: "amadeus://account/{address}/balance".into(),
                            name: "Account Balance".into(),
                            description: Some("Query all token balances for an account".into()),
                            mime_type: Some("application/json".into()),
                        },
                        annotations: None,
                    },
                    ResourceTemplate {
                        raw: RawResourceTemplate {
                            uri_template: "amadeus://account/{address}/history".into(),
                            name: "Transaction History".into(),
                            description: Some("Query transaction history for an account".into()),
                            mime_type: Some("application/json".into()),
                        },
                        annotations: None,
                    },
                    ResourceTemplate {
                        raw: RawResourceTemplate {
                            uri_template: "amadeus://contract/{address}/{key}".into(),
                            name: "Contract State".into(),
                            description: Some("Query smart contract storage by address and key".into()),
                            mime_type: Some("application/json".into()),
                        },
                        annotations: None,
                    },
                ],
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let uri = request.uri.as_str();

            // Parse the URI and route to appropriate handler
            if uri == "amadeus://chain/stats" {
                let stats = self
                    .blockchain
                    .get_chain_stats()
                    .await
                    .map_err(|e| Self::blockchain_error("get_chain_stats", e))?;

                let json_content = serde_json::to_string_pretty(&stats)
                    .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(json_content, uri)],
                });
            }

            if uri == "amadeus://validators" {
                let validators = self
                    .blockchain
                    .get_validators()
                    .await
                    .map_err(|e| Self::blockchain_error("get_validators", e))?;

                let json_content = serde_json::to_string_pretty(&serde_json::json!({
                    "validators": validators,
                    "count": validators.len()
                }))
                .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(json_content, uri)],
                });
            }

            // Handle templated URIs
            if let Some(height_str) = uri.strip_prefix("amadeus://block/") {
                let height: u64 = height_str.parse()
                    .map_err(|_| McpError::invalid_params("invalid_height", Some(serde_json::json!({ "message": "Height must be a valid number" }))))?;

                let entries = self
                    .blockchain
                    .get_block_by_height(height)
                    .await
                    .map_err(|e| Self::blockchain_error("get_block_by_height", e))?;

                let json_content = serde_json::to_string_pretty(&entries)
                    .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(json_content, uri)],
                });
            }

            if let Some(hash) = uri.strip_prefix("amadeus://transaction/") {
                let transaction = self
                    .blockchain
                    .get_transaction(hash)
                    .await
                    .map_err(|e| Self::blockchain_error("get_transaction", e))?;

                let json_content = serde_json::to_string_pretty(&transaction)
                    .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(json_content, uri)],
                });
            }

            if let Some(remainder) = uri.strip_prefix("amadeus://account/") {
                if let Some(address) = remainder.strip_suffix("/balance") {
                    let balance = self
                        .blockchain
                        .get_account_balance(address)
                        .await
                        .map_err(|e| Self::blockchain_error("get_account_balance", e))?;

                    let json_content = serde_json::to_string_pretty(&balance)
                        .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                    return Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(json_content, uri)],
                    });
                }

                if let Some(address) = remainder.strip_suffix("/history") {
                    let transactions = self
                        .blockchain
                        .get_transaction_history(address, Some(100), None, Some("desc"))
                        .await
                        .map_err(|e| Self::blockchain_error("get_transaction_history", e))?;

                    let json_content = serde_json::to_string_pretty(&transactions)
                        .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                    return Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(json_content, uri)],
                    });
                }
            }

            if let Some(remainder) = uri.strip_prefix("amadeus://contract/") {
                let parts: Vec<&str> = remainder.split('/').collect();
                if parts.len() == 2 {
                    let contract_address = parts[0];
                    let key = parts[1];

                    let state = self
                        .blockchain
                        .get_contract_state(contract_address, key)
                        .await
                        .map_err(|e| Self::blockchain_error("get_contract_state", e))?;

                    let json_content = serde_json::to_string_pretty(&serde_json::json!({
                        "contract_address": contract_address,
                        "key": key,
                        "value": state
                    }))
                    .map_err(|e| McpError::internal_error("serialization_error", Some(serde_json::json!({ "error": e.to_string() }))))?;

                    return Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(json_content, uri)],
                    });
                }
            }

            Err(McpError::invalid_params("invalid_uri", Some(serde_json::json!({ "message": format!("Unknown resource URI: {}", uri) }))))
        }
    }
}
