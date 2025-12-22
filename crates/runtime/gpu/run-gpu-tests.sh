#!/bin/bash
# GPU Test Runner
# Runs GPU tests in isolation to avoid wgpu duplicate symbol linker errors
#
# Usage: ./run-gpu-tests.sh [test-name]

set -e

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$CRATE_DIR"

echo "🔧 ToadStool GPU Runtime - Isolated Test Runner"
echo "================================================"
echo ""
echo "Note: GPU tests must run in isolation due to wgpu upstream linker conflicts"
echo "Issue: https://github.com/gfx-rs/wgpu/issues/..."
echo ""

# Function to run a single test
run_test() {
    local test_name="$1"
    echo "▶️  Running: $test_name"
    cargo test --lib "$test_name" -- --nocapture
    echo "✅ Passed: $test_name"
    echo ""
}

# If test name provided, run just that one
if [ $# -gt 0 ]; then
    run_test "$1"
    exit 0
fi

# Otherwise, run all GPU tests individually
echo "Running all GPU tests in isolation..."
echo ""

# Get list of GPU test names
TEST_LIST=$(cargo test --lib -- --list --format terse 2>/dev/null | grep ': test$' | sed 's/: test$//')

TEST_COUNT=$(echo "$TEST_LIST" | wc -l)
PASSED=0
FAILED=0

echo "Found $TEST_COUNT GPU tests"
echo ""

for test in $TEST_LIST; do
    if cargo test --lib "$test" -- --quiet 2>&1 | grep -q "test result: ok"; then
        echo "✅ $test"
        ((PASSED++))
    else
        echo "❌ $test"
        ((FAILED++))
    fi
done

echo ""
echo "================================================"
echo "Results: $PASSED passed, $FAILED failed (out of $TEST_COUNT)"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "🎉 All GPU tests passed!"
    exit 0
else
    echo "⚠️  Some GPU tests failed"
    exit 1
fi

