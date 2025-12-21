#!/bin/bash
# Level 0 Quick Demo - Single Game Execution
# Tests ToadStool's ability to execute games

set -e

echo "🎮 Level 0: Single Game Execution - Quick Demo"
echo "=============================================="
echo ""

# Check if ToadStool is running
echo "📋 Pre-flight checks..."
if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "⚠️  ToadStool server not running!"
    echo ""
    echo "Please start ToadStool in another terminal:"
    echo "  cd /home/eastgate/Development/ecoPrimals/toadstool"
    echo "  cargo run --release --bin toadstool-server"
    echo ""
    exit 1
fi
echo "  ✅ ToadStool server is running"

# Create a simple test "game"
echo ""
echo "🎮 Creating test game..."
cat > /tmp/test_game.sh << 'EOF'
#!/bin/bash
echo "🎮 Test Game Starting..."
echo "⚡ Loading assets..."
sleep 1
echo "⚡ Initializing graphics..."
sleep 1
echo "⚡ Starting game loop..."
sleep 2
echo "⚡ Player moved"
sleep 1
echo "⚡ Enemy defeated"
sleep 1
echo "✅ Victory! Game Complete!"
echo "📊 Final Score: 9000"
EOF

chmod +x /tmp/test_game.sh
echo "  ✅ Test game created at /tmp/test_game.sh"

# Execute the test game via ToadStool
echo ""
echo "🚀 Launching game via ToadStool..."
echo ""

# For now, execute directly (will wire up ToadStool API later)
echo "  🎮 Executing test game..."
/tmp/test_game.sh

echo ""
echo "✅ Level 0 Demo Complete!"
echo ""
echo "What just happened:"
echo "  1. Created a simple test 'game' (shell script)"
echo "  2. Executed it (simulating ToadStool execution)"
echo "  3. Monitored output (simulating job tracking)"
echo ""
echo "Next steps:"
echo "  - Try with a real game executable"
echo "  - Add resource monitoring"
echo "  - Integrate with ToadStool API"
echo ""
echo "For your CD games:"
echo "  - Copy game executable to /tmp/"
echo "  - Replace /tmp/test_game.sh with your game"
echo "  - Run this script again!"
echo ""

