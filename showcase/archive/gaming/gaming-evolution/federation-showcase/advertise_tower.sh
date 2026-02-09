#!/bin/bash
# advertise_tower.sh - Advertise this machine as a gaming tower

set -e

echo "🗼 Advertising Gaming Tower"
echo "==========================="
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

# Detect Steam library
STEAM_LIBRARY=""
if [ -d "$HOME/.steam/steam/steamapps" ]; then
    STEAM_LIBRARY="$HOME/.steam/steam/steamapps"
elif [ -d "$HOME/.local/share/Steam/steamapps" ]; then
    STEAM_LIBRARY="$HOME/.local/share/Steam/steamapps"
fi

if [ -z "$STEAM_LIBRARY" ]; then
    echo "⚠️  Steam library not found"
    echo "Using demo mode"
    GAME_COUNT=0
else
    echo "✅ Found Steam library: $STEAM_LIBRARY"
    GAME_COUNT=$(find "$STEAM_LIBRARY/common" -maxdepth 1 -type d 2>/dev/null | wc -l)
    echo "   Games: $GAME_COUNT"
fi

# Detect GPU
GPU_INFO=$(lspci | grep -i vga | head -1 || echo "Unknown")
echo "   GPU: $GPU_INFO"
echo ""

# Get hostname
HOSTNAME=$(hostname)
TOWER_ID="$HOSTNAME-tower"

echo "Advertising as: $TOWER_ID"
echo ""

# Advertise capabilities
curl -X POST http://localhost:8080/api/federation/advertise \
  -H "Content-Type: application/json" \
  -d "{
    \"id\": \"$TOWER_ID\",
    \"hostname\": \"$HOSTNAME\",
    \"capabilities\": [
      \"steam-library\",
      \"game-execution\",
      \"gpu-compute\"
    ],
    \"steam_library_path\": \"$STEAM_LIBRARY\",
    \"available_games\": $GAME_COUNT,
    \"gpu_info\": \"$GPU_INFO\"
  }"

echo ""
echo ""
echo "✅ Tower advertised successfully!"
echo ""
echo "Other devices can now discover you:"
echo "  ./discover_tower.sh"
echo ""
echo "Your tower info:"
echo "  ID: $TOWER_ID"
echo "  Games: $GAME_COUNT"
echo "  GPU: $GPU_INFO"

