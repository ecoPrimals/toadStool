#!/bin/bash
# quick_lan_game.sh
# Quickly launch both server and client for testing

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🎮 Quick OpenArena LAN Game Setup 🎮                   ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

MAP=${1:-"dm17"}
PORT=${2:-27960}

echo "Setting up LAN game on map: $MAP (port $PORT)"
echo ""

# Get IP
IP=$(hostname -I | awk '{print $1}')
echo "🌐 Server IP: $IP"
echo ""

echo "This will:"
echo "  1. Launch server in background"
echo "  2. Wait 5 seconds"
echo "  3. Launch client and auto-connect"
echo ""

read -p "Continue? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
fi

echo ""
echo "🎮 Starting server in background..."

# Start server in background (suppress output)
(
    openarena \
        +set dedicated 2 \
        +set net_port $PORT \
        +set sv_hostname "ecoPrimals LAN Server" \
        +set sv_maxclients 16 \
        +set g_gametype 0 \
        +set bot_enable 1 \
        +set bot_minplayers 2 \
        +map $MAP \
        > /tmp/openarena-server.log 2>&1
) &

SERVER_PID=$!
echo "  Server PID: $SERVER_PID"
echo "  Log: /tmp/openarena-server.log"
echo ""

echo "Waiting 5 seconds for server to start..."
sleep 5

echo ""
echo "🎮 Launching client and connecting to $IP:$PORT..."
echo ""

# Launch client and auto-connect
openarena +connect $IP:$PORT

echo ""
echo "Game ended. Server is still running (PID: $SERVER_PID)"
echo ""
echo "To stop server:"
echo "  kill $SERVER_PID"
echo ""
echo "To view server log:"
echo "  tail -f /tmp/openarena-server.log"

