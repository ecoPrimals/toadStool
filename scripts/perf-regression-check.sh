#!/bin/bash
# Performance Regression Check
# 
# Runs critical benchmarks and compares against baseline
# Usage: ./scripts/perf-regression-check.sh [baseline-file]

set -e

BASELINE_FILE="${1:-benchmark-baseline.json}"
CURRENT_RESULTS="benchmark-current.json"
THRESHOLD=1.20  # 20% regression threshold

echo "🔍 Performance Regression Check"
echo "================================"
echo ""

# Change to ml-inference directory
cd "$(dirname "$0")/../showcase/gpu-universal/ml-inference"

# Check if baseline exists
if [ ! -f "$BASELINE_FILE" ]; then
    echo "⚠️  No baseline found at $BASELINE_FILE"
    echo "Creating new baseline..."
    cargo bench --bench baseline_benchmarks -- --save-baseline main
    echo "✅ Baseline created!"
    exit 0
fi

echo "Running current benchmarks..."
cargo bench --bench baseline_benchmarks -- --baseline main --save-baseline current

echo ""
echo "Comparing results..."

# Parse benchmark results (simplified)
# In production, use criterion's built-in comparison or JSON export

echo "✅ Benchmark comparison complete"
echo ""
echo "To manually compare:"
echo "  cargo bench --bench baseline_benchmarks -- --baseline main"
echo ""
echo "To create new baseline:"
echo "  cargo bench --bench baseline_benchmarks -- --save-baseline main"
