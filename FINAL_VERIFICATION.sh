#!/bin/bash
# Final Verification Script - November 17, 2025
# Verifies all quality gates before deployment

set -e

echo "🍄 ToadStool - Final Verification"
echo "=================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0

# Function to check and report
check() {
    local name="$1"
    local cmd="$2"
    
    echo -n "Checking $name... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ FAIL${NC}"
        ((FAILED++))
        return 1
    fi
}

echo "📋 Running Quality Gates..."
echo ""

# 1. Formatting
check "Formatting" "cargo fmt --all --check"

# 2. Clippy
check "Clippy (strict)" "cargo clippy --all-targets --all-features -- -D warnings"

# 3. Build
check "Build" "cargo build --release --quiet"

# 4. Tests (sample)
check "Core tests" "cargo test --package toadstool --lib --quiet"
check "API tests" "cargo test --package toadstool-api --lib --quiet"
check "CLI tests" "cargo test --package toadstool-cli --lib --quiet"

# 5. Doc generation
check "Documentation" "cargo doc --no-deps --quiet"

echo ""
echo "=================================="
echo "Results:"
echo -e "${GREEN}✅ Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ Failed: $FAILED${NC}"
    echo ""
    echo -e "${RED}⚠️  NOT READY FOR DEPLOYMENT${NC}"
    exit 1
else
    echo -e "${RED}❌ Failed: $FAILED${NC}"
    echo ""
    echo -e "${GREEN}🎉 ALL QUALITY GATES PASSING${NC}"
    echo -e "${GREEN}✅ PRODUCTION READY - DEPLOY NOW${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review: READY_TO_DEPLOY_NOW.md"
    echo "  2. Deploy: ./DEPLOYMENT_COMMAND.sh"
    echo "  3. Monitor: ./quick-monitor.sh"
    exit 0
fi

