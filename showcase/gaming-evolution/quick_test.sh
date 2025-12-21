#!/bin/bash
# Quick Test - Run This Right Now!
# Tests the gaming system with downloaded games

set -e

echo "🎮 Quick Gaming Test - No CDs Needed!"
echo "====================================="
echo ""

cd "$(dirname "$0")"

# Step 1: Download test games
echo "Step 1: Downloading test games..."
echo ""
./download_test_games.sh

echo ""
echo "═══════════════════════════════════════════"
echo "  🧪 QUICK TESTS"
echo "═══════════════════════════════════════════"
echo ""

# Step 2: Test networking
echo "Step 2: Testing multiplayer networking..."
echo ""
echo "This will test if multiplayer works:"
echo "  • Server listens on port 6112"
echo "  • Client connects"
echo "  • Messages exchange"
echo ""

read -p "Press Enter to start network test..."

# Start server in background
python3 /tmp/games/test_multiplayer.py server &
SERVER_PID=$!

# Give server time to start
sleep 2

# Connect client
python3 /tmp/games/test_multiplayer.py client localhost

# Cleanup
sleep 1
kill $SERVER_PID 2>/dev/null || true

echo ""
echo "═══════════════════════════════════════════"
echo "  ✅ NETWORK TEST COMPLETE!"
echo "═══════════════════════════════════════════"
echo ""

# Step 3: Check for real games
echo "Step 3: Checking for downloaded games..."
echo ""

if [ -f "/tmp/games/quake-shareware/quake.exe" ] || [ -f "/tmp/games/quake-shareware/QUAKE.EXE" ]; then
    echo "✅ Quake shareware found!"
    echo ""
    echo "To play Quake:"
    echo "  cd lan-party-showcase"
    echo "  ./launch_game.sh /tmp/games/quake-shareware/quake.exe"
    echo ""
fi

if [ -f "/tmp/games/doom-shareware/doom.exe" ] || [ -f "/tmp/games/doom-shareware/DOOM.EXE" ]; then
    echo "✅ Doom shareware found!"
    echo ""
    echo "To play Doom:"
    echo "  cd lan-party-showcase"
    echo "  ./launch_game.sh /tmp/games/doom-shareware/doom.exe"
    echo ""
fi

# Summary
echo "═══════════════════════════════════════════"
echo "  🎉 READY TO PLAY!"
echo "═══════════════════════════════════════════"
echo ""
echo "What just happened:"
echo "  ✅ Downloaded legal test games"
echo "  ✅ Tested multiplayer networking"
echo "  ✅ Verified system works"
echo ""
echo "Next steps:"
echo "  1. Start Songbird:"
echo "     cd ../../../songbird"
echo "     cargo run --release --bin songbird-orchestrator"
echo ""
echo "  2. Setup gaming network:"
echo "     cd lan-party-showcase"
echo "     ./quick_start.sh"
echo ""
echo "  3. Launch a game:"
echo "     ./launch_game.sh /tmp/games/quake-shareware/quake.exe"
echo ""
echo "  4. In game, choose Multiplayer → LAN"
echo ""
echo "  5. PLAY! 🎮"
echo ""
echo "Games ready:"
ls -1 /tmp/games/ | grep -v "\.py" | while read game; do
    echo "  • $game"
done
echo ""
echo "Have fun! 🎉"

