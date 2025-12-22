#!/bin/bash
# K-mer Filtering Demo Runner

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║   Akida Bioinformatics: K-mer Filtering Demo              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Building examples...${NC}"
cargo build --examples --release
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 1. Train SNN Model"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
cargo run --example train_kmer_model --release
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 2. Run Akida Filtering"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
cargo run --example run_akida_filter --release -- --sequences 50000
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 3. Compare CPU vs Akida"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
cargo run --example compare_cpu_akida --release -- --sequences 100000 --iterations 5
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 4. Power Measurement"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
cargo run --example power_measurement --release -- --duration 30
echo ""

echo -e "${GREEN}✓ All bioinformatics demos complete!${NC}"
echo ""
echo "Results saved to:"
echo "  - results/comparison.json"
echo ""
echo "Next steps:"
echo "  • Integrate with Kraken2 pipeline on Strandgate"
echo "  • Test with real sequencing data"
echo "  • Measure production power savings"

