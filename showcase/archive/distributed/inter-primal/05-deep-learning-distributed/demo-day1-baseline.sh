#!/usr/bin/env bash
# Day 1: ResNet-18 baseline training on CIFAR-10

set -euo pipefail

echo "🧠 ToadStool Deep Learning - Day 1: Baseline Training"
echo "================================================================"
echo ""
echo "📊 Model: ResNet-18 (11.7M parameters)"
echo "📦 Dataset: CIFAR-10 (60K images, 10 classes)"
echo "🎮 Device: CUDA (if available) or CPU"
echo "⏱️  Duration: ~5 minutes for 10 epochs (quick test)"
echo ""
echo "================================================================"
echo ""

# Check if CUDA is available
if ! python3 -c "import torch; print('CUDA:', torch.cuda.is_available())" 2>/dev/null; then
    echo "⚠️  Warning: Could not detect CUDA via PyTorch"
fi

echo ""
echo "Starting training..."
echo ""

# Run training
LIBTORCH_USE_PYTORCH=1 cargo run --release --bin train-single

echo ""
echo "🎉 Training complete!"
echo ""
echo "Check outputs:"
echo "  • Checkpoints: checkpoints/resnet18-cifar10-*.pt"
echo "  • Metrics: outputs/training-metrics.json"
echo ""

