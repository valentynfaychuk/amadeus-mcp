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
      "httpUrl": "https://amadeus-mcp.faychuk.workers.dev",
    }
  }
}
```

### Install in Claude Code

Run this command. See [Claude Code MCP docs](https://docs.anthropic.com/en/docs/claude-code/mcp) for more info.

```sh
claude mcp add --transport http amadeus https://amadeus-mcp.faychuk.workers.dev
```

Or open the Claude Code config file. The location is `~/.claude.json`.
Find the following to the `mcpServers` object in the desired folder section:

```json
"mcpServers": {
  "amadeus": {
    "type": "http",
    "url": "https://amadeus-mcp.faychuk.workers.dev"
  }
}
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

### Faucet Tools
- `claim_testnet_ama` - Claim testnet AMA tokens (once per 24 hours per IP)

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
BLOCKCHAIN_URL=https://nodes.amadeus.bot
AMADEUS_TESTNET_RPC=https://nodes.amadeus.bot
AMADEUS_TESTNET_MINT_KEY (secret)
MCP_DATABASE (D1 binding)
```

### Database Migration

Create the faucet_claims table in D1:
```sql
CREATE TABLE faucet_claims (ip TEXT PRIMARY KEY, address TEXT, claimed_at INTEGER);
```
