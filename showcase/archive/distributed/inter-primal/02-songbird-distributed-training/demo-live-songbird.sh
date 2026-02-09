#!/usr/bin/env bash
#
# ToadStool + Live Songbird: Distributed Training Demo
# NO MOCKS - Uses real Songbird federation
#
# Usage: ./demo-live-songbird.sh

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  🍄🎵 ToadStool + Songbird: Live Distributed Training${NC}"
echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Check Songbird is running
echo -e "${BLUE}Step 1: Checking Songbird Federation${NC}"
echo -e "${CYAN}   Testing Eastgate tower...${NC}"

if curl -sk https://localhost:8000/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Eastgate online (https://localhost:8000)${NC}"
    EASTGATE_UP=true
else
    echo -e "${YELLOW}⚠️  Eastgate offline${NC}"
    echo -e "${CYAN}   To start: cd /home/eastgate/Development/ecoPrimals/songbird/showcase/06-toadstool-ml-orchestration${NC}"
    echo -e "${CYAN}             ./START_FEDERATION.sh${NC}"
    EASTGATE_UP=false
fi

echo -e "${CYAN}   Testing Strandgate tower...${NC}"
if curl -sk https://192.168.1.134:8081/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Strandgate online (https://192.168.1.134:8081)${NC}"
    STRANDGATE_UP=true
else
    echo -e "${YELLOW}⚠️  Strandgate offline (optional)${NC}"
    STRANDGATE_UP=false
fi

if [ "$EASTGATE_UP" = false ]; then
    echo ""
    echo -e "${YELLOW}⚠️  Songbird federation not running${NC}"
    echo -e "${CYAN}   This demo requires at least Eastgate tower${NC}"
    exit 1
fi

echo ""

# Step 2: Run ToadStool with Songbird
echo -e "${BLUE}Step 2: Running ToadStool Distributed Training${NC}"
echo -e "${CYAN}   Connecting to: https://localhost:8000${NC}"
echo -e "${CYAN}   Mode: Distributed MNIST training${NC}"
echo ""

# Build if needed
if [ ! -f target/release/distributed-train ]; then
    echo -e "${CYAN}   Building ToadStool showcase...${NC}"
    cargo build --release
    echo ""
fi

# Run with Songbird
echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}Starting ToadStool + Songbird Integration...${NC}"
echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

./target/release/distributed-train \
    --songbird-url https://localhost:8000 \
    --epochs 2 \
    --data-dir ../../gpu-universal/ml-inference/data/mnist

EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✅ Demo Complete!${NC}"
else
    echo -e "${YELLOW}⚠️  Demo completed with warnings (code: $EXIT_CODE)${NC}"
fi

echo ""
echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Summary:${NC}"
echo -e "  🎵 Songbird Federation: ${EASTGATE_UP:+✅}${EASTGATE_UP:-❌} Eastgate, ${STRANDGATE_UP:+✅}${STRANDGATE_UP:-⚠️} Strandgate"
echo -e "  🍄 ToadStool Training: ${EXIT_CODE:+✅}${EXIT_CODE:-❌} Complete"
echo -e "  🔗 Integration: Live (no mocks)"
echo ""
echo -e "${BLUE}🎉 ToadStool + Songbird distributed training working!${NC}"
echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
echo ""

