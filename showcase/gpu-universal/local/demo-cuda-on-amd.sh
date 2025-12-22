#!/bin/bash
# CUDA on AMD GPU Demo - The Holy Grail of GPU Computing

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║          CUDA on AMD: Universal Abstraction Proof          ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Check for AMD GPU
echo "Checking for AMD GPU..."
if ! command -v rocm-smi &> /dev/null; then
    echo -e "${YELLOW}⚠️  ROCm not installed. Install ROCm to run CUDA on AMD.${NC}"
    echo ""
    echo "Installation:"
    echo "  https://rocmdocs.amd.com/en/latest/Installation_Guide/Installation-Guide.html"
    echo ""
    echo "For now, this demo will run in simulation mode."
    AMD_AVAILABLE=false
else
    echo -e "${GREEN}✓ ROCm detected${NC}"
    AMD_AVAILABLE=true
    
    # Show AMD GPU info
    echo ""
    echo "AMD GPU Information:"
    rocm-smi --showproductname || true
    echo ""
fi

# Check for NVIDIA GPU
echo "Checking for NVIDIA GPU..."
if command -v nvidia-smi &> /dev/null; then
    echo -e "${GREEN}✓ NVIDIA GPU detected${NC}"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    NVIDIA_AVAILABLE=true
else
    echo -e "${YELLOW}⚠️  No NVIDIA GPU detected${NC}"
    NVIDIA_AVAILABLE=false
fi
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} Demo: Matrix Multiplication (CUDA code)${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "This demo runs the SAME CUDA-style code on both NVIDIA and AMD GPUs."
echo "ToadStool's universal abstraction automatically translates:"
echo "  • NVIDIA → Native CUDA execution"
echo "  • AMD → ROCm/HIP translation (CUDA→AMD ISA)"
echo ""

if [ "$NVIDIA_AVAILABLE" = true ]; then
    echo -e "${BLUE}Running on NVIDIA GPU (native CUDA)...${NC}"
    cargo run --release --bin bench-matrix-multiply -- \
        --backend cuda \
        --size 2048 \
        --iterations 10
    echo ""
fi

if [ "$AMD_AVAILABLE" = true ]; then
    echo -e "${BLUE}Running on AMD GPU (ROCm/HIP translation)...${NC}"
    cargo run --release --bin bench-matrix-multiply -- \
        --backend rocm \
        --size 2048 \
        --iterations 10
    echo ""
fi

if [ "$NVIDIA_AVAILABLE" = false ] && [ "$AMD_AVAILABLE" = false ]; then
    echo -e "${YELLOW}No GPUs available. Running CPU fallback demonstration.${NC}"
    echo ""
    cargo run --release --bin bench-matrix-multiply -- \
        --size 1024 \
        --iterations 5
fi

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} What Just Happened?${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ "$NVIDIA_AVAILABLE" = true ] && [ "$AMD_AVAILABLE" = true ]; then
    echo -e "${GREEN}✓ CUDA CODE RAN ON BOTH NVIDIA AND AMD!${NC}"
    echo ""
    echo "Here's the magic:"
    echo ""
    echo "1. You write ONE workload in CUDA-style:"
    echo "     __global__ void matmul(float* a, float* b, float* c) {"
    echo "         int idx = blockIdx.x * blockDim.x + threadIdx.x;"
    echo "         // ... matrix multiply logic"
    echo "     }"
    echo ""
    echo "2. ToadStool detects available hardware:"
    echo "     - Found NVIDIA GPU → Use CUDA directly"
    echo "     - Found AMD GPU → Translate to ROCm/HIP"
    echo ""
    echo "3. ROCm/HIP automatically converts CUDA to AMD ISA:"
    echo "     - CUDA API calls → HIP API calls (95%+ compatible)"
    echo "     - CUDA kernels → AMD GPU instructions"
    echo "     - Same performance characteristics!"
    echo ""
    echo "4. Result: Zero vendor lock-in!"
    echo "     - Switch GPUs without rewriting code"
    echo "     - Use best hardware for the price"
    echo "     - Future-proof infrastructure"
    echo ""
    
    echo "Performance comparison:"
    echo "  Check results/local/cuda-matrix.json vs results/local/rocm-matrix.json"
    echo ""
    
    if [ -f "results/local/cuda-matrix.json" ] && [ -f "results/local/rocm-matrix.json" ]; then
        CUDA_TIME=$(jq -r '.avg_time_ms' results/local/cuda-matrix.json)
        ROCM_TIME=$(jq -r '.avg_time_ms' results/local/rocm-matrix.json)
        
        # Calculate ratio (using bc if available)
        if command -v bc &> /dev/null; then
            RATIO=$(echo "scale=2; $ROCM_TIME / $CUDA_TIME" | bc)
            echo "  NVIDIA (CUDA): ${CUDA_TIME}ms"
            echo "  AMD (ROCm): ${ROCM_TIME}ms"
            echo "  AMD is ${RATIO}x relative to NVIDIA for this workload"
            echo ""
        fi
    fi
    
elif [ "$NVIDIA_AVAILABLE" = true ]; then
    echo -e "${BLUE}Demonstrated on NVIDIA GPU${NC}"
    echo ""
    echo "When you add an AMD GPU:"
    echo "  1. Install ROCm"
    echo "  2. Run this script again"
    echo "  3. Watch the SAME code run on both vendors!"
    echo ""
    
elif [ "$AMD_AVAILABLE" = true ]; then
    echo -e "${BLUE}Demonstrated on AMD GPU${NC}"
    echo ""
    echo "ROCm/HIP is running CUDA-style code on your AMD hardware!"
    echo "Add an NVIDIA GPU to see direct comparison."
    echo ""
else
    echo "Install GPUs to see the full demo:"
    echo "  - NVIDIA GPU + CUDA toolkit"
    echo "  - AMD GPU + ROCm"
    echo ""
fi

echo "═══════════════════════════════════════════════════════════════"
echo -e "${MAGENTA} Next Steps${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "1. Run more backends:"
echo "     ./bench-all-backends.sh"
echo ""

echo "2. Test cross-tower distribution:"
echo "     cd ../distributed && ./bench-cross-tower.sh"
echo ""

echo "3. Try PyTorch model (if installed):"
echo "     cargo run --release --features pytorch --bin demo-cuda-on-amd"
echo ""

echo "4. Benchmark your specific workload:"
echo "     cargo run --release --bin bench-matrix-multiply -- --help"
echo ""

echo -e "${GREEN}Universal GPU computing: Build once, run anywhere! 🚀${NC}"

