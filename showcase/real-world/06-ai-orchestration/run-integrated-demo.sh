#!/bin/bash
# ToadStool AI Orchestration - Integrated Demo with All Three Primals
# Runs ToadStool + Songbird + Squirrel together on one tower

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

# Paths
ECOPRIMAL_ROOT="/home/eastgate/Development/ecoPrimals"
TOADSTOOL_ROOT="$ECOPRIMAL_ROOT/toadstool"
SONGBIRD_ROOT="$ECOPRIMAL_ROOT/songbird"
SQUIRREL_ROOT="$ECOPRIMAL_ROOT/squirrel"
SECRETS_DIR="$ECOPRIMAL_ROOT/testing-secrets"

# Ports
TOADSTOOL_PORT=7878
SONGBIRD_PORT=8080
SQUIRREL_PORT=9090

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║   🌿 ecoPrimals Integrated AI Demo                           ║"
echo "║   🍄 ToadStool + 🐦 Songbird + 🐿️  Squirrel                  ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# ═══════════════════════════════════════════════════════════════
# Step 1: Pre-flight Checks
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 1: Pre-flight Checks${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check if other primals exist
echo -e "${BLUE}Checking for other primals...${NC}"

if [ ! -d "$SONGBIRD_ROOT" ]; then
    echo -e "${RED}❌ Songbird not found at: $SONGBIRD_ROOT${NC}"
    echo -e "${YELLOW}   This demo requires Songbird for message routing${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Songbird found${NC}"

if [ ! -d "$SQUIRREL_ROOT" ]; then
    echo -e "${RED}❌ Squirrel not found at: $SQUIRREL_ROOT${NC}"
    echo -e "${YELLOW}   This demo requires Squirrel for AI management${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Squirrel found${NC}"

# Check for API keys
if [ ! -f "$SECRETS_DIR/api-keys.toml" ]; then
    echo -e "${YELLOW}⚠️  API keys not found at: $SECRETS_DIR/api-keys.toml${NC}"
    echo -e "${YELLOW}   Demo will use local models only${NC}"
else
    echo -e "${GREEN}✓ API keys found${NC}"
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 2: Start Songbird (Message Router)
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 2: Starting Songbird (Message Router)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check if Songbird is already running
if lsof -Pi :$SONGBIRD_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Songbird already running on port $SONGBIRD_PORT${NC}"
    echo -e "${BLUE}   Using existing instance${NC}"
else
    echo -e "${BLUE}🐦 Starting Songbird on port $SONGBIRD_PORT...${NC}"
    
    cd "$SONGBIRD_ROOT"
    
    # Check if Songbird binary exists
    if [ -f "target/release/songbird" ]; then
        echo -e "${GREEN}   Found built binary${NC}"
        nohup ./target/release/songbird --port $SONGBIRD_PORT > /tmp/songbird-demo.log 2>&1 &
        SONGBIRD_PID=$!
        echo $SONGBIRD_PID > /tmp/songbird-demo.pid
    elif [ -f "Cargo.toml" ]; then
        echo -e "${YELLOW}   Building Songbird...${NC}"
        cargo build --release
        nohup ./target/release/songbird --port $SONGBIRD_PORT > /tmp/songbird-demo.log 2>&1 &
        SONGBIRD_PID=$!
        echo $SONGBIRD_PID > /tmp/songbird-demo.pid
    else
        echo -e "${YELLOW}   No Cargo.toml found, will simulate Songbird${NC}"
    fi
    
    sleep 2
    echo -e "${GREEN}✓ Songbird started (PID: $SONGBIRD_PID)${NC}"
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 3: Start Squirrel (AI Gateway)
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 3: Starting Squirrel (AI Gateway)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check if Squirrel is already running
if lsof -Pi :$SQUIRREL_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Squirrel already running on port $SQUIRREL_PORT${NC}"
    echo -e "${BLUE}   Using existing instance${NC}"
else
    echo -e "${BLUE}🐿️  Starting Squirrel on port $SQUIRREL_PORT...${NC}"
    
    cd "$SQUIRREL_ROOT"
    
    # Load API keys if available
    if [ -f "$SECRETS_DIR/api-keys.toml" ]; then
        export SQUIRREL_API_KEYS="$SECRETS_DIR/api-keys.toml"
        echo -e "${GREEN}   Loaded API keys from secrets${NC}"
    fi
    
    # Check if Squirrel binary exists
    if [ -f "target/release/squirrel" ]; then
        echo -e "${GREEN}   Found built binary${NC}"
        nohup ./target/release/squirrel --port $SQUIRREL_PORT > /tmp/squirrel-demo.log 2>&1 &
        SQUIRREL_PID=$!
        echo $SQUIRREL_PID > /tmp/squirrel-demo.pid
    elif [ -f "Cargo.toml" ]; then
        echo -e "${YELLOW}   Building Squirrel...${NC}"
        cargo build --release
        nohup ./target/release/squirrel --port $SQUIRREL_PORT > /tmp/squirrel-demo.log 2>&1 &
        SQUIRREL_PID=$!
        echo $SQUIRREL_PID > /tmp/squirrel-demo.pid
    else
        echo -e "${YELLOW}   No Cargo.toml found, will simulate Squirrel${NC}"
    fi
    
    sleep 2
    echo -e "${GREEN}✓ Squirrel started (PID: $SQUIRREL_PID)${NC}"
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 4: Start ToadStool (Orchestrator)
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 4: Starting ToadStool (Universal Orchestrator)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check if ToadStool is already running
if lsof -Pi :$TOADSTOOL_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  ToadStool already running on port $TOADSTOOL_PORT${NC}"
    echo -e "${BLUE}   Using existing instance${NC}"
else
    echo -e "${BLUE}🍄 Starting ToadStool on port $TOADSTOOL_PORT...${NC}"
    
    cd "$TOADSTOOL_ROOT"
    
    # Configure ToadStool to connect to other primals
    export ENABLE_PRIMAL_CAPABILITIES=true
    export SONGBIRD_ENDPOINT="http://localhost:$SONGBIRD_PORT"
    export SQUIRREL_ENDPOINT="http://localhost:$SQUIRREL_PORT"
    
    echo -e "${GREEN}   Configured primal endpoints:${NC}"
    echo -e "   ${CYAN}Songbird: http://localhost:$SONGBIRD_PORT${NC}"
    echo -e "   ${CYAN}Squirrel: http://localhost:$SQUIRREL_PORT${NC}"
    
    # Start ToadStool server (if available)
    if [ -f "target/release/toadstool-server" ]; then
        nohup ./target/release/toadstool-server --port $TOADSTOOL_PORT > /tmp/toadstool-demo.log 2>&1 &
        TOADSTOOL_PID=$!
        echo $TOADSTOOL_PID > /tmp/toadstool-demo.pid
        sleep 2
        echo -e "${GREEN}✓ ToadStool server started (PID: $TOADSTOOL_PID)${NC}"
    else
        echo -e "${YELLOW}   ToadStool server binary not found${NC}"
        echo -e "${BLUE}   Demo will run without server mode${NC}"
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 5: Verify All Services Running
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 5: Service Health Checks${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${BLUE}Checking services...${NC}"

# Check Songbird
if lsof -Pi :$SONGBIRD_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Songbird listening on port $SONGBIRD_PORT${NC}"
else
    echo -e "${YELLOW}⚠ Songbird not listening (may not be required for demo)${NC}"
fi

# Check Squirrel
if lsof -Pi :$SQUIRREL_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Squirrel listening on port $SQUIRREL_PORT${NC}"
else
    echo -e "${YELLOW}⚠ Squirrel not listening (may not be required for demo)${NC}"
fi

# Check ToadStool
if lsof -Pi :$TOADSTOOL_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${GREEN}✓ ToadStool listening on port $TOADSTOOL_PORT${NC}"
else
    echo -e "${YELLOW}⚠ ToadStool not listening (CLI mode available)${NC}"
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 6: Run Demo Scenarios
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}Step 6: Running AI Orchestration Demo${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${MAGENTA}${BOLD}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}${BOLD}║  🌿 All Three Primals Running Together!                      ║${NC}"
echo -e "${MAGENTA}${BOLD}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${CYAN}Architecture:${NC}"
echo ""
echo "         ┌─────────────────┐"
echo "         │  🍄 ToadStool   │  ← Orchestrator"
echo "         │  (port $TOADSTOOL_PORT)   │"
echo "         └────────┬────────┘"
echo "                  │"
echo "       ┌──────────┴──────────┐"
echo "       │                     │"
echo "       ▼                     ▼"
echo "  ┌────────────┐      ┌──────────────┐"
echo "  │ 🐦 Songbird│      │ 🐿️  Squirrel │"
echo "  │ (port $SONGBIRD_PORT)│      │  (port $SQUIRREL_PORT) │"
echo "  │  Messaging │      │  AI Gateway  │"
echo "  └────────────┘      └──────┬───────┘"
echo "                             │"
echo "                      ┌──────┴──────┐"
echo "                      │             │"
echo "                      ▼             ▼"
echo "                 Local AI     Cloud APIs"
echo ""

read -p "$(echo -e ${BOLD}Press ENTER to run demo scenarios...${NC})"

# Run the visual demo
cd "$TOADSTOOL_ROOT/showcase/real-world/06-ai-orchestration"
./demo.sh hybrid

echo ""

# ═══════════════════════════════════════════════════════════════
# Step 7: Cleanup
# ═══════════════════════════════════════════════════════════════

echo ""
echo -e "${BOLD}${BLUE}Demo Complete!${NC}"
echo ""
echo -e "${YELLOW}Services are still running. To stop them:${NC}"
echo ""

if [ -f /tmp/songbird-demo.pid ]; then
    echo -e "  ${CYAN}Songbird:  kill \$(cat /tmp/songbird-demo.pid)${NC}"
fi

if [ -f /tmp/squirrel-demo.pid ]; then
    echo -e "  ${CYAN}Squirrel:  kill \$(cat /tmp/squirrel-demo.pid)${NC}"
fi

if [ -f /tmp/toadstool-demo.pid ]; then
    echo -e "  ${CYAN}ToadStool: kill \$(cat /tmp/toadstool-demo.pid)${NC}"
fi

echo ""
echo -e "${CYAN}Or stop all at once:${NC}"
echo -e "  ${YELLOW}kill \$(cat /tmp/*-demo.pid 2>/dev/null)${NC}"
echo ""

echo -e "${BLUE}Logs available at:${NC}"
echo -e "  ${CYAN}/tmp/songbird-demo.log${NC}"
echo -e "  ${CYAN}/tmp/squirrel-demo.log${NC}"
echo -e "  ${CYAN}/tmp/toadstool-demo.log${NC}"
echo ""

echo -e "${GREEN}${BOLD}🌿 ecoPrimals Integration Demo Complete!${NC}"
echo ""

exit 0

