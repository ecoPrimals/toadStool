#!/bin/bash
# Run all neuromorphic benchmarks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║           Neuromorphic Benchmark Suite Runner              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Check for Akida boards
echo "Checking for Akida hardware..."
if cargo run --quiet --manifest-path ../01-akida-detection/Cargo.toml --example detect_akida 2>&1 | grep -q "Found.*Akida"; then
    AKIDA_AVAILABLE=true
    echo -e "${GREEN}✓ Akida boards detected${NC}"
else
    AKIDA_AVAILABLE=false
    echo -e "${YELLOW}⚠ No Akida boards detected (will skip Akida benchmarks)${NC}"
fi
echo ""

# Check datasets
echo "Checking datasets..."
if [ ! -d "datasets/mnist" ]; then
    echo -e "${YELLOW}⚠ MNIST not found. Run ./datasets/download.sh first${NC}"
    echo ""
fi

# Create results directory
mkdir -p results

# Timestamp for this run
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="results/run_${TIMESTAMP}"
mkdir -p "$RESULTS_DIR"

echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Track start time
START_TIME=$(date +%s)

# ============================================================================
# MNIST Benchmark
# ============================================================================

if [ -d "datasets/mnist" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " MNIST Benchmark"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # CPU baseline
    echo -e "${BLUE}Running CPU baseline...${NC}"
    if cargo run --release --bin mnist_benchmark -- \
        --platform cpu \
        --dataset datasets/mnist \
        --output "$RESULTS_DIR/mnist_cpu.json" 2>&1 | tee "$RESULTS_DIR/mnist_cpu.log"; then
        echo -e "${GREEN}✓ CPU baseline complete${NC}"
    else
        echo -e "${RED}✗ CPU baseline failed${NC}"
    fi
    echo ""
    
    # GPU baseline (if available)
    if command -v nvidia-smi &> /dev/null; then
        echo -e "${BLUE}Running GPU baseline...${NC}"
        if cargo run --release --bin mnist_benchmark -- \
            --platform gpu \
            --dataset datasets/mnist \
            --output "$RESULTS_DIR/mnist_gpu.json" 2>&1 | tee "$RESULTS_DIR/mnist_gpu.log"; then
            echo -e "${GREEN}✓ GPU baseline complete${NC}"
        else
            echo -e "${YELLOW}⚠ GPU baseline failed${NC}"
        fi
        echo ""
    fi
    
    # Akida (if available)
    if [ "$AKIDA_AVAILABLE" = true ]; then
        echo -e "${BLUE}Running Akida acceleration...${NC}"
        if cargo run --release --bin mnist_benchmark -- \
            --platform akida \
            --dataset datasets/mnist \
            --output "$RESULTS_DIR/mnist_akida.json" 2>&1 | tee "$RESULTS_DIR/mnist_akida.log"; then
            echo -e "${GREEN}✓ Akida benchmark complete${NC}"
        else
            echo -e "${RED}✗ Akida benchmark failed${NC}"
        fi
        echo ""
    fi
fi

# ============================================================================
# Bioinformatics Benchmark
# ============================================================================

if [ -d "datasets/bioinformatics" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Bioinformatics (K-mer Filtering) Benchmark"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # CPU baseline
    echo -e "${BLUE}Running CPU baseline...${NC}"
    if cargo run --release --manifest-path ../02-akida-bioinformatics/Cargo.toml --example compare_cpu_akida -- \
        --sequences 50000 \
        --iterations 3 \
        --output "$RESULTS_DIR/bioinformatics.json" 2>&1 | tee "$RESULTS_DIR/bioinformatics.log"; then
        echo -e "${GREEN}✓ Bioinformatics benchmark complete${NC}"
    else
        echo -e "${RED}✗ Bioinformatics benchmark failed${NC}"
    fi
    echo ""
fi

# ============================================================================
# LLM Intent Classification Benchmark
# ============================================================================

if [ -d "datasets/llm" ] && [ "$AKIDA_AVAILABLE" = true ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " LLM Intent Classification Benchmark"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    echo -e "${BLUE}Running intent classification benchmark...${NC}"
    if cargo run --release --bin llm_intent_benchmark -- \
        --dataset datasets/llm/intent_test_set.jsonl \
        --platform akida \
        --output "$RESULTS_DIR/llm_intent.json" 2>&1 | tee "$RESULTS_DIR/llm_intent.log"; then
        echo -e "${GREEN}✓ LLM intent benchmark complete${NC}"
    else
        echo -e "${RED}✗ LLM intent benchmark failed${NC}"
    fi
    echo ""
fi

# ============================================================================
# Summary
# ============================================================================

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                  Benchmark Suite Complete                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Total time: ${DURATION}s"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "View results:"
echo "  cat $RESULTS_DIR/*.json | jq"
echo ""
echo "Generate comparison charts:"
echo "  cargo run --release --bin generate_charts -- --input $RESULTS_DIR"
echo ""
echo "Generate report:"
echo "  cargo run --release --bin generate_report -- --input $RESULTS_DIR --output results/report.md"

