#!/usr/bin/env bash
# Day 5: Scale to 95%+ accuracy with 100 epochs

set -euo pipefail

echo "🧠 Day 5: Scaling to 95%+ Accuracy"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Configuration:"
echo "  • Model: ResNet-18"
echo "  • Dataset: CIFAR-10"
echo "  • Epochs: 100 (with early stopping)"
echo "  • Batch size: 128"
echo "  • Optimizer: SGD with momentum (0.9) + weight decay (5e-4)"
echo "  • LR schedule: Warmup (5 epochs) + Cosine decay"
echo "  • Data augmentation: Random horizontal flip"
echo "  • Target: 95%+ test accuracy"
echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""

# Set PyTorch library path
PYTORCH_PATH=$(python3 -c "import torch; print(torch.__path__[0])")
export LD_LIBRARY_PATH="$PYTORCH_PATH/lib:${LD_LIBRARY_PATH:-}"

# Set environment for PyTorch discovery
export LIBTORCH_USE_PYTORCH=1

# Build the scaling binary
echo "📦 Building training binary..."
cargo build --release --bin train-scale
echo ""

# Run training
echo "🚀 Starting 100-epoch training run..."
echo "   (This will take approximately 30-60 minutes depending on GPU)"
echo ""

./target/release/train-scale

# Check if target was reached
if [ -f "outputs/DAY5_SCALING_REPORT.md" ]; then
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "📊 Training Summary"
    echo "═══════════════════════════════════════════════════════════"
    cat outputs/DAY5_SCALING_REPORT.md
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "✅ Day 5 Complete!"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Generated files:"
echo "  • checkpoints/resnet18-cifar10-best.pt (best model)"
echo "  • checkpoints/resnet18-cifar10-final.pt (final model)"
echo "  • outputs/training-metrics-100epoch.json (all metrics)"
echo "  • outputs/DAY5_SCALING_REPORT.md (summary report)"
echo ""

