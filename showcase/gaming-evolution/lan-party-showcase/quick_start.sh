#!/bin/bash
# LAN Party Quick Start Script
# Gets you playing classic games FAST!

set -e

echo "🎉 LAN Party Quick Start!"
echo "========================"
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

# Check Songbird
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "  ✅ Songbird is running"
else
    echo "  ❌ Songbird is NOT running"
    echo ""
    echo "Please start Songbird first:"
    echo "  cd /home/eastgate/Development/ecoPrimals/songbird"
    echo "  cargo run --release --bin songbird-orchestrator"
    echo ""
    exit 1
fi

# Check for games directory
if [ ! -d "/tmp/games" ]; then
    echo "  📁 Creating /tmp/games directory..."
    mkdir -p /tmp/games
fi

echo ""
echo "🎮 Setting up Songbird gaming network..."

# One-touch gaming setup
SETUP_RESPONSE=$(curl -s -X POST http://localhost:8080/api/gaming/setup \
  -H "Content-Type: application/json" \
  -d '{"setup_type": "one_touch"}' 2>/dev/null || echo "{}")

if echo "$SETUP_RESPONSE" | grep -q "success"; then
    echo "  ✅ Gaming network configured!"
else
    echo "  ⚠️  Setup may have failed, but continuing..."
fi

echo ""
echo "✅ Ready to play!"
echo ""
echo "════════════════════════════════════════"
echo "  QUICK GAME SETUP"
echo "════════════════════════════════════════"
echo ""
echo "1. Copy your game to /tmp/games/:"
echo "   mkdir -p /tmp/games/starcraft"
echo "   cp -r /path/to/StarCraft/* /tmp/games/starcraft/"
echo ""
echo "2. Launch your game:"
echo "   ./launch_game.sh /tmp/games/starcraft/StarCraft.exe"
echo ""
echo "3. Friends do the same on their computers"
echo ""
echo "4. You'll auto-discover each other!"
echo ""
echo "════════════════════════════════════════"
echo ""
echo "Supported games:"
echo "  ✅ StarCraft"
echo "  ✅ Age of Empires II"
echo "  ✅ Diablo I & II"
echo "  ✅ Quake"
echo "  ✅ Command & Conquer"
echo "  ✅ Any IPX/DirectPlay game!"
echo ""
echo "Need help?"
echo "  cat README.md"
echo ""
echo "🎮 Happy gaming! 🎉"

