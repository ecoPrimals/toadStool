#!/bin/bash
# play_local.sh
# Start server and connect client on the SAME tower
# Perfect for testing!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🎮 Local Play - Server + Client Same Tower 🎮          ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

MAP=${1:-"dm17"}
PORT=${2:-27960}

echo "🎯 This will:"
echo "  1. Start dedicated server in background"
echo "  2. Launch client and connect to localhost"
echo "  3. You play on your own server!"
echo ""
echo "Map: $MAP"
echo "Port: $PORT"
echo ""

read -p "Start? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
fi

echo ""
echo "🎮 Starting server in background..."

# Check if Songbird is running (optional)
if curl -s http://localhost:8080/health &>/dev/null; then
    echo "✅ Songbird detected - using eco-native mode"
    USE_ECO=true
else
    echo "ℹ️  Songbird not running - using direct mode"
    USE_ECO=false
fi

# Create server config
mkdir -p ~/.openarena/baseoa
cat > ~/.openarena/baseoa/server.cfg << EOF
seta sv_hostname "🍄 $(hostname) - Local Server"
seta sv_maxclients 16
seta g_gametype 0
seta bot_enable 1
seta bot_minplayers 2
seta g_spSkill 2
EOF

# Start server in background
(
    openarena \
        +set dedicated 1 \
        +set net_port $PORT \
        +exec server.cfg \
        +map $MAP \
        > /tmp/openarena-server.log 2>&1
) &

SERVER_PID=$!
echo "  Server PID: $SERVER_PID"
echo "  Server log: /tmp/openarena-server.log"

# Save PID for cleanup
echo $SERVER_PID > /tmp/openarena-server.pid

echo ""
echo "⏳ Waiting 5 seconds for server to start..."
sleep 5

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ Server failed to start!"
    echo "Check log: cat /tmp/openarena-server.log"
    exit 1
fi

echo "✅ Server running!"
echo ""
echo "🎮 Launching client and connecting to localhost..."
echo ""
sleep 1

# Launch client and connect to localhost
openarena +connect 127.0.0.1:$PORT

echo ""
echo "Game ended."
echo ""
echo "Server is still running (PID: $SERVER_PID)"
echo ""
echo "To stop server:"
echo "  kill $SERVER_PID"
echo ""
echo "Or run:"
echo "  ./stop_local_server.sh"

