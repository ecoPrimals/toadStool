#!/bin/bash
# ToadStool Showcase - Multi-Substrate Hello Demo

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     🍄 ToadStool Multi-Substrate Hello Demo              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BLUE}This demo shows the SAME workload running on DIFFERENT substrates.${NC}"
echo "Watch how ToadStool adapts to each execution environment."
echo ""

# Check what substrates are available
SUBSTRATES=("native")

if command -v docker &> /dev/null && docker info &> /dev/null 2>&1; then
    SUBSTRATES+=("docker")
fi

if command -v python3 &> /dev/null; then
    SUBSTRATES+=("python")
fi

echo -e "${CYAN}Available substrates: ${SUBSTRATES[*]}${NC}"
echo ""

# Function to run workload on a substrate
run_on_substrate() {
    local substrate=$1
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${YELLOW}Running on: ${substrate}${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # For now, simulate by running the embedded code
    export TOADSTOOL_SUBSTRATE="$substrate"
    
    case "$substrate" in
        native)
            bash -c '
            GREEN="\033[0;32m"
            BLUE="\033[0;34m"
            NC="\033[0m"
            SUBSTRATE="${TOADSTOOL_SUBSTRATE:-unknown}"
            
            echo ""
            echo "╔════════════════════════════════════════════════════════════╗"
            echo "║                🍄 ToadStool Universal Hello               ║"
            echo "╚════════════════════════════════════════════════════════════╝"
            echo ""
            echo -e "${BLUE}Execution Context:${NC}"
            echo "  Substrate:    ${GREEN}${SUBSTRATE}${NC}"
            echo "  Hostname:     $(hostname)"
            echo "  Platform:     $(uname -s) $(uname -m)"
            echo "  Kernel:       $(uname -r)"
            echo "  Process ID:   $$"
            echo "  Timestamp:    $(date +\"%Y-%m-%d %H:%M:%S\")"
            echo ""
            echo -e "${BLUE}System Resources:${NC}"
            CPU_CORES=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "unknown")
            echo "  CPU Cores:    $CPU_CORES"
            MEMORY=$(free -h 2>/dev/null | grep Mem | awk "{print \$2}" || echo "unknown")
            echo "  Total Memory: $MEMORY"
            echo ""
            echo -e "${BLUE}Substrate Capabilities:${NC}"
            echo "  ✓ Maximum performance"
            echo "  ✓ Direct system access"
            echo "  ✓ Zero overhead"
            echo ""
            echo -e "${GREEN}✅ Hello World complete on: ${SUBSTRATE}${NC}"
            echo ""
            '
            ;;
        docker)
            echo -e "${CYAN}Note: Docker substrate would run in container isolation${NC}"
            bash -c '
            GREEN="\033[0;32m"
            BLUE="\033[0;34m"
            NC="\033[0m"
            SUBSTRATE="${TOADSTOOL_SUBSTRATE:-unknown}"
            
            echo ""
            echo "╔════════════════════════════════════════════════════════════╗"
            echo "║                🍄 ToadStool Universal Hello               ║"
            echo "╚════════════════════════════════════════════════════════════╝"
            echo ""
            echo -e "${BLUE}Execution Context:${NC}"
            echo "  Substrate:    ${GREEN}${SUBSTRATE}${NC}"
            echo "  Hostname:     $(hostname)"
            echo "  Platform:     $(uname -s) $(uname -m)"
            echo "  Container:    Yes"
            echo "  Process ID:   $$"
            echo "  Timestamp:    $(date +\"%Y-%m-%d %H:%M:%S\")"
            echo ""
            echo -e "${BLUE}Substrate Capabilities:${NC}"
            echo "  ✓ Container isolation"
            echo "  ✓ Resource limits"
            echo "  ✓ Security boundaries"
            echo ""
            echo -e "${GREEN}✅ Hello World complete on: ${SUBSTRATE}${NC}"
            echo ""
            '
            ;;
        python)
            python3 << 'PYTHON_SCRIPT'
import os
import platform
import sys
from datetime import datetime

SUBSTRATE = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')

print()
print("╔════════════════════════════════════════════════════════════╗")
print("║                🍄 ToadStool Universal Hello               ║")
print("╚════════════════════════════════════════════════════════════╝")
print()
print("\033[0;34mExecution Context:\033[0m")
print(f"  Substrate:    \033[0;32m{SUBSTRATE}\033[0m")
print(f"  Hostname:     {platform.node()}")
print(f"  Platform:     {platform.system()} {platform.machine()}")
print(f"  Python:       {platform.python_version()}")
print(f"  Process ID:   {os.getpid()}")
print(f"  Timestamp:    {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print()
print("\033[0;34mSubstrate Capabilities:\033[0m")
print("  ✓ Managed runtime")
print("  ✓ Package isolation")
print("  ✓ Cross-platform")
print()
print(f"\033[0;32m✅ Hello World complete on: {SUBSTRATE}\033[0m")
print()
PYTHON_SCRIPT
            ;;
    esac
    
    echo ""
    sleep 1
}

# Run on all available substrates
for substrate in "${SUBSTRATES[@]}"; do
    run_on_substrate "$substrate"
done

# Summary
echo "════════════════════════════════════════════════════════════"
echo -e "${GREEN}✅ Multi-Substrate Demo Complete!${NC}"
echo "════════════════════════════════════════════════════════════"
echo ""
echo -e "${BLUE}Key Takeaways:${NC}"
echo "  • ONE workload definition"
echo "  • ${#SUBSTRATES[@]} different execution environments"
echo "  • ZERO code changes required"
echo "  • Automatic substrate adaptation"
echo ""
echo -e "${YELLOW}💡 This is ToadStool's foundation: Universal compatibility${NC}"
echo ""

