#!/usr/bin/env bash
# Test JSON-RPC over Unix socket

set -e

SOCKET_PATH="${1:-/run/user/$(id -u)/toadstool-default.jsonrpc.sock}"

echo "Testing JSON-RPC 2.0 server at: $SOCKET_PATH"
echo ""

# Test 1: Health Check
echo "1. Testing health check..."
echo '{"jsonrpc":"2.0","method":"toadstool.health","id":1}' | \
  socat - "UNIX-CONNECT:$SOCKET_PATH" | \
  python3 -m json.tool
echo ""

# Test 2: Version Query
echo "2. Testing version query..."
echo '{"jsonrpc":"2.0","method":"toadstool.version","id":2}' | \
  socat - "UNIX-CONNECT:$SOCKET_PATH" | \
  python3 -m json.tool
echo ""

# Test 3: Capabilities Query
echo "3. Testing capabilities query..."
echo '{"jsonrpc":"2.0","method":"toadstool.query_capabilities","id":3}' | \
  socat - "UNIX-CONNECT:$SOCKET_PATH" | \
  python3 -m json.tool
echo ""

# Test 4: Invalid method (should return error)
echo "4. Testing invalid method (should return error)..."
echo '{"jsonrpc":"2.0","method":"invalid.method","id":4}' | \
  socat - "UNIX-CONNECT:$SOCKET_PATH" | \
  python3 -m json.tool
echo ""

echo "✅ All tests complete!"

