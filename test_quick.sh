#!/bin/bash
# Quick test for JSON-RPC socket (raw format)

SOCKET="${XDG_RUNTIME_DIR:-/run/user/1000}/toadstool-default.jsonrpc.sock"

if [ ! -S "$SOCKET" ]; then
    echo "❌ Socket not found: $SOCKET"
    echo "   Start daemon: cargo run --release -- daemon"
    exit 1
fi

echo "🧪 Testing JSON-RPC socket: $SOCKET"
echo ""
echo "Sending: {\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"params\":{},\"id\":1}"

# Send raw JSON-RPC request (what biomeOS sends)
response=$(echo '{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":1}' | nc -U -w 2 "$SOCKET")

if [ -n "$response" ]; then
    echo "✅ Response received:"
    echo "$response" | python3 -m json.tool 2>/dev/null || echo "$response"
    echo ""
    echo "🎉 JSON-RPC socket is working!"
    exit 0
else
    echo "❌ No response (timeout)"
    exit 1
fi
