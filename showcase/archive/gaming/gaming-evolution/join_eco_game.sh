#!/bin/bash
# join_eco_game.sh
# Auto-connect to game server via Songbird discovery
# NO MANUAL IP NEEDED!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║         🎮 Join ecoPrimals Game - Auto! 🎮                   ║"
echo "║                                                              ║"
echo "║     Zero config, just works, eco-native!                     ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

GAME_TYPE=${1:-"openarena"}

# Check if Songbird is running
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    echo ""
    echo "Start Songbird first:"
    echo "  cd ../../../songbird"
    echo "  cargo run --release --bin songbird-orchestrator"
    exit 1
fi

echo "🔍 Discovering $GAME_TYPE servers..."
echo ""

# Discover servers
DISCOVERY_RESULT=$(curl -s -X POST http://localhost:8080/api/services/discover \
  -H "Content-Type: application/json" \
  -d "{
    \"capabilities\": [\"game-server\", \"$GAME_TYPE\"],
    \"timeout_seconds\": 5
  }" || echo '{"services": []}')

SERVER_COUNT=$(echo "$DISCOVERY_RESULT" | jq -r '.services | length' 2>/dev/null || echo "0")

if [ "$SERVER_COUNT" = "0" ]; then
    echo "❌ No $GAME_TYPE servers found"
    echo ""
    echo "Start a server first:"
    echo "  ./start_eco_game_server.sh"
    exit 1
fi

echo "✅ Found $SERVER_COUNT server(s)!"
echo ""

# Get first server (or let user choose if multiple)
if [ "$SERVER_COUNT" = "1" ]; then
    # Auto-join first server
    SERVER_ADDRESS=$(echo "$DISCOVERY_RESULT" | jq -r '.services[0].address')
    SERVER_HOSTNAME=$(echo "$DISCOVERY_RESULT" | jq -r '.services[0].hostname')
    SERVER_MAP=$(echo "$DISCOVERY_RESULT" | jq -r '.services[0].metadata.map // "unknown"')
    
    echo "🎮 Joining server:"
    echo "   Tower: $SERVER_HOSTNAME"
    echo "   Map: $SERVER_MAP"
    echo "   Address: $SERVER_ADDRESS"
    echo ""
    echo "🚀 Launching OpenArena..."
    sleep 2
    
    openarena +connect $SERVER_ADDRESS
else
    # Multiple servers - let user choose
    echo "Multiple servers available:"
    echo ""
    
    echo "$DISCOVERY_RESULT" | jq -r '.services[] | 
      "  [\(.hostname)] - Map: \(.metadata.map // "?") - \(.address)"'
    
    echo ""
    read -p "Enter tower name to join (or press Enter for first): " CHOICE
    
    if [ -z "$CHOICE" ]; then
        # Join first
        SERVER_ADDRESS=$(echo "$DISCOVERY_RESULT" | jq -r '.services[0].address')
    else
        # Find by hostname
        SERVER_ADDRESS=$(echo "$DISCOVERY_RESULT" | jq -r ".services[] | select(.hostname | contains(\"$CHOICE\")) | .address" | head -1)
        
        if [ -z "$SERVER_ADDRESS" ]; then
            echo "❌ Server not found: $CHOICE"
            exit 1
        fi
    fi
    
    echo ""
    echo "🚀 Connecting to $SERVER_ADDRESS..."
    sleep 1
    
    openarena +connect $SERVER_ADDRESS
fi

echo ""
echo "Game ended."

