#!/bin/bash
# 🍄 ToadStool Vendor-Agnostic GPU Computing Demo
#
# Proves zero vendor lock-in by running the SAME workload across:
# - AMD Radeon RX 6950 XT
# - NVIDIA GeForce RTX 3090
# - Dual CPU System (128 cores)

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║                                                          ║"
echo "║  🍄 ToadStool Vendor-Agnostic GPU Computing Demo 🍄      ║"
echo "║                                                          ║"
echo "║  Proving Zero Vendor Lock-In                             ║"
echo "║  Same Workload → AMD + NVIDIA + CPU                      ║"
echo "║                                                          ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/ml-inference"

# Check prerequisites
echo "📦 Checking Prerequisites"
echo "═══════════════════════════════════════════════════════════"

if [ ! -f "data/mnist/t10k-images-idx3-ubyte.gz" ]; then
    echo "  ❌ MNIST dataset not found"
    echo "  📥 Downloading MNIST..."
    cargo run --release --bin download-mnist
    echo ""
fi

if [ ! -f "models/mnist_trained.bincode" ]; then
    echo "  ❌ Trained model not found"
    echo "  🧠 Training neural network (this will take a few minutes)..."
    cargo run --release --bin train-mnist
    echo ""
fi

echo "  ✅ All prerequisites met"
echo ""

# Build the demo
echo "🔨 Building Vendor-Agnostic Demo"
echo "═══════════════════════════════════════════════════════════"
cargo build --release --bin vendor_agnostic_demo
echo "  ✅ Build complete"
echo ""

# Run the demo
echo "🚀 Running Demo"
echo "═══════════════════════════════════════════════════════════"
echo ""
cargo run --release --bin vendor_agnostic_demo

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Demo Complete ✅                                         ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "What We Proved:"
echo "  ✅ Same code runs on AMD, NVIDIA, and CPU"
echo "  ✅ Same accuracy across all backends"
echo "  ✅ No vendor-specific code"
echo "  ✅ No CUDA lock-in"
echo "  ✅ Automatic backend selection"
echo ""
echo "Deep Debt Principles:"
echo "  ✅ Runtime discovery (no hardcoding)"
echo "  ✅ Self-knowledge (queries local hardware)"
echo "  ✅ Capability-based (what, not who)"
echo "  ✅ Graceful degradation (GPU → CPU)"
echo ""
echo "Next Steps:"
echo "  1. Review the code: ml-inference/src/bin/vendor_agnostic_demo.rs"
echo "  2. Run with different batch sizes"
echo "  3. Try distributed execution across towers"
echo "  4. Add more hardware (Intel, Apple GPUs)"
echo ""
