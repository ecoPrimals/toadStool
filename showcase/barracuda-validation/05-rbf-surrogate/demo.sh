#!/usr/bin/env bash
set -euo pipefail

echo "╔══════════════════════════════════════════════════════╗"
echo "║   RBF Surrogate Learning - GPU Accelerated          ║"
echo "║   hotSpring Physics Integration Demo                ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")"

echo "[Building...]"
cargo build --release --quiet

echo "[Running demo...]"
echo ""
cargo run --release

echo ""
echo "✅ Demo complete!"
echo ""
echo "📚 Read more:"
echo "  - README.md (this showcase)"
echo "  - ../../../RBF_SURROGATE_COMPLETE.md (technical details)"
echo "  - ../../../BARRACUDA_SCIENTIFIC_COMPUTING.md (full guide)"
