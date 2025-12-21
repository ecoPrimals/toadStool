#!/bin/bash
# ToadStool Coverage Runner - Two-Tier Strategy
# Tier 1: Standard tests with llvm-cov (unit/integration)
# Tier 2: Robustness tests without coverage (E2E/chaos)

set -e

echo "🍄 ToadStool Coverage Runner - Month 2"
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
        "toadstool-cli"
        "toadstool-server"
        "toadstool-client"
        "toadstool-api"
        "toadstool-distributed"
        "toadstool-auto-config"
        "toadstool-security-sandbox"
        "toadstool-security-policies"
        "toadstool-security-monitoring"
        "toadstool-management-resources"
        "toadstool-management-monitoring"
        "toadstool-management-performance"
        "toadstool-management-analytics"
        "toadstool-runtime-wasm"
        "toadstool-runtime-native"
        "toadstool-runtime-python"
        "toadstool-runtime-container"
        "toadstool-runtime-gpu"
        "toadstool-integration-primals"
        "toadstool-integration-nestgate"
        "toadstool-integration-protocols"
        "toadstool-integration-orchestrator"
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
    
    # Get coverage percentage
    COVERAGE=$(cargo llvm-cov report | grep "TOTAL" | awk '{print $10}')
    echo ""
    echo -e "${GREEN}📊 Coverage: $COVERAGE${NC}"
    echo ""
}

run_tier2() {
    echo -e "${GREEN}🔥 TIER 2: Robustness Tests (Production Hardening)${NC}"
    echo "Running E2E and chaos tests (not measured in coverage)..."
    echo ""
    
    # E2E Tests
    echo -e "${YELLOW}Running E2E tests...${NC}"
    cargo test --test e2e_tests --test e2e_concurrent_integration_suite -- --test-threads=1
    cargo test e2e_workflow_week3 e2e_biome_lifecycle
    
    # Chaos Tests
    echo -e "${YELLOW}Running chaos tests...${NC}"
    cargo test --test fault_tests
    cd tests/chaos && cargo test --all
    cd ../..
    
    # Long-running E2E (optional, can take hours)
    if [ "${RUN_LONG_E2E:-false}" = "true" ]; then
        echo -e "${YELLOW}Running long-running E2E tests...${NC}"
        cargo test --test full_system_tests -- --ignored
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
    echo "  - Target: 75% by end of Month 2"
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

