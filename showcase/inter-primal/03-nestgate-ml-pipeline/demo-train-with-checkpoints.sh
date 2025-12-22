#!/usr/bin/env bash
# Demo: Train MNIST with checkpoint saving to NestGate

set -e

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DEMO_DIR"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 ToadStool + NestGate: Training with Checkpoints"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check if NestGate is available
echo "🔍 Checking for NestGate storage service..."
if curl -sf http://localhost:8084/health > /dev/null 2>&1; then
    echo "✅ NestGate is running"
else
    echo "⚠️  NestGate not detected at localhost:8084"
    echo "   Demo will run in local-only mode"
    echo "   (Checkpoints won't actually be saved)"
fi
echo

# Build the demo
echo "🔨 Building demo..."
cargo build --release --bin train-with-checkpoints
echo "✅ Build complete"
echo

# Run training
echo "🎯 Starting training with checkpoint saving..."
echo
cargo run --release --bin train-with-checkpoints

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Demo complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

