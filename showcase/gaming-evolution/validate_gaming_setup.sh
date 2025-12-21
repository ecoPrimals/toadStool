#!/bin/bash
# validate_gaming_setup.sh
# Comprehensive validation of entire gaming setup
# Identifies all gaps and issues

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       ✅ Gaming Setup Validation Suite ✅                     ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

pass_test() {
    echo "✅ PASS: $1"
    ((PASS_COUNT++))
}

fail_test() {
    echo "❌ FAIL: $1"
    echo "   Fix: $2"
    ((FAIL_COUNT++))
}

warn_test() {
    echo "⚠️  WARN: $1"
    echo "   Info: $2"
    ((WARN_COUNT++))
}

echo "Starting comprehensive validation..."
echo ""

# Category 1: Prerequisites
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 1: Prerequisites"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Test: OpenArena installed
if command -v openarena &> /dev/null; then
    pass_test "OpenArena is installed"
    echo "   Path: $(which openarena)"
else
    fail_test "OpenArena not installed" "sudo apt install openarena"
fi

# Test: jq installed (for JSON parsing)
if command -v jq &> /dev/null; then
    pass_test "jq is installed (for JSON parsing)"
else
    warn_test "jq not installed" "sudo apt install jq (needed for eco-native discovery)"
fi

# Test: curl installed
if command -v curl &> /dev/null; then
    pass_test "curl is installed"
else
    fail_test "curl not installed" "sudo apt install curl"
fi

# Test: netcat available
if command -v nc &> /dev/null; then
    pass_test "netcat is installed (for network testing)"
else
    warn_test "netcat not installed" "sudo apt install netcat (useful for debugging)"
fi

echo ""

# Category 2: Scripts
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 2: Scripts & Files"
echo "═══════════════════════════════════════════════════════════════"
echo ""

SCRIPTS=(
    "play_local.sh"
    "start_eco_game_server.sh"
    "discover_eco_game_servers.sh"
    "join_eco_game.sh"
    "test_server_connectivity.sh"
    "stop_local_server.sh"
)

for script in "${SCRIPTS[@]}"; do
    if [ -f "$script" ]; then
        if [ -x "$script" ]; then
            pass_test "Script $script exists and is executable"
        else
            fail_test "Script $script not executable" "chmod +x $script"
        fi
    else
        fail_test "Script $script missing" "Check git status, may need to pull"
    fi
done

echo ""

# Category 3: Network Configuration
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 3: Network Configuration"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Test: Can resolve hostname
if hostname -I &> /dev/null; then
    IP=$(hostname -I | awk '{print $1}')
    pass_test "Network configured, IP: $IP"
else
    fail_test "Cannot determine IP address" "Check network configuration"
fi

# Test: Port 27960 available
PORT_CHECK=$(netstat -an 2>/dev/null | grep ":27960" || echo "")
if [ -z "$PORT_CHECK" ]; then
    pass_test "Port 27960 is available"
else
    warn_test "Port 27960 already in use" "May have existing server running"
    echo "   $PORT_CHECK"
fi

echo ""

# Category 4: Firewall
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 4: Firewall Configuration"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if command -v ufw &> /dev/null; then
    UFW_STATUS=$(sudo ufw status 2>/dev/null || echo "inactive")
    
    if echo "$UFW_STATUS" | grep -q "inactive"; then
        pass_test "UFW firewall is inactive (no blocking)"
    else
        if echo "$UFW_STATUS" | grep -q "27960"; then
            pass_test "UFW allows port 27960"
        else
            warn_test "UFW active but port 27960 not explicitly allowed" \
                "May block remote connections. Run: sudo ufw allow 27960"
        fi
    fi
else
    warn_test "UFW not available" "Cannot check firewall rules automatically"
fi

echo ""

# Category 5: Songbird Integration (optional)
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 5: Songbird Integration (Optional)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if curl -s http://localhost:8080/health &> /dev/null; then
    pass_test "Songbird is running and accessible"
    echo "   Eco-native discovery will work!"
else
    warn_test "Songbird not running" \
        "Eco-native discovery unavailable, will use fallback. Start with: cd ../../../songbird && cargo run --release --bin songbird-orchestrator"
fi

echo ""

# Category 6: Server Configuration Files
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 6: Configuration Files"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check if .openarena directory exists
if [ -d ~/.openarena ]; then
    pass_test "OpenArena config directory exists"
else
    warn_test "OpenArena config directory missing" \
        "Will be created on first run"
fi

# Check for existing server config
if [ -f ~/.openarena/baseoa/server.cfg ]; then
    pass_test "Server config file exists"
    
    # Validate key settings
    if grep -q "sv_hostname" ~/.openarena/baseoa/server.cfg; then
        pass_test "Server config has hostname set"
    else
        warn_test "Server config missing hostname" "May use default"
    fi
else
    warn_test "No server.cfg found" "Will be created by server scripts"
fi

echo ""

# Category 7: System Resources
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 7: System Resources"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check available memory
AVAILABLE_MEM=$(free -m | awk '/^Mem:/{print $7}')
if [ "$AVAILABLE_MEM" -gt 500 ]; then
    pass_test "Sufficient memory available (${AVAILABLE_MEM}MB free)"
else
    warn_test "Low memory (${AVAILABLE_MEM}MB free)" \
        "May impact performance with many bots"
fi

# Check disk space
AVAILABLE_DISK=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
if [ "$AVAILABLE_DISK" -gt 1 ]; then
    pass_test "Sufficient disk space (${AVAILABLE_DISK}GB free)"
else
    warn_test "Low disk space (${AVAILABLE_DISK}GB free)" \
        "Should have at least 2GB for game files"
fi

echo ""

# Category 8: Running Processes
echo "═══════════════════════════════════════════════════════════════"
echo "CATEGORY 8: Current State"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check for running OpenArena
RUNNING=$(pgrep -a openarena || echo "")
if [ -n "$RUNNING" ]; then
    warn_test "OpenArena already running" \
        "May interfere with new servers. PIDs: $(pgrep openarena | tr '\n' ' ')"
    echo "   $RUNNING"
else
    pass_test "No OpenArena processes running (clean state)"
fi

# Check for server PID file
if [ -f /tmp/openarena-server.pid ]; then
    PID=$(cat /tmp/openarena-server.pid)
    if kill -0 $PID 2>/dev/null; then
        warn_test "Server PID file exists with running server" \
            "Server may already be running (PID: $PID)"
    else
        warn_test "Stale server PID file" \
            "Old PID file exists. Run: rm /tmp/openarena-server.pid"
    fi
else
    pass_test "No stale PID files"
fi

echo ""

# Final Summary
echo "═══════════════════════════════════════════════════════════════"
echo "VALIDATION SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Tests Passed:  $PASS_COUNT ✅"
echo "Tests Failed:  $FAIL_COUNT ❌"
echo "Warnings:      $WARN_COUNT ⚠️"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo "🎉 ALL CRITICAL TESTS PASSED!"
    echo ""
    echo "Your system is ready for gaming!"
    echo ""
    
    if [ $WARN_COUNT -eq 0 ]; then
        echo "✨ PERFECT SETUP - No warnings!"
        echo ""
        echo "Ready to play:"
        echo "  ./play_local.sh"
    else
        echo "⚠️  Some optional components missing."
        echo "   Gaming will work, but check warnings above for full features."
        echo ""
        echo "Ready to test:"
        echo "  ./play_local.sh"
    fi
else
    echo "❌ SETUP INCOMPLETE"
    echo ""
    echo "Fix the failed tests above before proceeding."
    echo ""
    echo "Common fixes:"
    echo "  1. Install OpenArena: sudo apt install openarena"
    echo "  2. Install tools: sudo apt install jq curl"
    echo "  3. Make scripts executable: chmod +x *.sh"
fi
echo ""

echo "For detailed server diagnostics:"
echo "  ./test_server_connectivity.sh"
echo ""

exit $FAIL_COUNT

