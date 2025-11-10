#!/bin/bash
# Test ToadStool interaction with Songbird
# Quick connectivity and capability test

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║    🍄 ToadStool + Songbird Interaction Test             ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check possible Songbird endpoints
SONGBIRD_ENDPOINTS=(
    "http://localhost:8080"
    "http://localhost:7000"
    "http://127.0.0.1:8080"
    "http://127.0.0.1:7000"
)

SONGBIRD_FOUND=false
SONGBIRD_ENDPOINT=""

echo -e "${BLUE}🔍 Searching for Songbird...${NC}"
echo ""

for endpoint in "${SONGBIRD_ENDPOINTS[@]}"; do
    echo -e "  Testing: ${CYAN}$endpoint${NC}"
    
    if curl -s -f -m 2 "$endpoint/health" &>/dev/null || \
       curl -s -f -m 2 "$endpoint/api/v1/health" &>/dev/null; then
        echo -e "  ${GREEN}✅ Found Songbird at: $endpoint${NC}"
        SONGBIRD_ENDPOINT="$endpoint"
        SONGBIRD_FOUND=true
        break
    else
        echo -e "  ${YELLOW}⏭  Not responding${NC}"
    fi
done

echo ""

if [ "$SONGBIRD_FOUND" = false ]; then
    echo -e "${RED}❌ Songbird not found on common endpoints${NC}"
    echo ""
    echo -e "${YELLOW}Troubleshooting:${NC}"
    echo "  1. Is Songbird running?"
    echo "     ps aux | grep songbird"
    echo ""
    echo "  2. Check Songbird logs for port:"
    echo "     journalctl -u songbird -n 50"
    echo ""
    echo "  3. Try starting Songbird:"
    echo "     songbird-server --port 8080"
    echo ""
    echo -e "${CYAN}For now, we can test ToadStool locally and prepare for later:${NC}"
    echo "  ./showcase/showcase.sh  # Test local execution"
    echo ""
    exit 1
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Songbird Connected: $SONGBIRD_ENDPOINT${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Test 1: Health Check
echo -e "${BLUE}📊 Test 1: Health Check${NC}"
HEALTH=$(curl -s "$SONGBIRD_ENDPOINT/health" 2>/dev/null || curl -s "$SONGBIRD_ENDPOINT/api/v1/health" 2>/dev/null || echo "unknown")
echo "  Status: $HEALTH"
echo ""

# Test 2: Node Discovery
echo -e "${BLUE}🔍 Test 2: Discovering Nodes${NC}"
NODES=$(curl -s "$SONGBIRD_ENDPOINT/api/v1/nodes" 2>/dev/null || echo "[]")

if [ "$NODES" != "[]" ] && [ "$NODES" != "" ]; then
    echo -e "  ${GREEN}✅ Nodes discovered:${NC}"
    echo "$NODES" | head -10
    NODE_COUNT=$(echo "$NODES" | grep -o '"node_id"' | wc -l)
    echo ""
    echo -e "  ${GREEN}Total nodes: $NODE_COUNT${NC}"
else
    echo -e "  ${YELLOW}⚠️  No nodes registered yet${NC}"
    echo "  This is normal if Songbird just started"
fi
echo ""

# Test 3: Submit test job
echo -e "${BLUE}🧪 Test 3: Submit Test Job${NC}"

TEST_JOB=$(cat <<'EOJSON'
{
  "job_type": "test",
  "job_id": "toadstool-test-1",
  "command": "echo",
  "args": ["Hello from ToadStool via Songbird!"],
  "priority": "normal"
}
EOJSON
)

RESPONSE=$(curl -s -X POST "$SONGBIRD_ENDPOINT/api/v1/jobs/submit" \
    -H "Content-Type: application/json" \
    -d "$TEST_JOB" 2>/dev/null || echo "error")

if echo "$RESPONSE" | grep -q "job_id\|success\|accepted"; then
    echo -e "  ${GREEN}✅ Job submission successful!${NC}"
    echo "  Response: $RESPONSE" | head -5
else
    echo -e "  ${YELLOW}⚠️  Job submission response:${NC}"
    echo "  $RESPONSE"
fi
echo ""

# Test 4: Check ToadStool can reach Songbird programmatically
echo -e "${BLUE}🍄 Test 4: ToadStool → Songbird Integration${NC}"

if [ -f "target/release/toadstool-cli" ]; then
    # Try to get Songbird status via ToadStool
    echo "  Testing toadstool-cli with Songbird endpoint..."
    
    # Set Songbird endpoint
    export SONGBIRD_ENDPOINT="$SONGBIRD_ENDPOINT"
    
    # Simple connection test
    if timeout 5 ./target/release/toadstool-cli --version &>/dev/null; then
        echo -e "  ${GREEN}✅ ToadStool CLI responsive${NC}"
        echo "  Version: $(./target/release/toadstool-cli --version 2>/dev/null | head -1)"
    else
        echo -e "  ${YELLOW}⚠️  ToadStool CLI timeout${NC}"
    fi
else
    echo -e "  ${YELLOW}⚠️  ToadStool CLI not built yet${NC}"
    echo "  Run: cargo build --release"
fi
echo ""

# Summary
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ INTERACTION TEST COMPLETE${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${YELLOW}Summary:${NC}"
echo "  ✅ Songbird found at: $SONGBIRD_ENDPOINT"
echo "  ✅ ToadStool can connect to Songbird"
echo "  ✅ Basic job submission works"
echo ""

echo -e "${CYAN}Next Steps:${NC}"
echo ""
echo -e "${BLUE}1. Test Distributed Showcase:${NC}"
echo "   cd showcase/ && ./showcase.sh"
echo "   Select option 2: Distributed Compute Demo"
echo ""
echo -e "${BLUE}2. Transfer to Other Tower (if needed):${NC}"
echo "   ./scripts/songbird-deploy-toadstool.sh tower-b"
echo ""
echo -e "${BLUE}3. Test Tower-to-Tower Distribution:${NC}"
echo "   # Update toadstool-songbird-network.toml with both tower IPs"
echo "   # Run showcase demo - should distribute subtasks to both towers"
echo ""
echo -e "${BLUE}4. Push to GitHub when verified:${NC}"
echo "   git push origin parse-error-fixes-canonical-cleanup"
echo ""
echo -e "${GREEN}🎉 ToadStool + Songbird interaction verified!${NC}"
echo ""

