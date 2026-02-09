#!/usr/bin/env bash
# NPU vs GPU Raytracing Comparison Demo
# Deep Debt: Uses ToadStool for hardware discovery

set -euo pipefail

echo "╔══════════════════════════════════════════════════════╗"
echo "║   NPU vs GPU Raytracing Comparison                  ║"
echo "║   Demonstrates ToadStool + BarraCUDA Architecture    ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# Build
echo "[1/3] Building raytracing showcase..."
cargo build --release --quiet

# Run comparison
echo ""
echo "[2/3] Running benchmark comparison..."
echo ""
cargo run --release --example compare_raytracing

# Summary
echo ""
echo "[3/3] Summary:"
echo "  ✓ ToadStool discovered hardware"
echo "  ✓ Compared NPU (sparse) vs GPU (dense)"
echo "  ✓ Demonstrated workload-specific selection"
echo ""
echo "See README.md for details"
echo ""
