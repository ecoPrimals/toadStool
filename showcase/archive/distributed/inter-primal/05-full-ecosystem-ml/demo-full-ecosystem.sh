#!/usr/bin/env bash
# Demo: Full ecosystem ML pipeline with all 5 primals

set -e

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DEMO_DIR"

echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║     🌐 FULL ECOSYSTEM ML PIPELINE 🌐                         ║"
echo "║                                                              ║"
echo "║     All 5 Primals Working Together!                          ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo

echo "🔨 Building demo..."
cargo build --release --bin full-ecosystem-demo 2>&1 | grep -v "^   Compiling\|^   Finished" || true
echo "✅ Build complete"
echo

echo "🚀 Running full ecosystem demonstration..."
echo
cargo run --release --bin full-ecosystem-demo

echo
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║     ✅ ECOSYSTEM DEMONSTRATION COMPLETE! ✅                   ║"
echo "║                                                              ║"
echo "║  This is the ecoPrimals vision in action:                   ║"
echo "║  5 primals, 1 unified ecosystem, infinite possibilities!    ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo

