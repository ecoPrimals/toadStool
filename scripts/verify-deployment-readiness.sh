#!/bin/bash
# Deployment Readiness Verification Script
# Verifies all quality gates before staging deployment

set -e

echo "🍄 ToadStool Deployment Readiness Check"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FAILED=0

# Function to check a gate
check_gate() {
    local name="$1"
    local command="$2"
    
    echo -n "Checking $name... "
    
    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}"
        return 0
    else
        echo -e "${RED}❌ FAIL${NC}"
        FAILED=1
        return 1
    fi
}

echo "📋 Quality Gate Checks:"
echo ""

# Gate 1: Formatting
check_gate "Formatting" "cargo fmt --check"

# Gate 2: Production Linting
check_gate "Production Linting" "cargo clippy --lib --all-features -- -D warnings"

# Gate 3: Tests
check_gate "Tests" "cargo test --lib"

# Gate 4: Build (lib only, examples may have warnings)
check_gate "Library Build" "cargo build --lib"

echo ""
echo "📊 Metrics:"
echo ""

# Coverage
echo -n "Test Coverage: "
if command -v cargo-llvm-cov &> /dev/null; then
    COVERAGE=$(cargo llvm-cov --lib --summary-only 2>/dev/null | grep "TOTAL" | awk '{print $10}')
    echo -e "${YELLOW}$COVERAGE${NC} (target: 90% for production)"
else
    echo -e "${YELLOW}Not measured${NC} (install cargo-llvm-cov)"
fi

# Test count
TEST_COUNT=$(cargo test --lib 2>&1 | grep "test result:" | tail -1 | awk '{print $4}')
echo "Tests Passing: ${GREEN}$TEST_COUNT${NC}"

# File count
FILE_COUNT=$(find crates -name "*.rs" -type f | wc -l)
echo "Rust Files: $FILE_COUNT"

# Crate count
CRATE_COUNT=$(find crates -name "Cargo.toml" -type f | wc -l)
echo "Crates: $CRATE_COUNT"

echo ""
echo "🔍 Code Quality:"
echo ""

# Unsafe blocks
UNSAFE_COUNT=$(grep -r "unsafe" crates/ --include="*.rs" 2>/dev/null | grep -v "test\|comment" | wc -l)
if [ "$UNSAFE_COUNT" -eq 0 ]; then
    echo -e "Unsafe Blocks: ${GREEN}0${NC} 🏆"
else
    echo -e "Unsafe Blocks: ${RED}$UNSAFE_COUNT${NC}"
    FAILED=1
fi

# TODOs in production
TODO_COUNT=$(grep -r "TODO\|FIXME" crates/ --include="*.rs" 2>/dev/null | grep -v "tests/" | wc -l)
echo "TODOs (production): $TODO_COUNT"

echo ""
echo "========================================"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL GATES PASSED${NC}"
    echo ""
    echo "🚀 Ready for staging deployment!"
    echo ""
    echo "To deploy, run:"
    echo "  ./deploy-to-staging.sh"
    echo ""
    exit 0
else
    echo -e "${RED}❌ SOME GATES FAILED${NC}"
    echo ""
    echo "Please fix the issues above before deploying."
    echo ""
    exit 1
fi

