#!/usr/bin/env bash
# Run all Level 1: Multi-Runtime demos in sequence

set -e

# Color codes
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "════════════════════════════════════════════════════════"
echo "🍄 ToadStool Multi-Runtime Workflows - Complete Tour"
echo "════════════════════════════════════════════════════════"
echo ""
echo "This will run all 3 Level 1 demos:"
echo "  1. Runtime Comparison (7 min) - Benchmark all 3"
echo "  2. Cross-Runtime Workflow (7 min) - Mix runtimes"
echo "  3. Runtime Selection Guide (6 min) - Choose wisely"
echo ""
echo "Total time: ~20 minutes"
echo ""
echo -e "${CYAN}💡 Building on Level 0: Now see them work together!${NC}"
echo ""
read -p "Press ENTER to continue or Ctrl+C to exit..."
echo ""

# Demo 1
echo "═══════════════════════════════════════════════════════"
echo "Demo 1/3: Runtime Comparison"
echo "═══════════════════════════════════════════════════════"
echo ""
./demo-runtime-comparison.sh
echo ""
echo -e "${GREEN}✅ Runtime comparison complete!${NC}"
echo ""
read -p "Press ENTER for next demo or Ctrl+C to exit..."
echo ""

# Demo 2
echo "═══════════════════════════════════════════════════════"
echo "Demo 2/3: Cross-Runtime Workflow"
echo "═══════════════════════════════════════════════════════"
echo ""
./demo-cross-runtime-workflow.sh
echo ""
echo -e "${GREEN}✅ Cross-runtime workflow complete!${NC}"
echo ""
read -p "Press ENTER for final demo or Ctrl+C to exit..."
echo ""

# Demo 3
echo "═══════════════════════════════════════════════════════"
echo "Demo 3/3: Runtime Selection Guide"
echo "═══════════════════════════════════════════════════════"
echo ""
./demo-runtime-selection.sh
echo ""
echo -e "${GREEN}✅ Runtime selection guide complete!${NC}"
echo ""

# Summary
echo "════════════════════════════════════════════════════════"
echo -e "${GREEN}🏆 Level 1: Multi-Runtime Workflows Complete!${NC}"
echo "════════════════════════════════════════════════════════"
echo ""
echo "🎓 You've mastered:"
echo ""
echo "   1. ${CYAN}Runtime Benchmarking${NC}"
echo "      • Compare Native, WASM, Python"
echo "      • Understand performance trade-offs"
echo ""
echo "   2. ${YELLOW}Cross-Runtime Pipelines${NC}"
echo "      • Mix runtimes strategically"
echo "      • Optimize each step independently"
echo ""
echo "   3. ${GREEN}Runtime Selection${NC}"
echo "      • Choose the right runtime"
echo "      • Avoid common anti-patterns"
echo ""
echo "💡 Key Lessons:"
echo "   • Native:  Maximum performance, trusted code"
echo "   • WASM:    Security + portability"
echo "   • Python:  ML/AI + rapid development"
echo "   • ${MAGENTA}Mix them for best results!${NC}"
echo ""
echo "➡️  What's Next?"
echo ""
echo "   ${CYAN}Level 2: Resource Management${NC}"
echo "   • CPU and memory limits"
echo "   • Fair scheduling"
echo "   • GPU quotas"
echo "   • Path: cd ../03-resource-management"
echo ""
echo "   ${CYAN}Level 4: GPU Compute${NC} (if you have GPU)"
echo "   • CUDA/ROCm acceleration"
echo "   • ML training demos"
echo "   • Path: cd ../05-gpu-compute"
echo ""
echo "   ${CYAN}Level 5: Production Patterns${NC}"
echo "   • Real-world examples"
echo "   • Fair classroom, symbiotic gaming"
echo "   • Path: cd ../06-production-patterns"
echo ""
echo "   ${YELLOW}Ecosystem Integration:${NC}"
echo "   • ToadStool + NestGate"
echo "   • ToadStool + Songbird"
echo "   • Path: cd ../../nestgate-integration"
echo ""
echo "════════════════════════════════════════════════════════"
echo "🍄 You're now a multi-runtime expert!"
echo "════════════════════════════════════════════════════════"
echo ""

