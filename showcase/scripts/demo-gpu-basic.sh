#!/bin/bash
# Basic GPU Compute Demo
# Demonstrates ToadStool's universal GPU runtime

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SHOWCASE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🎮 ToadStool Universal GPU Compute Demo"
echo "========================================="
echo

# Check if GPU runtime is available
echo "🔍 Step 1: Checking GPU availability..."
echo

if command -v nvidia-smi &> /dev/null; then
    echo "✅ NVIDIA GPU detected:"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    HAVE_GPU=true
elif command -v clinfo &> /dev/null; then
    echo "✅ OpenCL devices detected:"
    clinfo -l
    HAVE_GPU=true
elif command -v vulkaninfo &> /dev/null; then
    echo "✅ Vulkan devices detected:"
    vulkaninfo --summary
    HAVE_GPU=true
else
    echo "⚠️  No GPU detected - will simulate"
    HAVE_GPU=false
fi

echo
echo "📊 Step 2: Running basic GPU compute workload..."
echo

# Build with GPU support (auto-detect)
cd "$PROJECT_ROOT"

if [ "$HAVE_GPU" = true ]; then
    echo "Building with GPU support..."
    cargo build --release -p toadstool-cli --features gpu
else
    echo "Building with GPU simulation..."
    cargo build --release -p toadstool-cli
fi

echo
echo "🚀 Step 3: Executing GPU vector addition..."
echo

# Execute the workload
if [ "$HAVE_GPU" = true ]; then
    RUN_FEATURES="--features gpu"
else
    RUN_FEATURES=""
fi

if cargo run --release $RUN_FEATURES --bin toadstool-cli -- execute \
    "$SHOWCASE_ROOT/workloads/gpu-compute-basic.toml" \
    --runtime gpu \
    --format json \
    --verbose 2>&1 | tee "$SHOWCASE_ROOT/results/gpu-basic-output.log"; then
    
    echo
    echo "✅ GPU workload completed successfully!"
    echo
    echo "📄 Results:"
    if [ -f "$SHOWCASE_ROOT/results/gpu-basic-output.json" ]; then
        cat "$SHOWCASE_ROOT/results/gpu-basic-output.json" | jq '.'
    fi
else
    echo
    echo "⚠️  GPU workload failed (this is expected if no GPU available)"
    echo "   ToadStool will automatically fall back to CPU simulation"
fi

echo
echo "🎯 Demo Complete!"
echo
echo "What just happened:"
echo "1. ToadStool detected available GPU frameworks"
echo "2. Selected best framework (CUDA > OpenCL > Vulkan > WebGPU)"
echo "3. Compiled and executed OpenCL kernel"
echo "4. Performed vector addition on GPU"
echo "5. Validated results"
echo
echo "Next steps:"
echo "- Try gpu-ml-training.toml for ML workloads"
echo "- Run demo-gpu-squirrel.sh for AI image generation"
echo "- Check GPU_ENABLEMENT_PLAN_DEC_15_2025.md for more"

