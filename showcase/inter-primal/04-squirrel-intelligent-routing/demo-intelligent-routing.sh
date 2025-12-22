#!/usr/bin/env bash
# Demo: Intelligent workload routing with Squirrel AI

set -e

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DEMO_DIR"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🐿️  ToadStool + Squirrel: Intelligent Routing"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check if Squirrel is available
echo "🔍 Checking for Squirrel AI service..."
if curl -sf http://localhost:8085/health > /dev/null 2>&1; then
    echo "✅ Squirrel AI is running"
else
    echo "⚠️  Squirrel not detected at localhost:8085"
    echo "   Demo will run with rule-based fallback"
    echo "   (AI predictions won't be available)"
fi
echo

# Build the demo
echo "🔨 Building demo..."
cargo build --release --bin demo-intelligent-routing
echo "✅ Build complete"
echo

# Run the demo
echo "🎯 Starting intelligent routing demo..."
echo
cargo run --release --bin demo-intelligent-routing

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Demo complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "💡 Try running this demo multiple times to see AI learning!"

