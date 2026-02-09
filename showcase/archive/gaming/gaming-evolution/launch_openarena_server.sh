#!/bin/bash
# launch_openarena_server.sh
# Launches a dedicated OpenArena LAN server

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║          🎮 OpenArena LAN Server Launcher 🎮                 ║"
echo "║                                                              ║"
echo "║     Host a server for friends to join on your LAN!          ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check if openarena is installed
if ! command -v openarena-server &> /dev/null && ! command -v openarena &> /dev/null; then
    echo "❌ OpenArena not installed!"
    echo ""
    echo "Install it first:"
    echo "  sudo apt install openarena"
    exit 1
fi

echo "✅ OpenArena is installed"
echo ""

# Server configuration
MAP=${1:-"dm17"}
PORT=${2:-27960}
MAXCLIENTS=${3:-16}
HOSTNAME=${4:-"ecoPrimals LAN Server"}

echo "📊 Server Configuration:"
echo "  Map: $MAP"
echo "  Port: $PORT"
echo "  Max Players: $MAXCLIENTS"
echo "  Hostname: $HOSTNAME"
echo ""

# Popular maps
echo "💡 Popular Maps:"
echo "  dm17  - The Longest Yard (space platforms)"
echo "  dm6   - The Campgrounds (small arena)"
echo "  dm1   - Gates of Hell (classic)"
echo "  dm4   - Arena Gate (tight combat)"
echo "  dm7   - Abandoned Base (medium size)"
echo ""

# Get IP address
IP_ADDRESS=$(hostname -I | awk '{print $1}')
echo "🌐 Server will be accessible at:"
echo "  IP: $IP_ADDRESS"
echo "  Port: $PORT"
echo ""

echo "To change settings, run:"
echo "  $0 <map> <port> <maxclients> <hostname>"
echo ""
echo "Example:"
echo "  $0 dm6 27960 8 \"My Awesome Server\""
echo ""

read -p "🚀 Launch server now? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "No problem! Run this script again when ready."
    exit 0
fi

echo ""
echo "🎮 Launching dedicated server..."
echo ""
echo "Server console commands:"
echo "  /map <mapname>    - Change map"
echo "  /status           - Show players"
echo "  /kick <name>      - Kick player"
echo "  /quit             - Stop server"
echo ""
echo "Players can join by:"
echo "  1. In-game: Multiplayer → Specify → $IP_ADDRESS:$PORT"
echo "  2. Console: /connect $IP_ADDRESS:$PORT"
echo ""
sleep 2

# Check if openarena-server exists (dedicated server binary)
if command -v openarena-server &> /dev/null; then
    # Use dedicated server
    openarena-server \
        +set dedicated 2 \
        +set net_port $PORT \
        +set sv_hostname "$HOSTNAME" \
        +set sv_maxclients $MAXCLIENTS \
        +set g_gametype 0 \
        +set bot_enable 1 \
        +set bot_minplayers 2 \
        +map $MAP
else
    # Use regular openarena in dedicated mode
    openarena \
        +set dedicated 2 \
        +set net_port $PORT \
        +set sv_hostname "$HOSTNAME" \
        +set sv_maxclients $MAXCLIENTS \
        +set g_gametype 0 \
        +set bot_enable 1 \
        +map $MAP
fi

echo ""
echo "Server stopped."

