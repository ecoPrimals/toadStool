#!/bin/bash
# fix_server_config.sh
# Fix common server configuration issues causing "awaiting challenge"

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║
║         🔧 Fix Server Configuration Issues 🔧                 ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

echo "This script fixes the 'awaiting challenge' error by:"
echo "  1. Creating proper server configuration"
echo "  2. Setting correct network settings"
echo "  3. Ensuring server is discoverable"
echo ""

# Create config directory
mkdir -p ~/.openarena/baseoa

# Create WORKING server config
echo "📝 Creating optimized server.cfg..."

cat > ~/.openarena/baseoa/server.cfg << 'EOF'
// OpenArena Server Config - Optimized for LAN
// Fixes "awaiting challenge" issues

// Server Identity
seta sv_hostname "🍄 ecoPrimals LAN Server"
seta sv_maxclients 16

// Network Settings - CRITICAL for connectivity
seta dedicated 1                    // LAN mode with heartbeat
seta net_port 27960
seta sv_maxRate 0                   // Unlimited (LAN)
seta sv_lanForceRate 1              // Force good rates on LAN
seta sv_fps 20                      // Server FPS

// Game Settings
seta g_gametype 0                   // Free For All
seta fraglimit 25
seta timelimit 20
seta g_motd "Welcome to ecoPrimals Gaming!"

// Bot Settings
seta bot_enable 1
seta bot_minplayers 2               // Keep 2 bots minimum
seta g_spSkill 2                    // Medium difficulty

// Challenge/Authentication Settings - KEY FOR FIXING ERROR
seta sv_strictAuth 0                // Don't require strict auth
seta sv_pure 0                      // Allow modified clients
seta g_needpass 0                   // No password required

// Gameplay
seta g_allowVote 1
seta g_inactivity 300

// Logging
seta g_log "games.log"
seta g_logSync 1

// Master Server - DISABLE for pure LAN
seta sv_master1 ""
seta sv_master2 ""
seta sv_master3 ""
seta sv_master4 ""
seta sv_master5 ""

// Additional fixes
seta sv_floodProtect 0              // No flood protection on LAN
seta sv_timeout 200                 // Longer timeout
seta cl_timeout 200                 // Client timeout
EOF

echo "✅ server.cfg created"
echo ""

# Create autoexec for server
echo "📝 Creating server autoexec.cfg..."

cat > ~/.openarena/baseoa/autoexec_server.cfg << 'EOF'
// Server Autoexec
// Runs automatically when server starts

echo "================================"
echo "  ecoPrimals Gaming Server"
echo "  Ready for connections!"
echo "================================"

// Ensure bots are enabled
set bot_enable 1
set bot_minplayers 2

// Status display
set sv_timeout 200
set cl_timeout 200

echo "Server configured for LAN play"
echo "Awaiting players..."
EOF

echo "✅ autoexec_server.cfg created"
echo ""

# Create client config to fix connection issues
echo "📝 Creating client connection config..."

cat > ~/.openarena/baseoa/autoexec_client.cfg << 'EOF'
// Client Autoexec for better connectivity

// Network settings
set cl_maxpackets 125
set cl_packetdup 1
set rate 25000
set snaps 40

// Connection timeouts
set cl_timeout 200

// Reduce strict checking
set cl_allowDownload 1

echo "Client configured for LAN connections"
EOF

echo "✅ autoexec_client.cfg created"
echo ""

# Show what was configured
echo "═══════════════════════════════════════════════════════════════"
echo "KEY SETTINGS APPLIED:"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  dedicated: 1 (LAN mode with heartbeat)"
echo "  sv_strictAuth: 0 (no strict authentication)"
echo "  sv_pure: 0 (allow modified clients)"
echo "  sv_maxRate: 0 (unlimited for LAN)"
echo "  bot_enable: 1 (bots active)"
echo "  Master servers: DISABLED (pure LAN)"
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "WHAT THIS FIXES:"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  ✅ 'Awaiting challenge' errors"
echo "  ✅ Connection timeouts"
echo "  ✅ Authentication failures"
echo "  ✅ LAN discovery issues"
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "NEXT STEPS:"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "1. Stop any running servers:"
echo "   ./stop_local_server.sh"
echo ""
echo "2. Test the fixed configuration:"
echo "   ./play_local.sh"
echo ""
echo "3. If still having issues, run diagnostics:"
echo "   ./test_server_connectivity.sh"
echo ""

echo "Configuration fixed! ✅"

