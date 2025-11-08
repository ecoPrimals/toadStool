#!/bin/bash
# ToadStool Showcase - Main Runner
# One-command demo of ToadStool's universal compute capabilities

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

# Change to showcase directory
cd "$(dirname "$0")"

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════════╗"
echo "║                                                                   ║"
echo "║               🍄  TOADSTOOL UNIVERSAL COMPUTE                    ║"
echo "║                      Live Showcase                                ║"
echo "║                                                                   ║"
echo "╚═══════════════════════════════════════════════════════════════════╝"
echo ""
echo -e "${BOLD}${BLUE}Run Anything, Anywhere, Zero Config${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Show menu
echo "Select a demo to run:"
echo ""
echo -e "  ${GREEN}1${NC}. 🌍 Multi-Substrate Hello World"
echo -e "       ${BLUE}└─${NC} Run the SAME workload on different substrates"
echo ""
echo -e "  ${GREEN}2${NC}. ⚡ Distributed Compute Demo ${MAGENTA}[NEW!]${NC}"
echo -e "       ${BLUE}└─${NC} Watch ToadStool split jobs & execute subtasks in parallel"
echo ""
echo -e "  ${GREEN}3${NC}. 📊 Performance Benchmarks"
echo -e "       ${BLUE}└─${NC} Compare CPU & I/O performance across substrates"
echo ""
echo -e "  ${GREEN}4${NC}. 🚀 Live Migration Demo ${MAGENTA}[THE KILLER FEATURE]${NC}"
echo -e "       ${BLUE}└─${NC} Move running workload between substrates with zero downtime"
echo ""
echo -e "  ${GREEN}5${NC}. 🎬 Full Showcase (All Demos)"
echo -e "       ${BLUE}└─${NC} Run complete showcase sequence"
echo ""
echo -e "  ${GREEN}6${NC}. 🔧 System Verification"
echo -e "       ${BLUE}└─${NC} Check prerequisites and system capabilities"
echo ""
echo -e "  ${GREEN}0${NC}. 🚪 Exit"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
read -p "$(echo -e ${YELLOW}Enter your choice [1-6, 0 to exit]:${NC} )" choice

echo ""

case $choice in
    1)
        echo -e "${CYAN}Running Multi-Substrate Hello Demo...${NC}"
        echo ""
        ./utils/setup.sh
        echo ""
        ./scripts/demo-hello.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    2)
        echo -e "${CYAN}Running Distributed Compute Demo...${NC}"
        echo ""
        ./utils/setup.sh
        echo ""
        ./scripts/demo-distributed-compute.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    3)
        echo -e "${CYAN}Running Performance Benchmarks...${NC}"
        echo ""
        ./utils/setup.sh
        echo ""
        ./scripts/demo-benchmark.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    4)
        echo -e "${CYAN}Running Live Migration Demo...${NC}"
        echo ""
        ./utils/setup.sh
        echo ""
        ./scripts/demo-migration.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    5)
        echo -e "${CYAN}Running Full Showcase...${NC}"
        echo ""
        ./utils/setup.sh
        echo ""
        
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo -e "${MAGENTA}${BOLD}   PART 1: MULTI-SUBSTRATE BASICS    ${NC}"
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo ""
        sleep 2
        ./scripts/demo-hello.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to continue to benchmarks...${NC})"
        
        echo ""
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo -e "${MAGENTA}${BOLD}   PART 2: PERFORMANCE ANALYSIS       ${NC}"
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo ""
        sleep 2
        ./scripts/demo-benchmark.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to continue to live migration...${NC})"
        
        echo ""
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo -e "${MAGENTA}${BOLD}   PART 3: THE KILLER FEATURE         ${NC}"
        echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════${NC}"
        echo ""
        sleep 2
        ./scripts/demo-migration.sh
        
        echo ""
        echo "╔═══════════════════════════════════════════════════════════════════╗"
        echo "║                                                                   ║"
        echo "║                 🎉 FULL SHOWCASE COMPLETE! 🎉                    ║"
        echo "║                                                                   ║"
        echo "╚═══════════════════════════════════════════════════════════════════╝"
        echo ""
        echo -e "${GREEN}${BOLD}ToadStool Universal Compute Platform${NC}"
        echo ""
        echo -e "${BLUE}What You Just Saw:${NC}"
        echo "  ✓ Multi-substrate execution (native, docker, python)"
        echo "  ✓ Performance benchmarking across substrates"
        echo "  ✓ Live workload migration with zero downtime"
        echo ""
        echo -e "${YELLOW}Why This Matters:${NC}"
        echo "  • True universal compute: run ANY workload ANYWHERE"
        echo "  • Zero-config deployment: no dockerfile, no k8s yaml, no setup"
        echo "  • Intelligent placement: route to optimal substrate"
        echo "  • Live migration: move workloads without interruption"
        echo "  • Future-proof: add new substrates without code changes"
        echo ""
        echo -e "${CYAN}Ready for Production:${NC}"
        echo "  • Edge to cloud hybrid deployments"
        echo "  • Cost optimization through dynamic placement"
        echo "  • High availability through live migration"
        echo "  • Legacy system integration without refactoring"
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    6)
        echo -e "${CYAN}Running System Verification...${NC}"
        echo ""
        ./utils/verify.sh
        echo ""
        read -p "$(echo -e ${YELLOW}Press ENTER to return to menu...${NC})"
        exec "$0"
        ;;
    0)
        echo -e "${GREEN}Thank you for exploring ToadStool!${NC}"
        echo ""
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice. Please select 0-6.${NC}"
        sleep 2
        exec "$0"
        ;;
esac

