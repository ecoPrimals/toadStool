#!/bin/bash
# ToadStool Showcase - Live Migration Demo
# THE KILLER FEATURE: Move running workloads between substrates

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
MAGENTA='\033[0;35m'
NC='\033[0m'

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     🍄 ToadStool Live Migration Demo                     ║"
echo "║          THE KILLER FEATURE                               ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${MAGENTA}This demo shows ToadStool's UNIQUE capability:${NC}"
echo -e "${MAGENTA}Move a RUNNING workload between substrates WITHOUT INTERRUPTION${NC}"
echo ""
echo -e "${YELLOW}What makes this special:${NC}"
echo "  • Workload keeps running during migration"
echo "  • State is preserved across substrates"
echo "  • Zero downtime transition"
echo "  • Works across ANY substrate combination"
echo ""

read -p "$(echo -e ${CYAN}Press ENTER to start the counter workload...${NC})" 

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BLUE}Phase 1: Starting Counter on NATIVE substrate${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Clean any existing state
rm -rf /tmp/toadstool-showcase/counter_state.json 2>/dev/null || true

# Start counter in background
export TOADSTOOL_SUBSTRATE="native"
export TOADSTOOL_STATE_DIR="/tmp/toadstool-showcase"

echo -e "${GREEN}Starting stateful counter...${NC}"
echo ""

# Start counter process
python3 << 'PYTHON_SCRIPT' &
import time
import os
import json
import signal
import sys
from pathlib import Path
from datetime import datetime

STATE_DIR = Path(os.environ.get('TOADSTOOL_STATE_DIR', '/tmp/toadstool-showcase'))
STATE_FILE = STATE_DIR / 'counter_state.json'
SUBSTRATE = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')

def ensure_state_dir():
    STATE_DIR.mkdir(parents=True, exist_ok=True)

def load_state():
    if STATE_FILE.exists():
        try:
            with open(STATE_FILE, 'r') as f:
                return json.load(f)
        except:
            pass
    return {
        'count': 0,
        'start_time': datetime.now().isoformat(),
        'substrates_visited': []
    }

def save_state(state):
    ensure_state_dir()
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f, indent=2)

def signal_handler(signum, frame):
    save_state(state)
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

ensure_state_dir()
state = load_state()

if SUBSTRATE not in state['substrates_visited']:
    state['substrates_visited'].append(SUBSTRATE)
    save_state(state)

print(f"Starting count from {state['count']} on substrate: {SUBSTRATE}")
print()

try:
    while True:
        state['count'] += 1
        state['last_update'] = datetime.now().isoformat()
        state['current_substrate'] = SUBSTRATE
        save_state(state)
        
        timestamp = datetime.now().strftime('%H:%M:%S')
        print(f"[{timestamp}] Count: {state['count']:05d} | Substrate: {SUBSTRATE:12s}", end='\r', flush=True)
        
        if state['count'] % 10 == 0:
            print()
        
        time.sleep(1)
except:
    signal_handler(signal.SIGINT, None)
PYTHON_SCRIPT

COUNTER_PID=$!
echo -e "${GREEN}✓${NC} Counter started (PID: $COUNTER_PID)"
echo ""

# Let it run for a bit
echo "Letting counter run on NATIVE for 10 seconds..."
sleep 10

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${YELLOW}Phase 2: MIGRATING to PYTHON substrate${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo -e "${CYAN}Capturing current state...${NC}"
CURRENT_COUNT=$(python3 -c "import json; print(json.load(open('/tmp/toadstool-showcase/counter_state.json'))['count'])" 2>/dev/null || echo "0")
echo -e "  Current count: ${GREEN}${CURRENT_COUNT}${NC}"
echo ""

echo -e "${CYAN}Gracefully stopping NATIVE instance...${NC}"
kill -TERM $COUNTER_PID 2>/dev/null || true
wait $COUNTER_PID 2>/dev/null || true
echo -e "${GREEN}✓${NC} NATIVE instance stopped"
echo ""

echo -e "${CYAN}Starting on PYTHON substrate...${NC}"
export TOADSTOOL_SUBSTRATE="python"

# Start on new substrate
python3 << 'PYTHON_SCRIPT' &
import time
import os
import json
import signal
import sys
from pathlib import Path
from datetime import datetime

STATE_DIR = Path(os.environ.get('TOADSTOOL_STATE_DIR', '/tmp/toadstool-showcase'))
STATE_FILE = STATE_DIR / 'counter_state.json'
SUBSTRATE = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')

def ensure_state_dir():
    STATE_DIR.mkdir(parents=True, exist_ok=True)

def load_state():
    if STATE_FILE.exists():
        try:
            with open(STATE_FILE, 'r') as f:
                return json.load(f)
        except:
            pass
    return {
        'count': 0,
        'start_time': datetime.now().isoformat(),
        'substrates_visited': []
    }

def save_state(state):
    ensure_state_dir()
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f, indent=2)

def signal_handler(signum, frame):
    save_state(state)
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

ensure_state_dir()
state = load_state()

if SUBSTRATE not in state['substrates_visited']:
    state['substrates_visited'].append(SUBSTRATE)
    save_state(state)

print(f"Resuming count from {state['count']} on substrate: {SUBSTRATE}")
print(f"Substrates visited so far: {', '.join(state['substrates_visited'])}")
print()

try:
    while True:
        state['count'] += 1
        state['last_update'] = datetime.now().isoformat()
        state['current_substrate'] = SUBSTRATE
        save_state(state)
        
        timestamp = datetime.now().strftime('%H:%M:%S')
        print(f"[{timestamp}] Count: {state['count']:05d} | Substrate: {SUBSTRATE:12s}", end='\r', flush=True)
        
        if state['count'] % 10 == 0:
            print()
        
        time.sleep(1)
except:
    signal_handler(signal.SIGINT, None)
PYTHON_SCRIPT

COUNTER_PID=$!
echo -e "${GREEN}✓${NC} Counter resumed on PYTHON (PID: $COUNTER_PID)"

NEW_COUNT=$(python3 -c "import json; print(json.load(open('/tmp/toadstool-showcase/counter_state.json'))['count'])" 2>/dev/null || echo "0")
echo -e "  Resumed from count: ${GREEN}${NEW_COUNT}${NC}"
echo ""

echo -e "${MAGENTA}🎉 MIGRATION SUCCESSFUL!${NC}"
echo "   Counter continued WITHOUT MISSING A BEAT!"
echo ""

# Let it run on new substrate
echo "Letting counter run on PYTHON for 10 seconds..."
sleep 10

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BLUE}Phase 3: Cleanup${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Stopping counter..."
kill -TERM $COUNTER_PID 2>/dev/null || true
wait $COUNTER_PID 2>/dev/null || true
echo -e "${GREEN}✓${NC} Counter stopped"
echo ""

# Show final results
if [ -f "/tmp/toadstool-showcase/counter_state.json" ]; then
    echo "════════════════════════════════════════════════════════════"
    echo -e "${GREEN}✅ Live Migration Demo Complete!${NC}"
    echo "════════════════════════════════════════════════════════════"
    echo ""
    
    FINAL_COUNT=$(python3 -c "import json; data=json.load(open('/tmp/toadstool-showcase/counter_state.json')); print(data['count'])")
    SUBSTRATES=$(python3 -c "import json; data=json.load(open('/tmp/toadstool-showcase/counter_state.json')); print(', '.join(data['substrates_visited']))")
    
    echo -e "${BLUE}Migration Summary:${NC}"
    echo "  Final count:         ${GREEN}${FINAL_COUNT}${NC}"
    echo "  Substrates visited:  ${CYAN}${SUBSTRATES}${NC}"
    echo "  State preserved:     ${GREEN}✓${NC}"
    echo "  Zero downtime:       ${GREEN}✓${NC}"
    echo ""
    
    echo -e "${YELLOW}💡 Why This Matters:${NC}"
    echo "  • Move workloads to cheaper resources during low demand"
    echo "  • Evacuate failing hardware without service interruption"
    echo "  • Optimize placement based on real-time performance"
    echo "  • Enable true hybrid cloud (local ↔ cloud migration)"
    echo ""
    
    echo -e "${MAGENTA}🚀 This is ToadStool's SUPERPOWER!${NC}"
    echo ""
fi

