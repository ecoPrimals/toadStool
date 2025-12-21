#!/bin/bash
# discover_tower.sh - Find gaming towers on your network

set -e

echo "🔍 Discovering Gaming Towers on Network"
echo "========================================"
echo ""

# Check if Songbird is running
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    echo ""
    echo "Start Songbird first:"
    echo "  cd /home/eastgate/Development/ecoPrimals/songbird"
    echo "  cargo run --release --bin songbird-orchestrator"
    exit 1
fi

echo "✅ Songbird is running"
echo ""
echo "Scanning network for towers with capabilities:"
echo "  • steam-library"
echo "  • game-execution"
echo "  • gpu-compute"
echo ""

# Discover towers
# Note: This endpoint will be implemented in Songbird
RESULT=$(curl -s -X POST http://localhost:8080/api/federation/discover \
  -H "Content-Type: application/json" \
  -d '{
    "capabilities": ["steam-library", "game-execution"],
    "timeout_seconds": 10
  }' || echo '{"towers": []}')

# Parse results
TOWER_COUNT=$(echo "$RESULT" | jq -r '.towers | length' 2>/dev/null || echo "0")

if [ "$TOWER_COUNT" = "0" ]; then
    echo "⚠️  No towers discovered"
    echo ""
    echo "Possible reasons:"
    echo "  1. Tower not running Songbird"
    echo "  2. Tower on different network"
    echo "  3. Firewall blocking discovery"
    echo ""
    echo "To advertise your tower:"
    echo "  cd federation-showcase"
    echo "  ./advertise_tower.sh"
    exit 0
fi

echo "✅ Found $TOWER_COUNT tower(s):"
echo ""

# Display towers
echo "$RESULT" | jq -r '.towers[] | 
  "🗼 Tower: \(.id)\n" +
  "   Address: \(.address)\n" +
  "   Steam Games: \(.steam_games // "unknown")\n" +
  "   GPU: \(.gpu // "unknown")\n" +
  "   Status: \(.status)\n"'

echo ""
echo "To connect to a tower:"
echo "  ./connect_to_tower.sh <tower-id>"
echo ""
echo "Example:"
echo "  ./connect_to_tower.sh gaming-tower-main"

