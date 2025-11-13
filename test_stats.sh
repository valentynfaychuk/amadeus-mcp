#!/bin/bash
export BLOCKCHAIN_URL=https://nodes.amadeus.bot

# Test fetching chain stats
(
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
sleep 0.5
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
sleep 0.5
echo '{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"amadeus://chain/stats"}}'
sleep 2
) | ./target/release/amadeus-mcp 2>&1
