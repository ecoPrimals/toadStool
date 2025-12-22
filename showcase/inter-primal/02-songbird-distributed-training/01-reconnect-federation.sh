#!/usr/bin/env bash
# Step 1: Reconnect Eastgate to Strandgate Federation

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📡 Step 1: Reconnect Eastgate to Federation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

STRANDGATE="https://192.168.1.134:8081"
EASTGATE="http://192.168.1.144:8080"

echo -e "${BLUE}Federation Configuration:${NC}"
echo "  • Tower A (Eastgate): 192.168.1.144:8080 (Local, RTX 2070)"
echo "  • Tower B (Strandgate): 192.168.1.134:8081 (Remote, Dual EPYC + RTX 3070)"
echo

# Check Strandgate
echo -e "${BLUE}Checking Strandgate (Tower B)...${NC}"
if curl -sk "${STRANDGATE}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Strandgate is online${NC}"
else
    echo -e "${RED}❌ Strandgate is unreachable${NC}"
    exit 1
fi
echo

# Start local Songbird if not running
echo -e "${BLUE}Checking Eastgate Songbird (Tower A)...${NC}"
if ! curl -s "${EASTGATE}/health" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Eastgate Songbird not running, starting...${NC}"
    
    cd ../../../../songbird
    
    # Start Songbird orchestrator
    cargo run --release --bin songbird-orchestrator -- \
        --port 8080 \
        --node-id "tower-a-eastgate" \
        --node-name "Eastgate" \
        > /tmp/songbird-eastgate.log 2>&1 &
    
    SONGBIRD_PID=$!
    echo "  Started Songbird (PID: $SONGBIRD_PID)"
    echo "  Log: /tmp/songbird-eastgate.log"
    
    # Wait for startup
    echo "  Waiting for Songbird to start..."
    for i in {1..30}; do
        if curl -s "${EASTGATE}/health" > /dev/null 2>&1; then
            echo -e "${GREEN}  ✅ Songbird started${NC}"
            break
        fi
        sleep 1
    done
    
    cd - > /dev/null
else
    echo -e "${GREEN}✅ Eastgate Songbird already running${NC}"
fi
echo

# Join federation
echo -e "${BLUE}Joining Federation...${NC}"
echo

FEDERATION_PAYLOAD=$(cat <<EOF
{
  "node_id": "tower-a-eastgate",
  "node_name": "Eastgate",
  "node_address": "192.168.1.144:8080",
  "cpu_cores": 24,
  "memory_gb": 32,
  "capabilities": ["compute", "universal-ml", "gpu-rtx-2070"],
  "metadata": {
    "gpu": "NVIDIA RTX 2070",
    "gpu_memory_gb": 8,
    "location": "eastgate",
    "cpu": "Intel i9-12900"
  }
}
EOF
)

echo "Sending federation join request to Strandgate..."
FEDERATION_RESPONSE=$(curl -sk -X POST "${STRANDGATE}/api/federation/join" \
    -H "Content-Type: application/json" \
    -d "$FEDERATION_PAYLOAD" 2>&1)

echo "Response:"
echo "$FEDERATION_RESPONSE" | jq '.' 2>/dev/null || echo "$FEDERATION_RESPONSE"
echo

if echo "$FEDERATION_RESPONSE" | grep -q "success\|joined\|ok"; then
    echo -e "${GREEN}✅ Successfully joined federation!${NC}"
else
    echo -e "${YELLOW}⚠️  Response received (federation may already be established)${NC}"
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Federation Setup Complete!${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Tower A (Eastgate): ${EASTGATE}"
echo "Tower B (Strandgate): ${STRANDGATE}"
echo
echo "Next: Run distributed training"
echo "  ./02-run-distributed-training.sh"
echo

