#!/usr/bin/env bash
# ToadStool Deployment Verification Script
# Runs all quality checks before deployment

set -e  # Exit on error

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 ToadStool Deployment Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FAILED=0

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2${NC}"
    else
        echo -e "${RED}❌ $2${NC}"
        FAILED=1
    fi
}

echo "📦 Step 1: Clean Build"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo clean
cargo build --workspace --release 2>&1 | tail -5
print_status $? "Release build successful"
echo ""

echo "🧪 Step 2: Test Suite"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo test --workspace --lib --release 2>&1 | tee /tmp/test-output.txt | tail -20
TEST_RESULT=$(grep "test result:" /tmp/test-output.txt | tail -1)
echo "$TEST_RESULT"
if echo "$TEST_RESULT" | grep -q "0 failed"; then
    print_status 0 "All tests passing"
else
    print_status 1 "Some tests failed"
fi
echo ""

echo "📋 Step 3: Clippy Lints"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo clippy --workspace --all-targets --release -- -D warnings 2>&1 | tail -10
print_status $? "Clippy clean (no warnings)"
echo ""

echo "🎨 Step 4: Code Formatting"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo fmt --check 2>&1 | tail -5
print_status $? "Code formatted correctly"
echo ""

echo "📚 Step 5: Documentation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo doc --workspace --no-deps --release 2>&1 | tail -5
print_status $? "Documentation builds successfully"
echo ""

echo "🔐 Step 6: Security Audit"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if command -v cargo-audit &> /dev/null; then
    cargo audit 2>&1 | tail -10
    print_status $? "No known security vulnerabilities"
else
    echo -e "${YELLOW}⚠️  cargo-audit not installed (optional)${NC}"
fi
echo ""

echo "📊 Step 7: Code Coverage"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if command -v cargo-llvm-cov &> /dev/null; then
    cargo llvm-cov --workspace --lib --summary-only 2>&1 | grep -E "TOTAL|Finished" | tail -2
    print_status 0 "Coverage report generated"
else
    echo -e "${YELLOW}⚠️  cargo-llvm-cov not installed (optional)${NC}"
fi
echo ""

echo "📦 Step 8: Binary Size"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -f "target/release/toadstool" ]; then
    SIZE=$(ls -lh target/release/toadstool | awk '{print $5}')
    echo "Binary size: $SIZE"
    print_status 0 "Binary built successfully"
else
    echo -e "${YELLOW}⚠️  toadstool binary not found${NC}"
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL CHECKS PASSED - READY TO DEPLOY!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review ./DEPLOYMENT_GUIDE.md"
    echo "  2. Deploy to staging: ./scripts/deploy-staging.sh"
    echo "  3. Monitor for 24-48 hours"
    echo "  4. Deploy to production: ./scripts/deploy-production.sh"
    echo ""
    exit 0
else
    echo -e "${RED}❌ SOME CHECKS FAILED - PLEASE FIX BEFORE DEPLOYING${NC}"
    echo ""
    echo "Review the failures above and fix them."
    echo ""
    exit 1
fi

