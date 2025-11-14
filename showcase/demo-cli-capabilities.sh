#!/bin/bash
# ToadStool v0.1.0 Beta - CLI Capabilities Demo
# This script demonstrates what ACTUALLY works in the beta release

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

TOADSTOOL_CLI="../target/release/toadstool-cli"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     🍄 ToadStool v0.1.0 Beta - CLI Capabilities Demo     ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if binary exists
if [ ! -f "$TOADSTOOL_CLI" ]; then
    echo -e "${YELLOW}⚠️  ToadStool CLI not found. Building...${NC}"
    cd .. && cargo build --release --bin toadstool-cli && cd showcase
fi

echo -e "${GREEN}✅ Binary located: $TOADSTOOL_CLI${NC}"
echo ""

# Demo 1: Version
echo -e "${BLUE}═══ Demo 1: Version Check ═══${NC}"
$TOADSTOOL_CLI --version
echo ""
sleep 1

# Demo 2: Help
echo -e "${BLUE}═══ Demo 2: Available Commands ═══${NC}"
echo "Running: toadstool-cli --help"
$TOADSTOOL_CLI --help 2>&1 | head -25
echo "... (14 commands total)"
echo ""
sleep 1

# Demo 3: Capabilities
echo -e "${BLUE}═══ Demo 3: System Capabilities ═══${NC}"
echo "Running: toadstool-cli capabilities"
echo "(This shows what runtimes your system supports)"
$TOADSTOOL_CLI capabilities 2>&1 | head -20
echo ""
sleep 1

# Demo 4: Init
echo -e "${BLUE}═══ Demo 4: Generate Biome Manifest ═══${NC}"
echo "Running: toadstool-cli init --template basic /tmp/demo-biome"
$TOADSTOOL_CLI init --template basic /tmp/demo-biome 2>&1 | grep -v "^{" | head -15
echo ""
echo -e "${GREEN}✅ Generated manifest at: /tmp/demo-biome/biome.yaml${NC}"
echo ""
sleep 1

# Demo 5: Validate
echo -e "${BLUE}═══ Demo 5: Validate Manifest ═══${NC}"
echo "Running: toadstool-cli validate /tmp/demo-biome/biome.yaml"
$TOADSTOOL_CLI validate /tmp/demo-biome/biome.yaml 2>&1 | grep -A 10 "Manifest validation"
echo ""
sleep 1

# Demo 6: CLI-generated example
echo -e "${BLUE}═══ Demo 6: Validate Showcase Example ═══${NC}"
if [ -f "biomes/cli-generated-basic.yaml" ]; then
    echo "Running: toadstool-cli validate biomes/cli-generated-basic.yaml"
    $TOADSTOOL_CLI validate biomes/cli-generated-basic.yaml 2>&1 | grep -A 10 "Manifest validation"
else
    echo -e "${YELLOW}⚠️  CLI-generated example not found${NC}"
fi
echo ""

# Summary
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      Demo Complete!                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}What you just saw:${NC}"
echo "  ✅ Binary execution (version, help)"
echo "  ✅ Capability detection (system introspection)"
echo "  ✅ Template generation (init command)"
echo "  ✅ Manifest validation (validate command)"
echo ""
echo -e "${YELLOW}What requires server (coming in v1.0):${NC}"
echo "  🔄 Runtime execution (run, up, down)"
echo "  🔄 Service orchestration (ps, logs)"
echo "  🔄 Live migration"
echo "  🔄 Distributed computing"
echo ""
echo -e "${BLUE}ToadStool v0.1.0 Beta: Solid CLI foundation! 🍄${NC}"
echo ""

# Cleanup
rm -rf /tmp/demo-biome

echo "Demo files cleaned up."
echo "Check out the generated manifest format by running:"
echo "  $TOADSTOOL_CLI init --template basic ~/my-biome"
echo ""

