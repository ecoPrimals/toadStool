#!/usr/bin/env bash
set -euo pipefail

#══════════════════════════════════════════════════════════════════════════════
# bench-all-local.sh - Complete Local Benchmark Suite
#══════════════════════════════════════════════════════════════════════════════

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ToadStool Universal GPU Benchmark - Local Suite            ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Configuration
MATRIX_SIZE=2048
ITERATIONS=5
RESULTS_DIR="results/local"

mkdir -p "$RESULTS_DIR"

# Build if needed
if [[ ! -f target/release/bench-matrix-multiply ]]; then
    echo "Building benchmark..."
    cargo build --release --bin bench-matrix-multiply
    echo ""
fi

# System info
echo "═══ System Information ═══"
echo ""
echo "Hostname: $(hostname)"
echo "CPU: $(lscpu | grep 'Model name' | sed 's/Model name: *//')"
echo "Cores: $(nproc) ($(lscpu | grep '^CPU(s):' | awk '{print $2}') logical)"
echo "RAM: $(free -h | grep Mem | awk '{print $2}')"
echo ""

if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU:"
    nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader
    echo ""
fi

if command -v rocm-smi &> /dev/null; then
    echo "AMD GPU:"
    rocm-smi --showproductname
    echo ""
fi

echo "═══════════════════════════════════════════════════════════════"
echo ""

# CPU Baseline
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ CPU Baseline                                                │"
echo "└─────────────────────────────────────────────────────────────┘"
echo ""

./target/release/bench-matrix-multiply \
    --backend cpu \
    --size $MATRIX_SIZE \
    --iterations $ITERATIONS

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo ""

# GPU Backends
declare -a BACKENDS=()

if nvidia-smi &> /dev/null; then
    BACKENDS+=("cuda")
fi

if rocm-smi &> /dev/null; then
    BACKENDS+=("rocm")
fi

# WebGPU is portable
BACKENDS+=("webgpu")

for backend in "${BACKENDS[@]}"; do
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│ $backend Backend                                            │" | sed 's/$/                                        /' | cut -c1-65
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    
    ./target/release/bench-matrix-multiply \
        --backend "$backend" \
        --size $MATRIX_SIZE \
        --iterations $ITERATIONS || echo "⚠ Backend $backend not available"
    
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
done

# Summary
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Summary                                                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

echo "Results saved to:"
ls -1 "$RESULTS_DIR"/*.json 2>/dev/null || echo "  (no JSON files created)"
echo ""

# Create comparison table if jq is available
if command -v jq &> /dev/null; then
    echo "Performance Comparison:"
    echo ""
    printf "%-12s %12s %12s %12s %12s\n" "Backend" "Avg Time" "GFLOPS" "Throughput" "Power (W)"
    printf "%-12s %12s %12s %12s %12s\n" "--------" "---------" "-------" "----------" "---------"
    
    for file in "$RESULTS_DIR"/*.json; do
        if [[ -f "$file" ]]; then
            backend=$(jq -r '.backend' "$file")
            avg_ms=$(jq -r '.avg_time_ms' "$file")
            gflops=$(jq -r '.gflops' "$file")
            throughput=$(jq -r '.throughput' "$file")
            power=$(jq -r '.power_watts // "N/A"' "$file")
            
            printf "%-12s %11.2fms %11.2f %11.2f/s %12s\n" \
                "$backend" "$avg_ms" "$gflops" "$throughput" "$power"
        fi
    done
    echo ""
fi

echo "✅ Local benchmark suite complete!"
echo ""
echo "Next steps:"
echo "  1. Run on other towers: scp this script to Northgate, Southgate, etc."
echo "  2. Compare cross-tower performance"
echo "  3. Test with RX 6700 when it arrives"
echo "  4. Run distributed workloads across mesh"
echo ""

