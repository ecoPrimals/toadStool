#!/bin/bash
#
# GPU Benchmarking Runner
# Runs comprehensive benchmarks on all available GPUs (NVIDIA + AMD)
# Compares WGPU performance against native backends

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "╔══════════════════════════════════════════════════════════════════════════╗"
echo "║                                                                          ║"
echo "║               GPU BENCHMARKING SUITE - barraCUDA                        ║"
echo "║                                                                          ║"
echo "╚══════════════════════════════════════════════════════════════════════════╝"
echo ""

# Detect available GPUs
echo -e "${BLUE}🔍 Detecting Available GPUs...${NC}"
echo ""

HAS_NVIDIA=false
HAS_AMD=false

# Check for NVIDIA GPU
if command -v nvidia-smi &> /dev/null; then
    NVIDIA_INFO=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null || echo "")
    if [ -n "$NVIDIA_INFO" ]; then
        HAS_NVIDIA=true
        echo -e "${GREEN}✅ NVIDIA GPU Detected:${NC}"
        echo "   $NVIDIA_INFO"
        echo ""
    fi
fi

# Check for AMD GPU
if command -v rocm-smi &> /dev/null; then
    AMD_INFO=$(rocm-smi --showproductname 2>/dev/null | grep "Card model" || echo "")
    if [ -n "$AMD_INFO" ]; then
        HAS_AMD=true
        echo -e "${GREEN}✅ AMD GPU Detected:${NC}"
        echo "   $AMD_INFO"
        echo ""
    fi
fi

if [ "$HAS_NVIDIA" = false ] && [ "$HAS_AMD" = false ]; then
    echo -e "${RED}❌ No GPUs detected. Exiting.${NC}"
    exit 1
fi

# Create results directory
RESULTS_DIR="benchmark_results/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}📁 Results Directory: ${RESULTS_DIR}${NC}"
echo ""

# Navigate to ML inference showcase
cd "$(dirname "$0")/../showcase/gpu-universal/ml-inference"

echo "═══════════════════════════════════════════════════════════════════════════"
echo ""

# Function to run benchmarks on a specific GPU
run_benchmarks() {
    local GPU_TYPE=$1
    local ENV_VAR=$2
    
    echo -e "${YELLOW}🚀 Running Benchmarks on ${GPU_TYPE}...${NC}"
    echo ""
    
    # Set environment variable to select GPU
    export WGPU_ADAPTER_NAME="${ENV_VAR}"
    
    # Run comprehensive benchmarks
    echo -e "${BLUE}📊 Comprehensive Operations Benchmark...${NC}"
    cargo bench --bench gpu_ops_comprehensive 2>&1 | tee "${RESULTS_DIR}/${GPU_TYPE}_comprehensive.log"
    
    echo ""
    echo -e "${BLUE}📊 Hot Path Operations Benchmark...${NC}"
    cargo bench --bench simple_benchmarks 2>&1 | tee "${RESULTS_DIR}/${GPU_TYPE}_hot_paths.log"
    
    echo ""
    echo -e "${GREEN}✅ ${GPU_TYPE} Benchmarks Complete${NC}"
    echo ""
}

# Run benchmarks on NVIDIA
if [ "$HAS_NVIDIA" = true ]; then
    run_benchmarks "NVIDIA" "NVIDIA"
fi

# Run benchmarks on AMD
if [ "$HAS_AMD" = true ]; then
    run_benchmarks "AMD" "AMD"
fi

echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo -e "${GREEN}🎉 All Benchmarks Complete!${NC}"
echo ""
echo -e "${BLUE}📊 Results saved to: ${RESULTS_DIR}${NC}"
echo ""

# Generate comparison report
if [ "$HAS_NVIDIA" = true ] && [ "$HAS_AMD" = true ]; then
    echo -e "${YELLOW}📈 Generating Comparison Report...${NC}"
    
    cat > "${RESULTS_DIR}/COMPARISON_SUMMARY.md" << EOF
# GPU Benchmark Comparison Report

**Date**: $(date)
**GPUs**: NVIDIA + AMD

## Hardware

### NVIDIA
${NVIDIA_INFO}

### AMD
${AMD_INFO}

## Benchmark Files

- NVIDIA Comprehensive: nvidia_comprehensive.log
- NVIDIA Hot Paths: nvidia_hot_paths.log
- AMD Comprehensive: amd_comprehensive.log
- AMD Hot Paths: amd_hot_paths.log

## Analysis

Run the following to extract performance metrics:

\`\`\`bash
grep "time:" *_comprehensive.log | sort
\`\`\`

## Next Steps

1. Analyze performance differences
2. Identify optimization opportunities
3. Compare against CUDA/ROCm baselines
4. Plan evolution based on findings

EOF
    
    echo -e "${GREEN}✅ Comparison report generated${NC}"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo -e "${GREEN}✅ BENCHMARK SESSION COMPLETE${NC}"
echo ""
