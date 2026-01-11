#!/usr/bin/env bash
# ToadStool Socket Configuration Tests
# biomeOS Standardization Requirements

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 🧪 ToadStool Socket Configuration Tests"
echo " biomeOS Primal Standardization Requirements"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TOADSTOOL_BIN="./target/release/toadstool-server"

if [ ! -f "$TOADSTOOL_BIN" ]; then
    echo "❌ ToadStool binary not found. Run: cargo build --release"
    exit 1
fi

echo "Using ToadStool binary: $TOADSTOOL_BIN"
echo ""

# Helper function to clean up background processes
cleanup() {
    echo "🧹 Cleaning up test processes and sockets..."
    pkill -f "toadstool" || true
    rm -f /tmp/test-socket*.sock
    rm -f /tmp/toadstool-*.sock
    sleep 1
}

trap cleanup EXIT

# Test 1: Environment Variable Override
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 1: TOADSTOOL_SOCKET Environment Variable Override"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TEST_SOCKET="/tmp/test-socket-override.sock"
rm -f "$TEST_SOCKET"

echo "Setting TOADSTOOL_SOCKET=$TEST_SOCKET"
echo "Starting ToadStool..."

export TOADSTOOL_SOCKET="$TEST_SOCKET"
export TOADSTOOL_FAMILY="test0"

timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test1.log || true

sleep 1

if [ -S "$TEST_SOCKET" ]; then
    echo "✅ Test 1 PASSED: Socket created at specified path: $TEST_SOCKET"
    ls -lh "$TEST_SOCKET"
else
    echo "❌ Test 1 FAILED: Socket not found at: $TEST_SOCKET"
    echo "Logs:"
    cat /tmp/toadstool-test1.log
    exit 1
fi

unset TOADSTOOL_SOCKET
unset TOADSTOOL_FAMILY
cleanup
echo ""

# Test 2: XDG Runtime Directory
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 2: XDG Runtime Directory (Standard)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

export TOADSTOOL_FAMILY="xdg0"
EXPECTED_SOCKET="/run/user/$(id -u)/toadstool-xdg0.sock"

echo "Setting TOADSTOOL_FAMILY=xdg0"
echo "Expected socket: $EXPECTED_SOCKET"
echo "Starting ToadStool..."

timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test2.log || true

sleep 1

if [ -S "$EXPECTED_SOCKET" ]; then
    echo "✅ Test 2 PASSED: Socket created in XDG runtime directory"
    ls -lh "$EXPECTED_SOCKET"
    rm -f "$EXPECTED_SOCKET"
elif [ -S "/tmp/toadstool-xdg0-default.sock" ]; then
    echo "⚠️  Test 2 FALLBACK: XDG not available, used /tmp"
    echo "    (This is OK on systems without XDG runtime directory)"
    ls -lh "/tmp/toadstool-xdg0-default.sock"
else
    echo "❌ Test 2 FAILED: Socket not found"
    echo "Logs:"
    cat /tmp/toadstool-test2.log
    exit 1
fi

unset TOADSTOOL_FAMILY
cleanup
echo ""

# Test 3: Fallback to /tmp
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 3: Fallback to /tmp (No XDG)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

export XDG_RUNTIME_DIR="/nonexistent"
export TOADSTOOL_FAMILY="tmp0"
export TOADSTOOL_NODE_ID="node1"
EXPECTED_SOCKET="/tmp/toadstool-tmp0-node1.sock"

echo "Setting XDG_RUNTIME_DIR=/nonexistent (simulate missing XDG)"
echo "Setting TOADSTOOL_FAMILY=tmp0"
echo "Setting TOADSTOOL_NODE_ID=node1"
echo "Expected socket: $EXPECTED_SOCKET"
echo "Starting ToadStool..."

timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test3.log || true

sleep 1

if [ -S "$EXPECTED_SOCKET" ]; then
    echo "✅ Test 3 PASSED: Socket created in /tmp with family and node ID"
    ls -lh "$EXPECTED_SOCKET"
else
    echo "❌ Test 3 FAILED: Socket not found at: $EXPECTED_SOCKET"
    echo "Logs:"
    cat /tmp/toadstool-test3.log
    exit 1
fi

unset XDG_RUNTIME_DIR
unset TOADSTOOL_FAMILY
unset TOADSTOOL_NODE_ID
cleanup
echo ""

# Test 4: Socket Cleanup (Old Socket Removal)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 4: Socket Cleanup (Old Socket Removal)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TEST_SOCKET="/tmp/test-socket-cleanup.sock"

echo "Creating old socket file: $TEST_SOCKET"
touch "$TEST_SOCKET"
ls -lh "$TEST_SOCKET"

echo "Starting ToadStool (should remove old socket)..."
export TOADSTOOL_SOCKET="$TEST_SOCKET"

timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test4.log || true

sleep 1

if [ -S "$TEST_SOCKET" ]; then
    echo "✅ Test 4 PASSED: Old socket removed and new socket created"
    ls -lh "$TEST_SOCKET"
    
    # Check that it's actually a socket, not a regular file
    if [ -S "$TEST_SOCKET" ]; then
        echo "✅ Verified: New socket is a Unix socket (not regular file)"
    else
        echo "❌ ERROR: File exists but is not a socket"
        exit 1
    fi
else
    echo "❌ Test 4 FAILED: Socket not found after cleanup"
    echo "Logs:"
    cat /tmp/toadstool-test4.log
    exit 1
fi

unset TOADSTOOL_SOCKET
cleanup
echo ""

# Test 5: Multi-Instance with Different Node IDs
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 5: Multi-Instance with Different Node IDs"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

export XDG_RUNTIME_DIR="/nonexistent"
export TOADSTOOL_FAMILY="multi"

# Instance 1
export TOADSTOOL_NODE_ID="node1"
SOCKET1="/tmp/toadstool-multi-node1.sock"
echo "Starting instance 1 with NODE_ID=node1..."
timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test5a.log || true &
PID1=$!
sleep 1

# Instance 2
export TOADSTOOL_NODE_ID="node2"
SOCKET2="/tmp/toadstool-multi-node2.sock"
echo "Starting instance 2 with NODE_ID=node2..."
timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test5b.log || true &
PID2=$!
sleep 1

if [ -S "$SOCKET1" ] && [ -S "$SOCKET2" ]; then
    echo "✅ Test 5 PASSED: Multiple instances running with unique sockets"
    echo "   Instance 1: $SOCKET1"
    ls -lh "$SOCKET1"
    echo "   Instance 2: $SOCKET2"
    ls -lh "$SOCKET2"
else
    echo "❌ Test 5 FAILED: Multi-instance sockets not found"
    echo "Socket 1 ($SOCKET1): $([ -S "$SOCKET1" ] && echo 'EXISTS' || echo 'MISSING')"
    echo "Socket 2 ($SOCKET2): $([ -S "$SOCKET2" ] && echo 'EXISTS' || echo 'MISSING')"
    exit 1
fi

kill $PID1 $PID2 2>/dev/null || true
unset XDG_RUNTIME_DIR
unset TOADSTOOL_FAMILY
unset TOADSTOOL_NODE_ID
cleanup
echo ""

# Test 6: Parent Directory Creation
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Test 6: Parent Directory Creation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TEST_DIR="/tmp/toadstool-test-parent-dir"
rm -rf "$TEST_DIR"

export TOADSTOOL_SOCKET="$TEST_DIR/nested/deep/toadstool.sock"
echo "Setting TOADSTOOL_SOCKET=$TOADSTOOL_SOCKET"
echo "Parent directory does not exist yet: $TEST_DIR/nested/deep/"
echo "Starting ToadStool (should create parent directories)..."

timeout 3 "$TOADSTOOL_BIN" &> /tmp/toadstool-test6.log || true

sleep 1

if [ -S "$TOADSTOOL_SOCKET" ]; then
    echo "✅ Test 6 PASSED: Parent directories created and socket bound"
    ls -lh "$TOADSTOOL_SOCKET"
    echo "Directory structure:"
    ls -lhR "$TEST_DIR"
else
    echo "❌ Test 6 FAILED: Socket not created with nested directories"
    echo "Logs:"
    cat /tmp/toadstool-test6.log
    exit 1
fi

rm -rf "$TEST_DIR"
unset TOADSTOOL_SOCKET
cleanup
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " 🎉 ALL TESTS PASSED"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ Test 1: TOADSTOOL_SOCKET override"
echo "✅ Test 2: XDG runtime directory"
echo "✅ Test 3: /tmp fallback"
echo "✅ Test 4: Socket cleanup"
echo "✅ Test 5: Multi-instance with node IDs"
echo "✅ Test 6: Parent directory creation"
echo ""
echo "ToadStool is fully compliant with biomeOS socket standardization."
echo ""
echo "Different orders of the same architecture. 🍄🐸"
echo ""

