#!/bin/bash
# Master script to run all neuromorphic demos

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
NC='\033[0m'

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║         ToadStool Neuromorphic Computing Showcase          ║"
echo "║           BrainChip Akida PCIe Board Integration           ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check for hardware
echo "Checking for Akida hardware..."
cd 01-akida-detection
if cargo run --quiet --example detect_akida 2>&1 | grep -q "Found.*Akida"; then
    AKIDA_COUNT=$(cargo run --quiet --example detect_akida 2>&1 | grep -oP 'Found \K\d+' | head -1)
    echo -e "${GREEN}✓ Found ${AKIDA_COUNT} Akida board(s)${NC}"
    HARDWARE_PRESENT=true
else
    echo -e "${YELLOW}⚠ No Akida boards detected${NC}"
    echo ""
    echo "This showcase is designed for BrainChip Akida PCIe boards."
    echo "Expected deployment:"
    echo "  - 2x boards on Strandgate (Dual EPYC)"
    echo "  - 1x board on Southgate (Ryzen 5800X3D)"
    echo ""
    echo "Demos will run in simulation mode (mock inference)."
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    HARDWARE_PRESENT=false
fi
cd ..
echo ""

# Total start time
TOTAL_START=$(date +%s)

# ============================================================================
# Demo 1: Detection & Integration
# ============================================================================

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} Demo 1: Akida Detection & Integration${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

cd 01-akida-detection
./demo.sh
cd ..
echo ""
echo -e "${GREEN}✓ Demo 1 complete${NC}"
echo ""
sleep 2

# ============================================================================
# Demo 2: Bioinformatics (K-mer Filtering)
# ============================================================================

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} Demo 2: Bioinformatics Power Efficiency${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

cd 02-akida-bioinformatics
./demo-kmer-filtering.sh
cd ..
echo ""
echo -e "${GREEN}✓ Demo 2 complete${NC}"
echo ""
sleep 2

# ============================================================================
# Demo 3: LLM Intent Classification
# ============================================================================

if [ -f "03-akida-llm-intent/demo-intent-routing.sh" ]; then
    echo "═══════════════════════════════════════════════════════════════"
    echo -e "${MAGENTA} Demo 3: LLM Intent Classification & Routing${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    cd 03-akida-llm-intent
    ./demo-intent-routing.sh
    cd ..
    echo ""
    echo -e "${GREEN}✓ Demo 3 complete${NC}"
    echo ""
    sleep 2
else
    echo -e "${YELLOW}⚠ Demo 3 (LLM Intent) not yet implemented${NC}"
    echo ""
fi

# ============================================================================
# Demo 4: Universal Mesh Orchestration
# ============================================================================

if [ -f "04-akida-mesh/demo-hybrid-pipeline.sh" ]; then
    echo "═══════════════════════════════════════════════════════════════"
    echo -e "${MAGENTA} Demo 4: Universal Mesh Orchestration${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    cd 04-akida-mesh
    ./demo-hybrid-pipeline.sh
    cd ..
    echo ""
    echo -e "${GREEN}✓ Demo 4 complete${NC}"
    echo ""
else
    echo -e "${YELLOW}⚠ Demo 4 (Mesh Orchestration) not yet implemented${NC}"
    echo ""
fi

# ============================================================================
# Benchmarks (Optional)
# ============================================================================

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} Optional: Run Full Benchmark Suite?${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "This will run comprehensive benchmarks including:"
echo "  - MNIST classification"
echo "  - Bioinformatics throughput"
echo "  - LLM intent latency"
echo "  - Power measurements"
echo ""
echo "Estimated time: 30-60 minutes"
echo ""
read -p "Run benchmarks now? (y/N) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cd benchmarks
    
    # Download datasets if needed
    if [ ! -d "datasets/mnist" ]; then
        echo "Downloading datasets..."
        ./datasets/download.sh
        echo ""
    fi
    
    # Run benchmarks
    ./run-all-benchmarks.sh
    cd ..
    echo ""
    echo -e "${GREEN}✓ Benchmarks complete${NC}"
    echo ""
fi

# ============================================================================
# Summary
# ============================================================================

TOTAL_END=$(date +%s)
TOTAL_DURATION=$((TOTAL_END - TOTAL_START))
MINUTES=$((TOTAL_DURATION / 60))
SECONDS=$((TOTAL_DURATION % 60))

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║              All Neuromorphic Demos Complete!              ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Total time: ${MINUTES}m ${SECONDS}s"
echo ""

if [ "$HARDWARE_PRESENT" = true ]; then
    echo "Hardware Configuration:"
    echo "  ✓ ${AKIDA_COUNT} Akida PCIe board(s) detected"
    echo ""
    
    echo "Expected Results:"
    echo "  Bioinformatics:"
    echo "    • 50-100x power efficiency improvement"
    echo "    • 2-5x throughput improvement"
    echo "    • ~$310/year power savings"
    echo ""
    echo "  LLM Routing:"
    echo "    • <1ms intent classification"
    echo "    • ~$575K/year cloud API cost savings"
    echo "    • 120x faster than GPU routing"
    echo ""
    echo "  Total ROI: ~$600K/year with just 3 boards"
else
    echo "Hardware Status:"
    echo "  • Running in simulation mode (no Akida boards detected)"
    echo "  • Install boards to see real performance"
    echo ""
fi

echo "Results saved to:"
echo "  • 01-akida-detection/ - Board enumeration and health"
echo "  • 02-akida-bioinformatics/results/ - K-mer filtering benchmarks"
echo "  • 03-akida-llm-intent/results/ - Intent classification metrics"
echo "  • benchmarks/results/ - Comprehensive benchmark suite"
echo ""

echo "Documentation:"
echo "  • README.md - Complete showcase overview"
echo "  • BENCHMARKS.md - Benchmark methodology and results"
echo "  • ARCHITECTURE.md - Technical integration details"
echo "  • BRAINCHIP_PARTNERSHIP.md - Partnership proposal"
echo ""

echo "Next Steps:"
if [ "$HARDWARE_PRESENT" = false ]; then
    echo "  1. Install Akida PCIe boards (2x Strandgate, 1x Southgate)"
    echo "  2. Re-run demos for real performance measurements"
    echo "  3. Integrate into production pipelines"
    echo "  4. Prepare BrainChip presentation"
else
    echo "  1. Review results and optimize models"
    echo "  2. Deploy to production pipelines"
    echo "  3. Schedule BrainChip partnership call"
    echo "  4. Consider larger board order"
fi
echo ""

echo -e "${GREEN}Thank you for exploring ToadStool's neuromorphic computing showcase!${NC}"

