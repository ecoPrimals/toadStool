#!/usr/bin/env bash
# V2 Step 1 (Alt): Start Local Songbird Only (for testing)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SONGBIRD_DIR="/home/eastgate/Development/ecoPrimals/songbird"
FEDERATION_DIR="$SONGBIRD_DIR/showcase/02-federation"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎵 V2: Start Local Songbird (Standalone)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

EASTGATE_PORT="8080"

echo -e "${BLUE}Configuration:${NC}"
echo "  Mode: Standalone (no federation)"
echo "  Port: $EASTGATE_PORT"
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

# Check if already running
if lsof -i :$EASTGATE_PORT -sTCP:LISTEN > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Port $EASTGATE_PORT already in use${NC}"
    echo
    PID=$(lsof -i :$EASTGATE_PORT -sTCP:LISTEN | tail -1 | awk '{print $2}')
    echo "Existing Songbird PID: $PID"
    echo -e "${GREEN}✅ Already running${NC}"
    exit 0
fi

# Create logs directory
mkdir -p "$FEDERATION_DIR/logs"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🚀 Starting Local Songbird...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Export environment variables
export SONGBIRD_PORT="$EASTGATE_PORT"
export SONGBIRD_NODE_ID="tower-eastgate-local"
export SONGBIRD_BIND="0.0.0.0"
export RUST_LOG="info"

LOG_FILE="$FEDERATION_DIR/logs/tower-eastgate-local.log"

echo "Starting with:"
echo "  Node ID: $SONGBIRD_NODE_ID"
echo "  Port: $SONGBIRD_PORT"
echo "  Log: $LOG_FILE"
echo

# Start Songbird
"$SONGBIRD_BIN" > "$LOG_FILE" 2>&1 &
SONGBIRD_PID=$!

echo "Process ID: $SONGBIRD_PID"
echo "Waiting for startup..."

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
    echo -e "${GREEN}✅ Songbird started!${NC}"
    echo
    
    HEALTH=$(curl -s "http://localhost:$EASTGATE_PORT/health")
    echo "Health: $HEALTH"
    echo
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${GREEN}✅ Ready for V2 Training!${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo
    echo "🎯 Next:"
    echo "  ./03-run-v2-training.sh"
    echo
    echo "📊 Monitor:"
    echo "  tail -f $LOG_FILE"
    echo
    echo "🛑 Stop:"
    echo "  kill $SONGBIRD_PID"
    echo
    
    # Save PID for later
    echo "$SONGBIRD_PID" > "$SCRIPT_DIR/.songbird.pid"
else
    echo -e "${YELLOW}❌ Failed to start${NC}"
    echo "Check: tail -50 $LOG_FILE"
    exit 1
fi

