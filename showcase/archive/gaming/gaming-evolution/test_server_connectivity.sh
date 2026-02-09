#!/bin/bash
# test_server_connectivity.sh
# Comprehensive testing and diagnostics for game server

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🔍 Server Connectivity Testing & Diagnostics 🔍        ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

PORT=${1:-27960}
SERVER_IP=${2:-"127.0.0.1"}

echo "Testing server: $SERVER_IP:$PORT"
echo ""

# Test 1: Check if port is listening
echo "═══════════════════════════════════════════════════════════════"
echo "Test 1: Port Listening Check"
echo "───────────────────────────────────────────────────────────────"

if command -v netstat &> /dev/null; then
    LISTENING=$(netstat -an | grep ":$PORT" | grep -i listen || echo "")
    if [ -n "$LISTENING" ]; then
        echo "✅ Port $PORT is LISTENING"
        echo "$LISTENING"
    else
        UDPBIND=$(netstat -anu | grep ":$PORT" || echo "")
        if [ -n "$UDPBIND" ]; then
            echo "✅ Port $PORT has UDP binding"
            echo "$UDPBIND"
        else
            echo "❌ Port $PORT is NOT listening"
            echo "   Server may not be running or using different port"
        fi
    fi
else
    echo "⚠️  netstat not available, trying ss..."
    if command -v ss &> /dev/null; then
        LISTENING=$(ss -tuln | grep ":$PORT" || echo "")
        if [ -n "$LISTENING" ]; then
            echo "✅ Port $PORT is active"
            echo "$LISTENING"
        else
            echo "❌ Port $PORT not found"
        fi
    else
        echo "⚠️  Neither netstat nor ss available"
    fi
fi
echo ""

# Test 2: Check for OpenArena processes
echo "═══════════════════════════════════════════════════════════════"
echo "Test 2: OpenArena Process Check"
echo "───────────────────────────────────────────────────────────────"

PROCESSES=$(pgrep -a openarena || echo "")
if [ -n "$PROCESSES" ]; then
    echo "✅ OpenArena processes found:"
    echo "$PROCESSES"
else
    echo "❌ No OpenArena processes running"
    echo "   Server needs to be started"
fi
echo ""

# Test 3: Check server log (if exists)
echo "═══════════════════════════════════════════════════════════════"
echo "Test 3: Server Log Check"
echo "───────────────────────────────────────────────────────────────"

if [ -f /tmp/openarena-server.log ]; then
    echo "✅ Server log found"
    echo ""
    echo "Last 20 lines:"
    tail -20 /tmp/openarena-server.log
else
    echo "⚠️  No server log at /tmp/openarena-server.log"
    echo "   Server may not have been started with logging"
fi
echo ""

# Test 4: UDP connectivity test
echo "═══════════════════════════════════════════════════════════════"
echo "Test 4: UDP Connectivity Test"
echo "───────────────────────────────────────────────────────────────"

if command -v nc &> /dev/null; then
    echo "Testing UDP connection to $SERVER_IP:$PORT..."
    
    # Send a test packet
    timeout 2 bash -c "echo 'test' | nc -u -w1 $SERVER_IP $PORT" 2>/dev/null && \
        echo "✅ UDP connection successful" || \
        echo "⚠️  UDP connection test inconclusive (this is normal for game servers)"
else
    echo "⚠️  netcat (nc) not available for UDP test"
fi
echo ""

# Test 5: Check firewall
echo "═══════════════════════════════════════════════════════════════"
echo "Test 5: Firewall Check"
echo "───────────────────────────────────────────────────────────────"

if command -v ufw &> /dev/null; then
    UFW_STATUS=$(sudo ufw status 2>/dev/null || echo "inactive")
    echo "UFW Status:"
    echo "$UFW_STATUS"
    
    if echo "$UFW_STATUS" | grep -q "$PORT"; then
        echo "✅ Port $PORT is in firewall rules"
    else
        echo "⚠️  Port $PORT not explicitly in firewall rules"
        echo "   This may block remote connections"
        echo ""
        echo "   To allow:"
        echo "   sudo ufw allow $PORT/udp"
        echo "   sudo ufw allow $PORT/tcp"
    fi
else
    echo "⚠️  UFW not available, checking iptables..."
    if command -v iptables &> /dev/null; then
        IPTABLES=$(sudo iptables -L -n | grep $PORT || echo "")
        if [ -n "$IPTABLES" ]; then
            echo "✅ Port $PORT found in iptables"
            echo "$IPTABLES"
        else
            echo "⚠️  Port $PORT not found in iptables"
        fi
    else
        echo "⚠️  Cannot check firewall (no ufw or iptables access)"
    fi
fi
echo ""

# Test 6: Server config check
echo "═══════════════════════════════════════════════════════════════"
echo "Test 6: Server Configuration Check"
echo "───────────────────────────────────────────────────────────────"

if [ -f ~/.openarena/baseoa/server.cfg ]; then
    echo "✅ Server config found"
    echo ""
    echo "Key settings:"
    grep -E "sv_hostname|sv_maxclients|dedicated|bot_enable" ~/.openarena/baseoa/server.cfg || echo "  (No key settings found)"
else
    echo "⚠️  No server config at ~/.openarena/baseoa/server.cfg"
fi
echo ""

# Test 7: Network info
echo "═══════════════════════════════════════════════════════════════"
echo "Test 7: Network Information"
echo "───────────────────────────────────────────────────────────────"

echo "Hostname: $(hostname)"
echo "IP Addresses:"
hostname -I || echo "  (Cannot determine IP)"
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
echo "DIAGNOSTIC SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check if server seems to be running
if [ -n "$PROCESSES" ] && [ -n "$LISTENING" ]; then
    echo "✅ Server appears to be running correctly"
    echo ""
    echo "If client still can't connect, check:"
    echo "  1. Server finished loading (wait 10 seconds after start)"
    echo "  2. Client using correct IP and port"
    echo "  3. No firewall blocking on either end"
else
    echo "❌ Server may not be configured correctly"
    echo ""
    echo "Issues found:"
    [ -z "$PROCESSES" ] && echo "  • No OpenArena process running"
    [ -z "$LISTENING" ] && echo "  • Port $PORT not listening"
    echo ""
    echo "Try:"
    echo "  1. Stop any existing servers: ./stop_local_server.sh"
    echo "  2. Check server log: cat /tmp/openarena-server.log"
    echo "  3. Restart server: ./play_local.sh"
fi
echo ""

echo "For detailed server debugging:"
echo "  tail -f /tmp/openarena-server.log"

