#!/bin/bash
# discover_eco_game_servers.sh
# Discover game servers via Songbird - NO MANUAL IP NEEDED!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🔍 Discover ecoPrimals Game Servers 🔍                 ║"
echo "║                                                              ║"
echo "║        Zero config, automatic, eco-native!                   ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

GAME_TYPE=${1:-""}

# Check if Songbird is running
echo "🔍 Checking for Songbird..."
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    echo ""
    echo "Start Songbird first:"
    echo "  cd ../../../songbird"
    echo "  cargo run --release --bin songbird-orchestrator"
    echo ""
    echo "Or use in-game LAN browser (less secure, manual):"
    echo "  openarena"
    echo "  Multiplayer → Local Servers"
    exit 1
fi

echo "✅ Songbird is running!"
echo ""

# Discover game servers
echo "📡 Discovering game servers on the network..."
echo "   (Searching for capability: game-server)"
echo ""

DISCOVERY_RESULT=$(curl -s -X POST http://localhost:8080/api/services/discover \
  -H "Content-Type: application/json" \
  -d '{
    "capabilities": ["game-server"],
    "timeout_seconds": 5
  }' || echo '{"services": []}')

# Parse results
SERVER_COUNT=$(echo "$DISCOVERY_RESULT" | jq -r '.services | length' 2>/dev/null || echo "0")

if [ "$SERVER_COUNT" = "0" ]; then
    echo "⚠️  No game servers found"
    echo ""
    echo "Possible reasons:"
    echo "  1. No servers running (start with: ./start_eco_game_server.sh)"
    echo "  2. Servers on different network"
    echo "  3. Firewall blocking discovery"
    echo ""
    echo "To start a server:"
    echo "  ./start_eco_game_server.sh"
    exit 0
fi

echo "✅ Found $SERVER_COUNT game server(s):"
echo ""
echo "═══════════════════════════════════════════════════════════════"

# Display servers
echo "$DISCOVERY_RESULT" | jq -r '.services[] | 
  "🎮 \(.metadata.game // "Unknown Game")\n" +
  "   Tower: \(.hostname)\n" +
  "   Map: \(.metadata.map // "unknown")\n" +
  "   Players: 0/\(.metadata.max_players // "?")\n" +
  "   Service ID: \(.service_id)\n" +
  "   Address: \(.address)\n"'

echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "To join a server:"
echo "  ./join_eco_game.sh <game-type>"
echo ""
echo "Example:"
echo "  ./join_eco_game.sh openarena"
echo ""
echo "Or join the first server automatically:"
echo "  ./join_eco_game.sh"

