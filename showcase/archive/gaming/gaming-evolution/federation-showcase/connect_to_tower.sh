#!/bin/bash
# connect_to_tower.sh - Establish connection to a gaming tower

set -e

TOWER_ID=$1

if [ -z "$TOWER_ID" ]; then
    echo "Usage: $0 <tower-id>"
    echo ""
    echo "Example:"
    echo "  $0 gaming-tower-main"
    echo ""
    echo "To discover towers:"
    echo "  ./discover_tower.sh"
    exit 1
fi

echo "🔗 Connecting to Tower: $TOWER_ID"
echo "============================"
echo ""

# Check if Songbird is running
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    exit 1
fi

# Establish connection
RESULT=$(curl -s -X POST http://localhost:8080/api/federation/connect \
  -H "Content-Type: application/json" \
  -d "{
    \"tower_id\": \"$TOWER_ID\",
    \"purpose\": \"steam-library-access\",
    \"capabilities_requested\": [
      \"steam-library\",
      \"game-execution\"
    ]
  }")

# Check result
STATUS=$(echo "$RESULT" | jq -r '.status' 2>/dev/null || echo "error")

if [ "$STATUS" = "connected" ]; then
    echo "✅ Connected to $TOWER_ID!"
    echo ""
    
    # Show tower info
    echo "Tower information:"
    echo "$RESULT" | jq -r '
      "  Games: \(.games)\n" +
      "  Address: \(.address)\n" +
      "  GPU: \(.gpu)"'
    
    echo ""
    echo "Next steps:"
    echo "  1. Browse library:"
    echo "     ./browse_remote_library.sh $TOWER_ID"
    echo ""
    echo "  2. Launch a game:"
    echo "     ./launch_remote_game.sh <app_id> $TOWER_ID"
else
    echo "❌ Failed to connect to $TOWER_ID"
    echo ""
    echo "Error: $(echo "$RESULT" | jq -r '.error')"
    echo ""
    echo "Try:"
    echo "  1. Verify tower is advertising:"
    echo "     ./discover_tower.sh"
    echo ""
    echo "  2. Check network connectivity"
    exit 1
fi

