#!/usr/bin/env bash
# Day 2: Distributed training across towers

set -euo pipefail

echo "🧠 ToadStool Deep Learning - Day 2: Distributed Training"
echo "================================================================"
echo ""
echo "📊 Model: ResNet-18 (11.7M parameters)"
echo "📦 Dataset: CIFAR-10 (60K images)"
echo "🎵 Federation: Songbird (Eastgate + Strandgate)"
echo "⚡ Strategy: Data parallelism with gradient averaging"
echo ""
echo "================================================================"
echo ""

# Check Songbird status
echo "🔍 Checking Songbird federation status..."
echo ""

if curl -sk https://localhost:8000/health &>/dev/null; then
    echo "✅ Eastgate tower: ONLINE"
else
    echo "❌ Eastgate tower: OFFLINE"
fi

if curl -sk https://192.168.1.134:8081/health &>/dev/null; then
    echo "✅ Strandgate tower: ONLINE"
else
    echo "❌ Strandgate tower: OFFLINE"
fi

echo ""
echo "Starting distributed training..."
echo ""

# Set library path
export LD_LIBRARY_PATH=/home/eastgate/.local/lib/python3.10/site-packages/torch/lib:$LD_LIBRARY_PATH

# Run distributed training
LIBTORCH_USE_PYTORCH=1 cargo run --release --bin train-distributed

echo ""
echo "🎉 Distributed training session complete!"
echo ""
echo "Check results:"
echo "  • Checkpoints: checkpoints/resnet18-distributed-*.pt"
echo "  • Metrics: outputs/distributed-metrics.json"
echo ""

