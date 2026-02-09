#!/bin/bash
# Master Demo Runner - Execute all ToadStool real-world showcases
set -e

SHOWCASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

clear

cat << "BANNER"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║        🍄 ToadStool Real-World Showcase Collection 🍄        ║
║                                                               ║
║              5 Real Demos, Real Workloads                    ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
BANNER

echo ""
echo -e "${CYAN}This showcase demonstrates ToadStool's real-world capabilities:${NC}"
echo ""
echo "  1️⃣  GPU Classroom Manager - Share 3090 among students"
echo "  2️⃣  Symbiotic Gaming - 5090 gaming priority + background compute"
echo "  3️⃣  Home Game Server Hosting - Free hosting with personal priority"
echo "  4️⃣  Self-Managing ToadStool - Auto-healing & performance learning"
echo "  5️⃣  Network Pool - Distributed compute across ToadStool nodes"
echo ""

# Interactive menu
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}Select demo to run:${NC}"
echo ""
echo "  [1] GPU Classroom Manager (3 minutes)"
echo "  [2] Symbiotic Gaming + Compute (3 minutes)"
echo "  [3] Home Game Server Hosting (2 minutes)"
echo "  [4] Self-Managing ToadStool (3 minutes)"
echo "  [5] Network Pool (3 minutes)"
echo ""
echo "  [A] Run ALL demos sequentially (15 minutes)"
echo "  [Q] Quit"
echo ""
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo ""

read -p "Your choice: " CHOICE

case $CHOICE in
    1)
        echo -e "${GREEN}Running Demo 1: GPU Classroom Manager${NC}"
        sleep 1
        "$SHOWCASE_DIR/01-gpu-classroom/demo.sh"
        ;;
    2)
        echo -e "${GREEN}Running Demo 2: Symbiotic Gaming + Compute${NC}"
        sleep 1
        "$SHOWCASE_DIR/02-symbiotic-gaming/demo.sh"
        ;;
    3)
        echo -e "${GREEN}Running Demo 3: Home Game Server Hosting${NC}"
        sleep 1
        "$SHOWCASE_DIR/03-game-server-host/demo.sh"
        ;;
    4)
        echo -e "${GREEN}Running Demo 4: Self-Managing ToadStool${NC}"
        sleep 1
        "$SHOWCASE_DIR/04-self-monitoring/demo.sh"
        ;;
    5)
        echo -e "${GREEN}Running Demo 5: Network Pool${NC}"
        sleep 1
        "$SHOWCASE_DIR/05-network-pool/demo.sh"
        ;;
    A|a)
        echo -e "${MAGENTA}╔═══════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${MAGENTA}║           Running ALL Real-World Showcases!                 ║${NC}"
        echo -e "${MAGENTA}╚═══════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        sleep 2

        # Demo 1
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}[1/5] GPU Classroom Manager${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        sleep 1
        "$SHOWCASE_DIR/01-gpu-classroom/demo.sh"
        echo ""
        echo -e "${GREEN}✅ Demo 1 complete! Press Enter for Demo 2...${NC}"
        read

        # Demo 2
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}[2/5] Symbiotic Gaming + Compute${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        sleep 1
        "$SHOWCASE_DIR/02-symbiotic-gaming/demo.sh"
        echo ""
        echo -e "${GREEN}✅ Demo 2 complete! Press Enter for Demo 3...${NC}"
        read

        # Demo 3
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}[3/5] Home Game Server Hosting${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        sleep 1
        "$SHOWCASE_DIR/03-game-server-host/demo.sh"
        echo ""
        echo -e "${GREEN}✅ Demo 3 complete! Press Enter for Demo 4...${NC}"
        read

        # Demo 4
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}[4/5] Self-Managing ToadStool${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        sleep 1
        "$SHOWCASE_DIR/04-self-monitoring/demo.sh"
        echo ""
        echo -e "${GREEN}✅ Demo 4 complete! Press Enter for Demo 5...${NC}"
        read

        # Demo 5
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}[5/5] Network Pool${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        sleep 1
        "$SHOWCASE_DIR/05-network-pool/demo.sh"

        # Final summary
        echo ""
        echo ""
        echo -e "${MAGENTA}╔═══════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${MAGENTA}║            🎉 ALL SHOWCASES COMPLETE! 🎉                     ║${NC}"
        echo -e "${MAGENTA}╚═══════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "${GREEN}You just witnessed ToadStool's real-world capabilities:${NC}"
        echo ""
        echo "  ✅ GPU resource sharing (classroom with quotas)"
        echo "  ✅ Symbiotic computing (gaming priority + background jobs)"
        echo "  ✅ Game server hosting (free hosting with priority)"
        echo "  ✅ Self-management (auto-healing, performance learning)"
        echo "  ✅ Distributed compute (network pool, 4.2x speedup)"
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}ToadStool: Universal compute for the real world.${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
        echo ""
        ;;
    Q|q)
        echo "Exiting. Thanks for checking out ToadStool!"
        exit 0
        ;;
    *)
        echo -e "${YELLOW}Invalid choice. Exiting.${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Thanks for watching the ToadStool showcase!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

exit 0

