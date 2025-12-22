#!/bin/bash
# Benchmark all available GPU backends

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║         Universal GPU Backend Benchmark Suite              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

cd "$(dirname "$0")"

# Detect available backends
BACKENDS=()

if command -v nvidia-smi &> /dev/null; then
    BACKENDS+=("cuda")
    echo -e "${GREEN}✓ CUDA available (NVIDIA)${NC}"
fi

if command -v rocm-smi &> /dev/null; then
    BACKENDS+=("rocm")
    echo -e "${GREEN}✓ ROCm available (AMD)${NC}"
fi

# WebGPU always available (with CPU fallback)
BACKENDS+=("webgpu")
echo -e "${GREEN}✓ WebGPU available${NC}"

if command -v clinfo &> /dev/null && clinfo 2>/dev/null | grep -q "Number of platforms"; then
    BACKENDS+=("opencl")
    echo -e "${GREEN}✓ OpenCL available${NC}"
fi

echo ""
echo "Will benchmark ${#BACKENDS[@]} backend(s): ${BACKENDS[*]}"
echo ""

# Matrix size and iterations
SIZE=4096
ITERATIONS=10

echo "Configuration:"
echo "  Matrix size: ${SIZE}x${SIZE}"
echo "  Iterations: ${ITERATIONS}"
echo ""

# Run benchmarks
for backend in "${BACKENDS[@]}"; do
    echo "═══════════════════════════════════════════════════════════════"
    echo -e "${BLUE} Benchmarking: $backend${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    cargo run --release --bin bench-matrix-multiply -- \
        --backend "$backend" \
        --size "$SIZE" \
        --iterations "$ITERATIONS" || {
        echo "⚠️  Benchmark failed for $backend"
    }
    
    echo ""
done

# Generate comparison
echo "═══════════════════════════════════════════════════════════════"
echo " Comparison Summary"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if command -v jq &> /dev/null; then
    echo "Backend          | Avg Time (ms) | GFLOPS | Power (W) | Efficiency (GFLOPS/W)"
    echo "-----------------|---------------|--------|-----------|---------------------"
    
    for backend in "${BACKENDS[@]}"; do
        FILE="../../results/local/${backend}-matrix.json"
        if [ -f "$FILE" ]; then
            AVG_TIME=$(jq -r '.avg_time_ms' "$FILE")
            GFLOPS=$(jq -r '.gflops' "$FILE")
            POWER=$(jq -r '.power_watts // "N/A"' "$FILE")
            
            if [ "$POWER" != "N/A" ] && [ "$POWER" != "null" ]; then
                EFFICIENCY=$(echo "scale=2; $GFLOPS / $POWER" | bc)
            else
                EFFICIENCY="N/A"
                POWER="N/A"
            fi
            
            printf "%-16s | %13.2f | %6.2f | %9s | %s\n" \
                "$backend" "$AVG_TIME" "$GFLOPS" "$POWER" "$EFFICIENCY"
        fi
    done
    echo ""
    
    # Find fastest
    FASTEST_BACKEND=""
    FASTEST_TIME=999999
    
    for backend in "${BACKENDS[@]}"; do
        FILE="../../results/local/${backend}-matrix.json"
        if [ -f "$FILE" ]; then
            AVG_TIME=$(jq -r '.avg_time_ms' "$FILE")
            if (( $(echo "$AVG_TIME < $FASTEST_TIME" | bc -l) )); then
                FASTEST_TIME=$AVG_TIME
                FASTEST_BACKEND=$backend
            fi
        fi
    done
    
    if [ -n "$FASTEST_BACKEND" ]; then
        echo -e "${GREEN}Fastest backend: $FASTEST_BACKEND (${FASTEST_TIME}ms)${NC}"
    fi
    
else
    echo "Install 'jq' for detailed comparison"
    echo "  Results saved to ../../results/local/*.json"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " Analysis"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "Key Insights:"
echo "  1. CUDA typically fastest (if NVIDIA GPU present)"
echo "  2. ROCm competitive with CUDA (AMD GPUs)"
echo "  3. WebGPU universal but may be slower (improving)"
echo "  4. OpenCL legacy fallback (broad compatibility)"
echo ""

echo "Trade-offs:"
echo "  • CUDA: Fastest, NVIDIA-only"
echo "  • ROCm: Fast, AMD-only, CUDA-compatible"
echo "  • WebGPU: Universal, portable, future-proof"
echo "  • OpenCL: Universal, older, legacy"
echo ""

echo "ToadStool's Approach:"
echo "  ✓ Support ALL backends (no vendor lock-in)"
echo "  ✓ Auto-select best for each workload"
echo "  ✓ Transparent fallback on unavailable hardware"
echo "  ✓ Same code runs everywhere"
echo ""

echo -e "${GREEN}✓ Benchmark suite complete!${NC}"
echo ""
echo "Next: Run cross-tower benchmarks"
echo "  cd ../distributed && ./bench-cross-tower.sh"

