#!/bin/bash
# join_lan_server.sh
# Connect to a LAN OpenArena server

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║         🎮 Join OpenArena LAN Server 🎮                      ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

SERVER_IP=${1:-""}

if [ -z "$SERVER_IP" ]; then
    echo "Usage: $0 <server-ip>"
    echo ""
    echo "Examples:"
    echo "  $0 192.168.1.100"
    echo "  $0 localhost"
    echo ""
    
    # Try to detect local IP for hint
    LOCAL_IP=$(hostname -I | awk '{print $1}')
    echo "💡 Your IP is: $LOCAL_IP"
    echo "   (Others use this to connect to YOUR server)"
    echo ""
    
    echo "Or launch OpenArena and use the in-game menu:"
    echo "  Multiplayer → Specify Server → Enter IP"
    echo ""
    
    read -p "Enter server IP to connect (or press Enter to scan): " SERVER_IP
    
    if [ -z "$SERVER_IP" ]; then
        echo ""
        echo "🔍 Launching OpenArena to scan for servers..."
        echo ""
        echo "In OpenArena:"
        echo "  1. Click 'Multiplayer'"
        echo "  2. Click 'Local Servers'"
        echo "  3. Wait for servers to appear"
        echo "  4. Double-click to join!"
        echo ""
        sleep 2
        openarena
        exit 0
    fi
fi

echo "🎮 Connecting to server: $SERVER_IP"
echo ""
echo "Launching OpenArena..."
echo ""

openarena +connect $SERVER_IP

echo ""
echo "Game ended."

