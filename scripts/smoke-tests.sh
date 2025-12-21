#!/usr/bin/env bash
# Smoke Tests for ToadStool Staging Deployment
# Run immediately after deployment to verify basic functionality

set -e

# Configuration
STAGING_HOST="${STAGING_HOST:-localhost}"
STAGING_PORT="${STAGING_PORT:-8080}"
BASE_URL="http://${STAGING_HOST}:${STAGING_PORT}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 ToadStool Staging Smoke Tests"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Target: ${BASE_URL}"
echo "Started: $(date)"
echo ""

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_endpoint() {
    local name="$1"
    local endpoint="$2"
    local expected_code="${3:-200}"
    
    echo -n "Testing ${name}... "
    
    if response=$(curl -s -w "\n%{http_code}" "${BASE_URL}${endpoint}" 2>&1); then
        http_code=$(echo "$response" | tail -n1)
        body=$(echo "$response" | head -n-1)
        
        if [ "$http_code" = "$expected_code" ]; then
            echo "✅ PASS (HTTP ${http_code})"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        else
            echo "❌ FAIL (Expected ${expected_code}, got ${http_code})"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi
    else
        echo "❌ FAIL (Connection error)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Run tests
echo "Running smoke tests..."
echo ""

echo "📡 Connectivity Tests:"
test_endpoint "Health Check" "/health"
test_endpoint "Readiness Check" "/ready"
test_endpoint "Metrics Endpoint" "/metrics"

echo ""
echo "🔧 API Tests:"
test_endpoint "Cluster Status" "/api/v1/cluster/status"
test_endpoint "System Info" "/api/v1/system/info" 200

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Test Results:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Passed: ${TESTS_PASSED}"
echo "Failed: ${TESTS_FAILED}"
echo "Total:  $((TESTS_PASSED + TESTS_FAILED))"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo "✅ All smoke tests passed!"
    echo "🎉 Deployment verification successful!"
    exit 0
else
    echo "❌ Some tests failed!"
    echo "⚠️  Please investigate before proceeding."
    exit 1
fi

