#!/bin/bash
# run_via_toadstool.sh
# Level 1: Launch game server via ToadStool orchestration

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║    🍄 Level 1: ToadStool-Orchestrated Game Server 🍄        ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

echo "This demonstrates:"
echo "  ✅ ToadStool workload orchestration"
echo "  ✅ Resource management"
echo "  ✅ Health monitoring"
echo "  ✅ Auto-restart on failure"
echo "  ✅ Proper process lifecycle"
echo ""

# Check if ToadStool is available
if ! command -v toadstool &> /dev/null; then
    echo "❌ ToadStool CLI not found"
    echo ""
    echo "Install ToadStool CLI:"
    echo "  cd ../../.."
    echo "  cargo install --path crates/cli"
    echo ""
    echo "Or use cargo directly:"
    echo "  cargo run --bin toadstool -- submit biomes/game-server-openarena.yaml"
    exit 1
fi

echo "✅ ToadStool CLI found"
echo ""

# Ensure server config exists
if [ ! -f ~/.openarena/baseoa/server.cfg ]; then
    echo "📝 Creating server configuration..."
    ../../fix_server_config.sh
fi

echo "🚀 Submitting workload to ToadStool..."
echo ""

# Submit the biome
RESULT=$(toadstool submit biomes/game-server-openarena.yaml 2>&1)

if echo "$RESULT" | grep -q "error\|Error\|failed\|Failed"; then
    echo "❌ Failed to submit workload"
    echo ""
    echo "$RESULT"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check if ToadStool server is running:"
    echo "     cargo run --bin toadstool-server"
    echo ""
    echo "  2. Verify biome.yaml syntax:"
    echo "     cat biomes/game-server-openarena.yaml"
    echo ""
    echo "  3. Check ToadStool logs"
    exit 1
fi

echo "✅ Workload submitted!"
echo ""
echo "$RESULT"
echo ""

# Extract workload ID if available
WORKLOAD_ID=$(echo "$RESULT" | grep -oP 'workload[_-]id[":]*\s*\K[a-f0-9-]+' || echo "")

if [ -n "$WORKLOAD_ID" ]; then
    echo "📊 Workload ID: $WORKLOAD_ID"
    echo ""
    echo "Monitor status:"
    echo "  toadstool status $WORKLOAD_ID"
    echo ""
    echo "View logs:"
    echo "  toadstool logs $WORKLOAD_ID"
    echo ""
    echo "Stop server:"
    echo "  toadstool stop $WORKLOAD_ID"
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  🎊 Server running via ToadStool! 🎊"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "What ToadStool is doing:"
echo "  • Managing process lifecycle"
echo "  • Allocating resources (2 cores, 512MB RAM)"
echo "  • Monitoring health (UDP port 27960)"
echo "  • Auto-restarting on failure"
echo "  • Collecting metrics"
echo ""
echo "Connect to server:"
echo "  openarena +connect $(hostname -I | awk '{print $1}'):27960"
echo ""
echo "Or use the join script:"
echo "  cd .."
echo "  ./join_lan_server.sh $(hostname -I | awk '{print $1}')"

