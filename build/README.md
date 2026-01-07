# Amadeus MCP - Blockchain Server

MCP server enabling AI agents to interact with the Amadeus blockchain.

## Usage

### Install in Gemini CLI

See [Gemini CLI Configuration](https://google-gemini.github.io/gemini-cli/docs/tools/mcp-server.html) for details.

Open the Gemini CLI settings file. The location is `~/.gemini/settings.json`.
Add the following to the `mcpServers` object in your `settings.json` file:

```json
{
  "mcpServers": {
    "amadeus": {
      "httpUrl": "https://mcp.ama.one",
    }
  }
}
```

### Install in Claude Code

Run this command. See [Claude Code MCP docs](https://docs.anthropic.com/en/docs/claude-code/mcp) for more info.

```sh
claude mcp add --transport http amadeus https://mcp.ama.one
```

Or open the Claude Code config file. The location is `~/.claude.json`.
Find the following to the `mcpServers` object in the desired folder section:

```json
"mcpServers": {
  "amadeus": {
    "type": "http",
    "url": "https://mcp.ama.one"
  }
}
```

## Tools

- `create_transaction` - Create unsigned transaction for any contract call (args: signer, contract, function, args)
- `submit_transaction` - Submit signed transaction (args: transaction, signature, network: mainnet|testnet)
- `get_account_balance` - Query account balances
- `get_chain_stats` - Get blockchain statistics
- `get_block_by_height` - Get entries at height
- `get_transaction` - Get transaction by hash
- `get_transaction_history` - Get account transaction history
- `get_validators` - List validators
- `get_contract_state` - Query contract storage
- `claim_testnet_ama` - Claim testnet tokens (once per 24h per IP)

## Development

### Quick Start

#### Stdio Mode

```bash
cargo build --release
./target/release/amadeus-mcp
```

#### HTTP Mode (Cloudflare Workers)

Local dev:
```bash
npm i -g wrangler
cargo install worker-build
wrangler dev
```

Production (build locally, then deploy):
```bash
scripts/build.sh
wrangler deploy
wrangler secret put BLOCKCHAIN_API_KEY
```
### Configuration

```bash
BLOCKCHAIN_URL=https://nodes.amadeus.bot (mainnet, default)
AMADEUS_TESTNET_RPC=https://testnet.amadeus.bot (testnet, default)
AMADEUS_TESTNET_SK (secret, base58-encoded 64-byte key for faucet)
MCP_DATABASE (D1 binding)
```

### Database Migration

Create the faucet_claims table in D1:
```sql
CREATE TABLE faucet_claims (ip TEXT PRIMARY KEY, address TEXT, claimed_at INTEGER);
```

## Creating Transactions

TypeScript/JavaScript example:

```bash
cd examples && npm install @noble/curves bs58
node sign-transaction.mjs <sk_base58> <contract> <function> '<args_json>' [network]
```

Examples:
```bash
node sign-transaction.mjs YOUR_SK Coin transfer '[{"b58":"RECIPIENT"},"1000000000","AMA"]'
node sign-transaction.mjs YOUR_SK Coin transfer '[{"b58":"RECIPIENT"},"1000000000","AMA"]' testnet
```

Creates unsigned transaction via MCP, signs locally with BLS12-381, submits to mainnet (default) or testnet.
