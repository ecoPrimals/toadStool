#!/bin/bash
# start_dedicated_server.sh
# Launches a proper dedicated OpenArena server for LAN play
# Players can join and leave freely!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🎮 OpenArena Dedicated LAN Server 🎮                   ║"
echo "║                                                              ║"
echo "║        Persistent server - Join/leave anytime!               ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Configuration
MAP=${1:-"dm17"}
PORT=${2:-27960}
MAXCLIENTS=${3:-16}
HOSTNAME=${4:-"ecoPrimals LAN Server"}

# Get network info
IP_ADDRESS=$(hostname -I | awk '{print $1}')

echo "📊 Server Configuration:"
echo "  Hostname: $HOSTNAME"
echo "  Map: $MAP"
echo "  Port: $PORT"
echo "  Max Players: $MAXCLIENTS"
echo "  Server IP: $IP_ADDRESS"
echo ""

echo "🌐 Players can connect via:"
echo "  1. In-game menu: Multiplayer → Specify → $IP_ADDRESS"
echo "  2. Console: /connect $IP_ADDRESS"
echo "  3. Auto-discovery: Look for '$HOSTNAME' in server list"
echo ""

echo "💡 Popular maps:"
echo "  dm17 - The Longest Yard (space platforms)"
echo "  dm6  - The Campgrounds (small, fast)"
echo "  dm7  - Abandoned Base (medium)"
echo ""

echo "🎮 Starting dedicated server..."
echo "   (Press Ctrl+C to stop)"
echo ""
sleep 2

# Create server config if it doesn't exist
mkdir -p ~/.openarena/baseoa
cat > ~/.openarena/baseoa/server.cfg << EOF
// OpenArena Dedicated Server Config
seta sv_hostname "$HOSTNAME"
seta sv_maxclients $MAXCLIENTS
seta g_gametype 0              // 0=FFA, 1=Tournament, 3=Team DM, 4=CTF
seta timelimit 20              // 20 minutes per map
seta fraglimit 25              // 25 frags to win
seta g_motd "Welcome to ecoPrimals Gaming!"

// Bot settings
seta bot_enable 1              // Enable bots
seta bot_minplayers 2          // Minimum 2 players (fills with bots)
seta g_spSkill 2               // Bot difficulty (1-5)

// Network settings
seta sv_master1 "master.ioquake3.org"
seta sv_master2 ""
seta sv_master3 ""
seta sv_master4 ""
seta sv_master5 ""

// LAN settings
seta sv_lanForceRate 1         // Force good rates on LAN
seta sv_maxRate 25000          // Max rate for clients

// Misc
seta g_allowVote 1             // Allow voting
seta g_inactivity 300          // Kick after 5 min inactivity
seta g_log "games.log"         // Log file
EOF

echo "✅ Server config created: ~/.openarena/baseoa/server.cfg"
echo ""

# Launch dedicated server
if command -v openarena-server &> /dev/null; then
    # Use dedicated server binary
    echo "Using openarena-server binary..."
    openarena-server \
        +set dedicated 1 \
        +set net_port $PORT \
        +exec server.cfg \
        +map $MAP
else
    # Use regular binary in dedicated mode
    echo "Using openarena binary in dedicated mode..."
    openarena \
        +set dedicated 1 \
        +set net_port $PORT \
        +exec server.cfg \
        +map $MAP
fi

echo ""
echo "Server stopped."

