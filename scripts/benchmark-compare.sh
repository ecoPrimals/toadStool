#!/bin/bash
# Benchmark Comparison Tool
#
# Compares benchmark results between branches/commits
# Usage: ./scripts/benchmark-compare.sh <baseline-branch> <current-branch>

set -e

BASELINE_BRANCH="${1:-main}"
CURRENT_BRANCH="${2:-HEAD}"

echo "📊 Benchmark Comparison"
echo "======================="
echo ""
echo "Baseline: $BASELINE_BRANCH"
echo "Current:  $CURRENT_BRANCH"
echo ""

# Store current branch
ORIGINAL_BRANCH=$(git branch --show-current)

# Function to run benchmarks
run_benchmarks() {
    local branch=$1
    local output=$2
    
    echo "Running benchmarks on $branch..."
    git checkout "$branch" 2>/dev/null || true
    
    cd showcase/gpu-universal/ml-inference
    cargo bench --bench baseline_benchmarks -- --save-baseline "$output"
    cd ../../..
}

# Run baseline benchmarks
run_benchmarks "$BASELINE_BRANCH" "baseline"

# Run current benchmarks
run_benchmarks "$CURRENT_BRANCH" "current"

# Return to original branch
git checkout "$ORIGINAL_BRANCH" 2>/dev/null || true

echo ""
echo "✅ Benchmark comparison complete!"
echo ""
echo "View detailed comparison:"
echo "  cd showcase/gpu-universal/ml-inference"
echo "  cargo bench --bench baseline_benchmarks -- --baseline baseline"
