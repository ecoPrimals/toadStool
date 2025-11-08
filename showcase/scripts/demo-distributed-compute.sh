#!/bin/bash
# ToadStool Showcase - Distributed Compute Demo
# Demonstrates real job splitting and parallel subtask execution

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

# Path to showcase root
SHOWCASE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOADSTOOL_ROOT="$(cd "$SHOWCASE_ROOT/.." && pwd)"
DEMO_BIN="$TOADSTOOL_ROOT/target/release/toadstool-showcase-distributed"

# Check if demo binary exists, build if needed
if [ ! -f "$DEMO_BIN" ]; then
    echo -e "${YELLOW}Building distributed compute demo...${NC}"
    echo ""
    (cd "$TOADSTOOL_ROOT" && cargo build --release --bin toadstool-showcase-distributed)
    echo ""
fi

clear

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║    🍄 ToadStool Distributed Compute Demonstration        ║"
echo "║         Real Job Splitting & Parallel Execution          ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BOLD}${BLUE}This demo showcases ToadStool's distributed compute capabilities:${NC}"
echo ""
echo "  1. ✅ Automatic job analysis"
echo "  2. ✅ Intelligent subtask creation"
echo "  3. ✅ Parallel execution"
echo "  4. ✅ Results aggregation"
echo "  5. ✅ Performance metrics"
echo ""

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

read -p "$(echo -e ${YELLOW}Press ENTER to start the demonstration...${NC})"

echo ""
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}   RUNNING DISTRIBUTED COMPUTE DEMO    ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
echo ""

# Run the demo
"$DEMO_BIN"

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${GREEN}${BOLD}Demo Complete!${NC}"
echo ""
echo -e "${BLUE}What You Just Saw:${NC}"
echo "  ✓ Single task baseline execution"
echo "  ✓ Distributed execution with 10 parallel subtasks"
echo "  ✓ Performance comparison showing speedup"
echo ""

echo -e "${YELLOW}Try These Next:${NC}"
echo "  • Run individual workloads:"
echo "    ${CYAN}toadstool-cli execute workloads/distributed-data-processing.toml${NC}"
echo "    ${CYAN}toadstool-cli execute workloads/distributed-map-reduce.toml${NC}"
echo "    ${CYAN}toadstool-cli execute workloads/distributed-parallel-search.toml${NC}"
echo ""
echo "  • Run other showcase demos:"
echo "    ${CYAN}./scripts/demo-hello.sh${NC}"
echo "    ${CYAN}./scripts/demo-benchmark.sh${NC}"
echo ""

echo -e "${MAGENTA}🍄 ToadStool - Universal Compute Platform${NC}"
echo ""

exit 0

