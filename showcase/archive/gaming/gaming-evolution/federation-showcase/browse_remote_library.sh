#!/bin/bash
# browse_remote_library.sh - Browse games on a remote tower

set -e

TOWER_ID=${1:-"gaming-tower-main"}

echo "📚 Browsing Games on Tower: $TOWER_ID"
echo "======================================"
echo ""

# Check if Songbird is running
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    exit 1
fi

# Get library
LIBRARY=$(curl -s http://localhost:8080/api/federation/tower/$TOWER_ID/library)

# Check if successful
GAME_COUNT=$(echo "$LIBRARY" | jq -r '.games | length' 2>/dev/null || echo "0")

if [ "$GAME_COUNT" = "0" ]; then
    echo "⚠️  No games found or tower not connected"
    echo ""
    echo "Try:"
    echo "  1. Connect to tower:"
    echo "     ./connect_to_tower.sh $TOWER_ID"
    echo ""
    echo "  2. Verify tower is advertising games"
    exit 1
fi

echo "✅ Found $GAME_COUNT games"
echo ""

# Display top 20 games
echo "Top 20 games:"
echo ""
echo "$LIBRARY" | jq -r '.games[0:20][] | 
  "  \(.app_id): \(.name)\n" +
  "      Size: \(.size_gb)GB | Last played: \(.last_played // "never")"' 

echo ""
echo "... and $(($GAME_COUNT - 20)) more!"
echo ""
echo "To launch a game:"
echo "  ./launch_remote_game.sh <app_id> $TOWER_ID"
echo ""
echo "Popular Steam App IDs:"
echo "  730  - Counter-Strike: Global Offensive"
echo "  440  - Team Fortress 2"
echo "  570  - Dota 2"
echo "  252490 - Rust"
echo "  1086940 - Baldur's Gate 3"

