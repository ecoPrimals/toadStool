#!/usr/bin/env bash
# Dual-GPU Demo Runner
# Demonstrates vendor-agnostic GPU compute orchestration

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ToadStool: Vendor-Agnostic GPU Compute                 ║"
echo "║  Phase 1: GPU Discovery & Orchestration                 ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Check if data exists
if [ ! -f "data/mnist/t10k-images-idx3-ubyte.gz" ]; then
    echo "📊 MNIST data not found. Downloading..."
    cargo run --bin download-mnist
    echo ""
fi

# Build the demo
echo "🔨 Building dual-GPU demo..."
cargo build --release --bin dual-gpu-demo --features all-gpus
echo ""

# Run the demo
echo "🚀 Running demo..."
echo ""
cargo run --release --bin dual-gpu-demo --features all-gpus

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Demo Complete!                                          ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "📖 Read PHASE1_COMPLETE.md for detailed analysis"
echo "🔧 Read SETUP_DUAL_GPU.md for AMD GPU setup"
echo ""

