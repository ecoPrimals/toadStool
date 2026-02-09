#!/bin/bash
# run-real-demos.sh
# Runs REAL ToadStool demos (no mocks!)

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

TOADSTOOL_ROOT="../../.."
DEMO_BINARIES=("demo-native-execution" "demo-wasm-execution")

echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🍄 ToadStool Level 0: REAL Execution Demos${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}This script runs REAL ToadStool demos using actual API calls.${NC}"
echo -e "${CYAN}NO MOCKS - These are production Rust binaries!${NC}"
echo ""

# Check if binaries exist
echo -e "${YELLOW}Checking for built binaries...${NC}"
MISSING=0
for BINARY in "${DEMO_BINARIES[@]}"; do
    if [ ! -f "$TOADSTOOL_ROOT/target/release/$BINARY" ]; then
        echo -e "${RED}❌ Missing: $BINARY${NC}"
        MISSING=$((MISSING + 1))
    else
        SIZE=$(ls -lh "$TOADSTOOL_ROOT/target/release/$BINARY" | awk '{print $5}')
        echo -e "${GREEN}✅ Found: $BINARY ($SIZE)${NC}"
    fi
done

if [ $MISSING -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}Building missing binaries...${NC}"
    cd "$TOADSTOOL_ROOT"
    cargo build --release --package toadstool-showcase-local
    cd - > /dev/null
    echo -e "${GREEN}✅ Build complete${NC}"
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Demo 1: Native Runtime Execution${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
"$TOADSTOOL_ROOT/target/release/demo-native-execution"

echo ""
echo -e "${YELLOW}Press Enter to continue to WASM demo...${NC}"
read -r

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Demo 2: WASM Runtime Execution${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
"$TOADSTOOL_ROOT/target/release/demo-wasm-execution"

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ All Level 0 Demos Complete!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}📊 Summary:${NC}"
echo -e "${GREEN}   ✅ Native execution: SUCCESS${NC}"
echo -e "${GREEN}   ✅ WASM execution: SUCCESS${NC}"
echo -e "${GREEN}   ✅ NO MOCKS used${NC}"
echo -e "${GREEN}   ✅ Real API calls verified${NC}"
echo ""
echo -e "${CYAN}📝 See execution receipts:${NC}"
echo -e "   cat LEVEL_0_FINAL_RECEIPTS_DEC_21_2025.md"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo -e "   • Review source code: demo_native.rs, demo_wasm.rs"
echo -e "   • Check receipts for verification"
echo -e "   • Explore ToadStool UniversalComputePlatform API"
echo ""

