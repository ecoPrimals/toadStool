#!/bin/bash
# ToadStool Coverage Runner - Two-Tier Strategy
# Tier 1: Standard tests with llvm-cov (unit/integration)
# Tier 2: Robustness tests without coverage (E2E/chaos)

set -e

echo "ToadStool Coverage Runner"
echo "======================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse arguments
TIER="${1:-all}"
FORMAT="${2:-html}"

run_tier1() {
    echo -e "${GREEN}📊 TIER 1: Standard Tests (Coverage Measured)${NC}"
    echo "Running unit and integration tests with llvm-cov..."
    echo ""
    
    # Run per-crate to avoid performance test hangs
    # This is more robust than workspace-level coverage
    
    CRATES=(
        "toadstool-common"
        "toadstool-config"
        "toadstool"
        "toadstool-core"
        "toadstool-cli"
        "toadstool-client"
        "toadstool-server"
        "toadstool-distributed"
        "toadstool-auto-config"
        "toadstool-security-sandbox"
        "toadstool-security-policies"
        "toadstool-security-monitoring"
        "toadstool-management-performance"
        "toadstool-management-monitoring"
        "toadstool-management-analytics"
        "toadstool-management-resources"
        "toadstool-runtime-wasm"
        "toadstool-runtime-container"
        "toadstool-runtime-gpu"
        "toadstool-runtime-universal"
        "toadstool-runtime-adaptive"
        "toadstool-display"
        "toadstool-runtime-native"
        "toadstool-runtime-orchestration"
        "toadstool-runtime-python"
        "toadstool-runtime-secure-enclave"
        # toadstool-runtime-edge excluded (serialport→libudev C dep)
        "toadstool-runtime-specialty"
        "toadstool-integration-beardog"
        "toadstool-integration-primals"
        "toadstool-integration-nestgate"
        "toadstool-integration-protocols"
        "toadstool-testing"
    )
    
    # Clean previous coverage data
    cargo llvm-cov clean
    
    # Run coverage for each crate (avoids workspace hangs)
    for crate in "${CRATES[@]}"; do
        echo -e "${YELLOW}Testing: $crate${NC}"
        cargo llvm-cov \
            --package "$crate" \
            --no-report \
            --ignore-filename-regex "tests/" \
            -- --skip performance \
            2>&1 | grep -E "test result|error" || true
    done
    
    # Generate combined report
    echo ""
    echo -e "${GREEN}Generating coverage report...${NC}"
    
    if [ "$FORMAT" = "html" ]; then
        cargo llvm-cov report --html
        echo -e "${GREEN}✅ HTML report generated: target/llvm-cov/html/index.html${NC}"
    else
        cargo llvm-cov report
    fi
    
    # Get coverage percentage and enforce threshold
    COVERAGE=$(cargo llvm-cov report | grep "TOTAL" | awk '{print $10}')
    COVERAGE_NUM=$(echo "$COVERAGE" | sed 's/%//')
    THRESHOLD=90
    
    echo ""
    echo -e "${GREEN}📊 Coverage: $COVERAGE${NC}"
    echo -e "   Target: ${THRESHOLD}% (wateringHole standard)"
    echo ""
    
    # Enforce coverage threshold if ENFORCE_COVERAGE=true
    if [ "${ENFORCE_COVERAGE:-false}" = "true" ]; then
        if (( $(echo "$COVERAGE_NUM < $THRESHOLD" | bc -l) )); then
            echo -e "${RED}❌ Coverage ${COVERAGE_NUM}% is below threshold ${THRESHOLD}%${NC}"
            exit 1
        else
            echo -e "${GREEN}✅ Coverage ${COVERAGE_NUM}% meets threshold ${THRESHOLD}%${NC}"
        fi
    fi
}

run_tier2() {
    echo -e "${GREEN}🔥 TIER 2: Robustness Tests (Production Hardening)${NC}"
    echo "Running E2E and chaos tests (not measured in coverage)..."
    echo ""
    
    # E2E Tests
    echo -e "${YELLOW}Running E2E tests...${NC}"
    cargo test -p toadstool-integration-tests -- --test-threads=1 2>&1 | grep -E "test result|error" || true
    
    # Chaos Tests
    echo -e "${YELLOW}Running chaos tests...${NC}"
    cargo test -p toadstool-cli --test chaos_resource_scenarios_week4 2>&1 | grep -E "test result|error" || true
    cargo test -p toadstool-testing -- --test-threads=1 2>&1 | grep -E "test result|error" || true
    
    # Long-running E2E (optional, can take hours)
    if [ "${RUN_LONG_E2E:-false}" = "true" ]; then
        echo -e "${YELLOW}Running long-running E2E tests...${NC}"
        cargo test --workspace -- --ignored --test-threads=1 2>&1 | grep -E "test result|error" || true
    fi
    
    echo ""
    echo -e "${GREEN}✅ Tier 2 tests complete${NC}"
    echo ""
}

show_summary() {
    echo ""
    echo "======================================"
    echo "📊 Two-Tier Testing Summary"
    echo "======================================"
    echo ""
    echo "Tier 1 (Coverage): Unit & Integration tests"
    echo "  - Measured by llvm-cov"
    echo "  - Target: 90% (wateringHole standard)"
    echo "  - Current: Run './scripts/run-coverage.sh tier1' to measure"
    echo ""
    echo "Tier 2 (Robustness): E2E & Chaos tests"
    echo "  - NOT measured in coverage (too slow/complex)"
    echo "  - Critical for production hardening"
    echo "  - Run './scripts/run-coverage.sh tier2' to execute"
    echo ""
    echo "Full suite: './scripts/run-coverage.sh all'"
    echo ""
}

# Main execution
case "$TIER" in
    tier1)
        run_tier1
        ;;
    tier2)
        run_tier2
        ;;
    all)
        run_tier1
        run_tier2
        show_summary
        ;;
    *)
        echo "Usage: $0 [tier1|tier2|all] [html|text]"
        echo ""
        echo "  tier1: Run standard tests with coverage (unit/integration)"
        echo "  tier2: Run robustness tests without coverage (E2E/chaos)"
        echo "  all:   Run both tiers (default)"
        echo ""
        echo "Examples:"
        echo "  $0 tier1 html    # Run coverage tests, generate HTML report"
        echo "  $0 tier2         # Run robustness tests only"
        echo "  $0 all           # Run complete test suite"
        exit 1
        ;;
esac

echo -e "${GREEN}✅ Coverage run complete!${NC}"

