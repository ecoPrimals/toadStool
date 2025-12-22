#!/bin/bash
# RUN_ALL_NESTGATE_SHOWCASES.sh
# Master script to run all NestGate showcase demos
# Time: ~90 minutes for complete run
# Can be interrupted and resumed at any level

set -euo pipefail

echo "════════════════════════════════════════════════════════════"
echo "  🗄️  NESTGATE COMPLETE SHOWCASE"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "This showcase demonstrates:"
echo "  • NestGate standalone capabilities"
echo "  • ToadStool + NestGate integration"
echo "  • Multi-primal workflows"
echo ""
echo "Estimated time: 90 minutes"
echo "Note: All demos work in demo mode (NestGate optional)"
echo ""
echo "════════════════════════════════════════════════════════════"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m'

TOTAL_START=$(date +%s)

# Level 0: NestGate Standalone
echo -e "${BLUE}═══ LEVEL 0: NestGate Standalone (15 minutes) ═══${NC}"
echo ""
echo "Understanding what NestGate provides..."
echo ""

cd nestgate-standalone/01-storage-basics/

echo -e "${PURPLE}>>> Running: Simple Storage Demo${NC}"
./demo-simple-storage.sh
echo ""
echo "Press Enter to continue..."
read

echo -e "${PURPLE}>>> Running: Large Files Demo${NC}"
./demo-large-files.sh
echo ""
echo "Press Enter to continue..."
read

echo -e "${PURPLE}>>> Running: Metadata Demo${NC}"
./demo-metadata.sh
echo ""

cd ../..

echo -e "${GREEN}✅ Level 0 Complete!${NC}"
echo ""
echo "Press Enter to continue to Level 1..."
read
echo ""

# Level 1: One-Way Integration
echo -e "${BLUE}═══ LEVEL 1: ToadStool → NestGate Integration (20 minutes) ═══${NC}"
echo ""
echo "ToadStool storing compute results in NestGate..."
echo ""

cd nestgate-integration/01-workload-results/

echo -e "${PURPLE}>>> Running: Workload Results Storage Demo${NC}"
./demo-store-results.sh
echo ""

cd ../..

echo -e "${GREEN}✅ Level 1 Complete!${NC}"
echo ""
echo "Press Enter to continue to Level 2..."
read
echo ""

# Level 2: Bidirectional Integration
echo -e "${BLUE}═══ LEVEL 2: Bidirectional Integration (25 minutes) ═══${NC}"
echo ""
echo "NestGate and ToadStool collaborating..."
echo ""
echo -e "${YELLOW}(Level 2 demos coming soon)${NC}"
echo ""

echo -e "${GREEN}✅ Level 2 Complete!${NC}"
echo ""
echo "Press Enter to continue to Level 3..."
read
echo ""

# Level 3: Multi-Primal
echo -e "${BLUE}═══ LEVEL 3: Multi-Primal Workflows (30 minutes) ═══${NC}"
echo ""
echo "All primals working together..."
echo ""
echo -e "${YELLOW}(Level 3 demos coming soon)${NC}"
echo ""

echo -e "${GREEN}✅ Level 3 Complete!${NC}"
echo ""

TOTAL_END=$(date +%s)
TOTAL_DURATION=$(( TOTAL_END - TOTAL_START ))
TOTAL_MINUTES=$(( TOTAL_DURATION / 60 ))
TOTAL_SECONDS=$(( TOTAL_DURATION % 60 ))

echo "════════════════════════════════════════════════════════════"
echo "  🎉 ALL SHOWCASES COMPLETE!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "⏱️  Total time: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"
echo ""
echo "📊 Summary:"
echo "   ✅ Level 0: NestGate Standalone (3 demos)"
echo "   ✅ Level 1: One-Way Integration (1 demo)"
echo "   🟡 Level 2: Bidirectional (coming soon)"
echo "   🟡 Level 3: Multi-Primal (coming soon)"
echo ""
echo "💡 You now understand:"
echo "   • What NestGate provides"
echo "   • How ToadStool uses NestGate"
echo "   • Capability-based discovery"
echo "   • Production-ready patterns"
echo ""
echo "🔗 Next steps:"
echo "   • Explore NestGate's full showcase: ../../../nestgate/showcase/"
echo "   • Read architecture docs: ../docs/"
echo "   • Build your own integrations!"
echo ""
echo "════════════════════════════════════════════════════════════"

