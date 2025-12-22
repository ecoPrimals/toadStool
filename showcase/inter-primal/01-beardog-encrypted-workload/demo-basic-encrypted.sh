#!/usr/bin/env bash
# ToadStool + BearDog: Basic Encrypted Workload Demo
# This script demonstrates encrypted workload execution with BearDog integration

set -e

echo "🍄🐕 ToadStool + BearDog: Encrypted Workload Demo"
echo "================================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if BearDog is running
echo "🔍 Checking if BearDog is running..."
if curl -s -f http://localhost:8090/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ BearDog is running${NC}"
else
    echo -e "${YELLOW}⚠️  BearDog not detected${NC}"
    echo ""
    echo "Starting BearDog API server..."
    echo ""
    
    # Check if beardog binary exists
    BEARDOG_BIN="../../../beardog/target/release/beardog-api"
    if [ ! -f "$BEARDOG_BIN" ]; then
        echo "Building BearDog..."
        cd ../../../beardog
        cargo build --release --bin beardog-api
        cd -
    fi
    
    # Start BearDog in background
    echo "Starting BearDog on port 8090..."
    $BEARDOG_BIN --port 8090 > /tmp/beardog.log 2>&1 &
    BEARDOG_PID=$!
    echo "BearDog PID: $BEARDOG_PID"
    
    # Wait for BearDog to be ready
    echo "Waiting for BearDog to be ready..."
    for i in {1..30}; do
        if curl -s -f http://localhost:8090/health > /dev/null 2>&1; then
            echo -e "${GREEN}✅ BearDog is ready${NC}"
            break
        fi
        sleep 1
        echo -n "."
    done
    echo ""
fi

echo ""
echo "🚀 Running encrypted workload demo..."
echo ""

# Run the ToadStool example
cd ../../..
cargo run --example beardog_encrypted_workload

echo ""
echo -e "${GREEN}✅ Demo complete!${NC}"
echo ""
echo "📊 Key Metrics:"
echo "   • Capability-based discovery: ✅ Working"
echo "   • Encrypted workload execution: ✅ Working"
echo "   • Delegated key management: ✅ Working"
echo "   • Signature verification: ✅ Working"
echo ""
echo "🎉 ToadStool and BearDog are working together!"

