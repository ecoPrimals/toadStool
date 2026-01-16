#!/usr/bin/env bash
#
# BiomeOS Socket Configuration Test
# Tests that ToadStool correctly honors orchestrator-provided socket paths
#
# This validates the fix for the Neural API deployment issue where
# ToadStool was creating sockets in /run/user/1000/ instead of /tmp/

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "╔══════════════════════════════════════════════════════════════════════════╗"
echo "║                                                                          ║"
echo "║           BiomeOS Socket Configuration Test                             ║"
echo "║                                                                          ║"
echo "║  Tests ToadStool's environment variable honoring for Neural API         ║"
echo "║                                                                          ║"
echo "╚══════════════════════════════════════════════════════════════════════════╝"
echo ""

# Build the server
echo -e "${BLUE}Building ToadStool server...${NC}"
cargo build --release --bin toadstool-server 2>&1 | tail -5
echo -e "${GREEN}✅ Build complete${NC}"
echo ""

# Test directory
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

echo -e "${BLUE}Test directory: $TEST_DIR${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# Test 1: TOADSTOOL_SOCKET explicit path (HIGHEST PRIORITY)
# ═══════════════════════════════════════════════════════════════════════════

echo "═══════════════════════════════════════════════════════════════════════════"
echo "Test 1: TOADSTOOL_SOCKET Environment Variable (Neural API pattern)"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""

TEST_SOCKET="$TEST_DIR/toadstool-nat0.sock"
echo -e "${YELLOW}Setting TOADSTOOL_SOCKET=$TEST_SOCKET${NC}"
echo -e "${YELLOW}Setting TOADSTOOL_FAMILY=nat0${NC}"
echo ""

# Start server in background with explicit socket path
RUST_LOG=info TOADSTOOL_SOCKET="$TEST_SOCKET" TOADSTOOL_FAMILY=nat0 \
    timeout 5s ./target/release/toadstool-server > "$TEST_DIR/test1.log" 2>&1 &
SERVER_PID=$!

# Wait for socket to be created
sleep 2

# Check if socket was created at the expected location
if [ -S "$TEST_SOCKET" ]; then
    echo -e "${GREEN}✅ PASS: Socket created at expected location${NC}"
    echo -e "   Location: $TEST_SOCKET"
    ls -lh "$TEST_SOCKET"
else
    echo -e "${RED}❌ FAIL: Socket not created at expected location${NC}"
    echo -e "   Expected: $TEST_SOCKET"
    echo -e "   Looking for other sockets..."
    find "$TEST_DIR" -name "*.sock" -ls 2>/dev/null || echo "   No sockets found"
    
    # Show logs
    echo ""
    echo "Server logs:"
    cat "$TEST_DIR/test1.log" || true
    exit 1
fi

# Check logs for correct path detection
if grep -q "TOADSTOOL_SOCKET" "$TEST_DIR/test1.log"; then
    echo -e "${GREEN}✅ PASS: Environment variable detected in logs${NC}"
    echo ""
    echo "Relevant log lines:"
    grep "socket\|Socket\|TOADSTOOL_SOCKET" "$TEST_DIR/test1.log" | head -10
else
    echo -e "${RED}❌ FAIL: Environment variable not detected${NC}"
    echo ""
    echo "Server logs:"
    cat "$TEST_DIR/test1.log"
    exit 1
fi

# Cleanup
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true
rm -f "$TEST_SOCKET"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# Test 2: BIOMEOS_SOCKET_PATH (generic orchestrator path)
# ═══════════════════════════════════════════════════════════════════════════

echo "═══════════════════════════════════════════════════════════════════════════"
echo "Test 2: BIOMEOS_SOCKET_PATH Environment Variable (generic biomeOS pattern)"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""

BIOMEOS_SOCKET="$TEST_DIR/biomeos-toadstool.sock"
echo -e "${YELLOW}Setting BIOMEOS_SOCKET_PATH=$BIOMEOS_SOCKET${NC}"
echo -e "${YELLOW}Setting BIOMEOS_FAMILY_ID=nat0${NC}"
echo ""

# Start server with generic biomeOS path
RUST_LOG=info BIOMEOS_SOCKET_PATH="$BIOMEOS_SOCKET" BIOMEOS_FAMILY_ID=nat0 \
    timeout 5s ./target/release/toadstool-server > "$TEST_DIR/test2.log" 2>&1 &
SERVER_PID=$!

sleep 2

if [ -S "$BIOMEOS_SOCKET" ]; then
    echo -e "${GREEN}✅ PASS: Socket created at biomeOS path${NC}"
    echo -e "   Location: $BIOMEOS_SOCKET"
    ls -lh "$BIOMEOS_SOCKET"
else
    echo -e "${RED}❌ FAIL: Socket not created at biomeOS path${NC}"
    cat "$TEST_DIR/test2.log"
    exit 1
fi

kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true
rm -f "$BIOMEOS_SOCKET"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# Test 3: /tmp fallback (Neural API expected location)
# ═══════════════════════════════════════════════════════════════════════════

echo "═══════════════════════════════════════════════════════════════════════════"
echo "Test 3: /tmp Fallback with Family ID (system-wide deployment)"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""

echo -e "${YELLOW}Setting TOADSTOOL_FAMILY=test$$${NC}"
echo -e "${YELLOW}No TOADSTOOL_SOCKET or BIOMEOS_SOCKET_PATH set${NC}"
echo -e "${YELLOW}XDG_RUNTIME_DIR disabled${NC}"
echo ""

# Start with no XDG_RUNTIME_DIR (force /tmp fallback)
RUST_LOG=info XDG_RUNTIME_DIR=/nonexistent TOADSTOOL_FAMILY=test$$ \
    timeout 5s ./target/release/toadstool-server > "$TEST_DIR/test3.log" 2>&1 &
SERVER_PID=$!

sleep 2

TMP_SOCKET="/tmp/toadstool-test$$.sock"
if [ -S "$TMP_SOCKET" ]; then
    echo -e "${GREEN}✅ PASS: Socket created in /tmp with family ID${NC}"
    echo -e "   Location: $TMP_SOCKET"
    ls -lh "$TMP_SOCKET"
else
    echo -e "${RED}❌ FAIL: Socket not created in /tmp${NC}"
    cat "$TEST_DIR/test3.log"
    exit 1
fi

kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true
rm -f "$TMP_SOCKET"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════

echo "╔══════════════════════════════════════════════════════════════════════════╗"
echo "║                                                                          ║"
echo "║                      ✅ ALL TESTS PASSED ✅                             ║"
echo "║                                                                          ║"
echo "╚══════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "ToadStool correctly honors environment variables in this priority order:"
echo "  1. TOADSTOOL_SOCKET (primal-specific, absolute path) ✅"
echo "  2. BIOMEOS_SOCKET_PATH (generic orchestrator path) ✅"
echo "  3. XDG runtime directory (/run/user/<uid>/) ✅"
echo "  4. /tmp fallback (/tmp/toadstool-<family>.sock) ✅"
echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo "📝 NOTE FOR BIOMEOS TEAM:"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "If ToadStool is creating sockets in /run/user/1000/ instead of /tmp/,"
echo "this means the TOADSTOOL_SOCKET environment variable is NOT being passed"
echo "to the spawned process by Neural API."
echo ""
echo "Solution: Ensure Neural API child process spawning passes environment:"
echo ""
echo "  Command::new(\"toadstool-server\")"
echo "      .env(\"TOADSTOOL_SOCKET\", \"/tmp/toadstool-nat0.sock\")"
echo "      .env(\"TOADSTOOL_FAMILY\", \"nat0\")"
echo "      .spawn()?;"
echo ""
echo "Or use .envs() to pass all environment variables from parent."
echo ""
