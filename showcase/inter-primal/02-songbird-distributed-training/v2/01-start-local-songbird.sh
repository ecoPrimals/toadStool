#!/usr/bin/env bash
# V2 Step 1: Start Local Songbird and Connect to Federation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SONGBIRD_DIR="/home/eastgate/Development/ecoPrimals/songbird"
FEDERATION_DIR="$SONGBIRD_DIR/showcase/02-federation"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎵 V2 Step 1: Start Local Songbird & Connect to Federation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Configuration
EASTGATE_IP="192.168.1.144"
STRANDGATE_IP="192.168.1.134"
STRANDGATE_PORT="8081"
EASTGATE_PORT="8080"

echo -e "${BLUE}Configuration:${NC}"
echo "  Eastgate (Local):  $EASTGATE_IP:$EASTGATE_PORT"
echo "  Strandgate (Remote): $STRANDGATE_IP:$STRANDGATE_PORT"
echo

# Check if Songbird binary exists
SONGBIRD_BIN="$SONGBIRD_DIR/target/release/songbird-orchestrator"
if [ ! -f "$SONGBIRD_BIN" ]; then
    echo -e "${YELLOW}Building Songbird...${NC}"
    cd "$SONGBIRD_DIR"
    cargo build --release --bin songbird-orchestrator
    echo -e "${GREEN}✅ Build complete${NC}"
    echo
fi

# Check if Strandgate is reachable
echo -e "${BLUE}Checking Strandgate connectivity...${NC}"
if curl -sk "https://$STRANDGATE_IP:$STRANDGATE_PORT/health" -m 5 2>/dev/null | grep -q "ok\|healthy"; then
    echo -e "${GREEN}✅ Strandgate is online${NC}"
else
    echo -e "${RED}❌ Cannot reach Strandgate at https://$STRANDGATE_IP:$STRANDGATE_PORT${NC}"
    echo
    echo "Please ensure:"
    echo "  1. Strandgate's Songbird is running"
    echo "  2. Network connectivity between machines"
    echo "  3. Firewall allows port $STRANDGATE_PORT"
    exit 1
fi

# Check current federation status
echo
echo -e "${BLUE}Current federation status on Strandgate:${NC}"
FEDERATION_STATUS=$(curl -sk "https://$STRANDGATE_IP:$STRANDGATE_PORT/api/federation/status" 2>/dev/null)
echo "$FEDERATION_STATUS" | jq '.' 2>/dev/null || echo "$FEDERATION_STATUS"

ACTIVE_NODES=$(echo "$FEDERATION_STATUS" | jq -r '.active_nodes // 0' 2>/dev/null)
echo
echo "Active nodes in federation: $ACTIVE_NODES"
echo

# Check if Eastgate's Songbird is already running
if lsof -i :$EASTGATE_PORT -sTCP:LISTEN > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Port $EASTGATE_PORT already in use${NC}"
    echo
    read -p "Kill existing Songbird and restart? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        PID=$(lsof -i :$EASTGATE_PORT -sTCP:LISTEN | tail -1 | awk '{print $2}')
        echo "Killing PID $PID..."
        kill $PID 2>/dev/null || true
        sleep 3
    else
        echo "Using existing Songbird instance"
        echo -e "${GREEN}✅ Local Songbird already running${NC}"
        exit 0
    fi
fi

# Create logs directory
mkdir -p "$FEDERATION_DIR/logs"

# Start local Songbird with federation connection
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🚀 Starting Eastgate Songbird...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Export environment variables
export SONGBIRD_PORT="$EASTGATE_PORT"
export SONGBIRD_NODE_ID="tower-a-eastgate"
export SONGBIRD_FEDERATION="true"
export SONGBIRD_PEERS="$STRANDGATE_IP:$STRANDGATE_PORT"
export SONGBIRD_BIND="0.0.0.0"
export RUST_LOG="info"

LOG_FILE="$FEDERATION_DIR/logs/tower-a-eastgate-v2.log"

echo "Starting Songbird with:"
echo "  Node ID: $SONGBIRD_NODE_ID"
echo "  Port: $SONGBIRD_PORT"
echo "  Federation: enabled"
echo "  Peer: $SONGBIRD_PEERS"
echo "  Log: $LOG_FILE"
echo

# Start Songbird
"$SONGBIRD_BIN" > "$LOG_FILE" 2>&1 &
SONGBIRD_PID=$!

echo "Process ID: $SONGBIRD_PID"
echo "Waiting for startup..."
echo

# Wait for health check
STARTED=false
for i in {1..30}; do
    sleep 1
    if curl -s "http://localhost:$EASTGATE_PORT/health" -m 2 > /dev/null 2>&1; then
        STARTED=true
        break
    fi
    printf "."
done
echo

if [ "$STARTED" = true ]; then
    echo -e "${GREEN}✅ Eastgate Songbird started!${NC}"
    echo
    
    # Check health
    HEALTH=$(curl -s "http://localhost:$EASTGATE_PORT/health")
    echo "Health: $HEALTH"
    echo
    
    # Wait a moment for federation to sync
    echo "Waiting for federation sync..."
    sleep 3
    
    # Check updated federation status
    echo
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${BLUE}Updated Federation Status:${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    UPDATED_STATUS=$(curl -sk "https://$STRANDGATE_IP:$STRANDGATE_PORT/api/federation/status" 2>/dev/null)
    echo "$UPDATED_STATUS" | jq '.' 2>/dev/null || echo "$UPDATED_STATUS"
    
    NEW_ACTIVE_NODES=$(echo "$UPDATED_STATUS" | jq -r '.active_nodes // 0' 2>/dev/null)
    echo
    echo "Active nodes now: $NEW_ACTIVE_NODES"
    
    if [ "$NEW_ACTIVE_NODES" -gt "$ACTIVE_NODES" ]; then
        echo -e "${GREEN}✅ Successfully joined federation!${NC}"
    else
        echo -e "${YELLOW}⚠️  Federation may need manual registration${NC}"
        echo
        echo "Try registering manually:"
        echo "  curl -sk -X POST https://$STRANDGATE_IP:$STRANDGATE_PORT/api/federation/join \\"
        echo "    -H 'Content-Type: application/json' \\"
        echo "    -d '{\"node_id\":\"tower-a-eastgate\",\"node_address\":\"http://$EASTGATE_IP:$EASTGATE_PORT\",\"cpu_cores\":16,\"memory_gb\":32}'"
    fi
    
    echo
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${GREEN}✅ V2 Step 1 Complete!${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo
    echo "🎯 Next Steps:"
    echo "  1. Verify federation: ./02-verify-federation.sh"
    echo "  2. Run V2 training: ./03-run-v2-training.sh"
    echo
    echo "📊 Monitor:"
    echo "  tail -f $LOG_FILE"
    echo
    echo "🛑 To stop:"
    echo "  kill $SONGBIRD_PID"
    echo
    
else
    echo -e "${RED}❌ Failed to start Songbird${NC}"
    echo
    echo "Check logs: tail -50 $LOG_FILE"
    exit 1
fi

