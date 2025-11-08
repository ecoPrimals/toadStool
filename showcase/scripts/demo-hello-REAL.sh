#!/bin/bash
# ToadStool Showcase - Multi-Substrate Hello Demo (USING REAL TOADSTOOL)

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Path to toadstool-cli (from showcase/scripts/ to toadstool root)
TOADSTOOL_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOADSTOOL_CLI="$TOADSTOOL_ROOT/target/release/toadstool-cli"

if [ ! -f "$TOADSTOOL_CLI" ]; then
    echo -e "${YELLOW}Building toadstool-cli...${NC}"
    (cd "$TOADSTOOL_ROOT" && cargo build --release --bin toadstool-cli)
fi

# Suppress security warning for demo
export TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED=1

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     🍄 ToadStool Multi-Substrate Demo (REAL ENGINE)      ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BLUE}This demo uses ToadStool's REAL universal compute engine.${NC}"
echo "Watch as ToadStool detects and utilizes actual substrates."
echo ""

# Step 1: Detect available substrates
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${YELLOW}Step 1: Detecting Universal Compute Substrates${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

$TOADSTOOL_CLI universal detect

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Real Substrate Detection Complete!${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo -e "${BLUE}Key Takeaways:${NC}"
echo "  • ToadStool ACTUALLY detected your system's substrates"
echo "  • Docker, Python, Native - all REAL detections"
echo "  • This is not a simulation - it's LIVE substrate scanning"
echo ""

echo -e "${YELLOW}💡 Next: Run 'demo-benchmark-REAL.sh' to benchmark these substrates${NC}"
echo ""

