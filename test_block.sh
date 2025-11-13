#!/bin/bash
export BLOCKCHAIN_URL=https://nodes.amadeus.bot

# Test fetching block 37387100
(
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
sleep 0.5
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
sleep 0.5
echo '{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"amadeus://block/37387100"}}'
sleep 2
) | ./target/release/amadeus-mcp 2>&1
