# Amadeus MCP - Blockchain Server

Production-ready MCP server enabling AI agents to interact with blockchain networks.

## Quick Start

### Stdio Mode

```bash
cargo build --release
./target/release/amadeus-mcp
```

Configure in `.claude.json`:
```json
"amadeus-chain": {
  "command": "/path/to/amadeus-mcp",
  "args": []
}
```

### HTTP Mode

Local dev server (port 8787):
```bash
npm i -g wrangler && cargo install worker-build
wrangler dev
```

Production deployment:
```bash
wrangler deploy
wrangler secret put BLOCKCHAIN_API_KEY
```

## Use Cases

### 1. Token Transfer Assistant
AI agent helps users transfer tokens between accounts:
```
User: "Send 10 USDC from my wallet to alice@example.com"

Agent calls create_transfer:
{
  "symbol": "USDC",
  "source": "0x123...",
  "destination": "0x456...",
  "amount": "10"
}

→ Returns unsigned blob
→ User signs with their wallet
→ Agent calls submit_transaction
→ Transaction confirmed!
```

### 2. Portfolio Management
AI monitors and rebalances crypto portfolios:
```
Agent: Checking balances across accounts...
→ get_account_balance for each account
→ Analyzes allocation
→ Creates rebalancing transfers
→ User approves and signs
→ Executes transactions
```

### 3. Automated Payments
Schedule recurring blockchain payments:
```
Agent: Monthly rent payment due
→ create_transfer with payment details
→ Notification sent to user for signature
→ User signs via wallet app
→ submit_transaction
→ Payment complete, receipt logged
```

### 4. Multi-sig Coordinator
Coordinate multi-signature wallet operations:
```
Agent: 3 of 5 signatures needed
→ create_transfer for proposal
→ Collects signatures from authorized parties
→ submit_transaction when threshold met
→ Funds released
```

### 5. Compliance & Audit
AI assistant for transaction compliance:
```
Agent: Pre-flight checks before transfer
→ get_account_balance (verify funds)
→ create_transfer (validate transaction)
→ Checks compliance rules
→ User approves if compliant
→ submit_transaction
```

### 6. Blockchain Explorer
AI-powered blockchain data exploration:
```
User: "Show me the latest block and its transactions"

Agent calls get_chain_stats:
→ Gets current blockchain height

Agent calls get_block_by_height:
→ Retrieves block entries at current height

Agent calls get_transaction_history:
→ Analyzes transaction patterns
→ Presents insights and statistics
```

### 7. Portfolio Tracker
Track and analyze transaction history:
```
User: "Show my transaction history for the last week"

Agent calls get_transaction_history:
→ Fetches recent transactions
→ Calculates total inflows/outflows
→ Identifies most frequent recipients
→ Presents portfolio summary
```

## Tools

### Transaction Tools
- `create_transfer` - Build unsigned transaction blob
- `submit_transaction` - Broadcast signed transaction

### Account & Balance Tools
- `get_account_balance` - Query all token balances for an account

### Blockchain Query Tools
- `get_chain_stats` - Get current blockchain statistics (height, total transactions, total accounts)
- `get_block_by_height` - Retrieve blockchain entries at a specific height
- `get_transaction` - Get detailed transaction information by hash
- `get_transaction_history` - Query transaction history for an account (with pagination)

### Network Tools
- `get_validators` - Get list of current validator nodes (trainers)

### Smart Contract Tools
- `get_contract_state` - Query smart contract storage by address and key

## Configuration

```bash
BLOCKCHAIN_URL=https://nodes.amadeus.bot
BLOCKCHAIN_API_KEY=your-key
RUST_LOG=info
```
