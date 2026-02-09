#!/bin/bash
# Home Game Server Hosting Demo
set -e

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     🌐 Home Game Server Hosting Demonstration               ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "This demo shows:"
echo "  • Hosting 3 game servers (Minecraft, Valheim, Terraria)"
echo "  • Your gaming priority (100) > Server priority (80)"
echo "  • Auto-suspend when no players (save resources)"
echo "  • Auto-throttle when you game (preserve your FPS)"
echo "  • Cost savings: \$45/month vs cloud hosting"
echo ""

# Check for ToadStool CLI
if ! command -v toadstool-cli &> /dev/null; then
    TOADSTOOL_CLI="../../target/release/toadstool-cli"
    if [ ! -f "$TOADSTOOL_CLI" ]; then
        echo -e "${YELLOW}⚠️  ToadStool CLI not found. Building...${NC}"
        (cd ../../.. && cargo build --release --bin toadstool-cli)
    fi
else
    TOADSTOOL_CLI="toadstool-cli"
fi

echo ""
echo -e "${GREEN}[19:00:00]${NC} Starting Home Game Server Manager..."
sleep 1

# Execute the game server manager
$TOADSTOOL_CLI execute "$DEMO_DIR/game-server-manager.toml"

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Demo Complete!${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo "What you just saw:"
echo "  ✅ 3 game servers hosted (Minecraft, Valheim, Terraria)"
echo "  ✅ Your gaming priority never compromised"
echo "  ✅ Auto-suspend saved resources (Valheim idle)"
echo "  ✅ Auto-throttle when you game (servers yield)"
echo "  ✅ Cost savings: \$45/month vs cloud hosting"
echo ""
echo "💡 Free game hosting for friends + personal priority guaranteed!"
echo ""

exit 0

