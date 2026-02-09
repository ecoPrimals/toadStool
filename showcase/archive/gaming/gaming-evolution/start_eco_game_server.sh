#!/bin/bash
# start_eco_game_server.sh
# Launches OpenArena server and registers it with Songbird for auto-discovery
# NO MANUAL IP SHARING NEEDED!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🍄 ecoPrimals Gaming Server 🍄                         ║"
echo "║                                                              ║"
echo "║     Self-discovering, zero-config, eco-native!               ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Configuration
MAP=${1:-"dm17"}
PORT=${2:-27960}
MAXCLIENTS=${3:-16}
GAME_TYPE=${4:-"openarena"}

# Get local info
HOSTNAME=$(hostname)
IP_ADDRESS=$(hostname -I | awk '{print $1}')
SERVER_ID="$HOSTNAME-$GAME_TYPE-$(date +%s)"

echo "🗼 Tower: $HOSTNAME"
echo "🎮 Game: $GAME_TYPE"
echo "🗺️  Map: $MAP"
echo "🔌 Port: $PORT"
echo ""

# Check if Songbird is running
echo "🔍 Checking for Songbird..."
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "⚠️  Songbird not running - using fallback mode"
    echo ""
    echo "For full eco-native experience, start Songbird:"
    echo "  cd ../../../songbird"
    echo "  cargo run --release --bin songbird-orchestrator"
    echo ""
    echo "Continuing with direct discovery (mDNS)..."
    SONGBIRD_AVAILABLE=false
else
    echo "✅ Songbird is running!"
    SONGBIRD_AVAILABLE=true
fi
echo ""

# Create server config
mkdir -p ~/.openarena/baseoa
cat > ~/.openarena/baseoa/server.cfg << EOF
// ecoPrimals OpenArena Server Config
seta sv_hostname "🍄 $HOSTNAME - ecoPrimals Gaming"
seta sv_maxclients $MAXCLIENTS
seta g_gametype 0
seta timelimit 20
seta fraglimit 25
seta g_motd "Welcome to ecoPrimals! Discovered via Songbird!"

// Bot settings
seta bot_enable 1
seta bot_minplayers 2
seta g_spSkill 2

// Network
seta sv_lanForceRate 1
seta sv_maxRate 25000

// Discovery
seta sv_master1 ""
seta sv_master2 ""
seta sv_master3 ""
seta sv_master4 ""
seta sv_master5 ""
EOF

# Register with Songbird (if available)
if [ "$SONGBIRD_AVAILABLE" = true ]; then
    echo "📡 Registering game server with Songbird..."
    
    REGISTER_RESULT=$(curl -s -X POST http://localhost:8080/api/services/register \
      -H "Content-Type: application/json" \
      -d "{
        \"service_id\": \"$SERVER_ID\",
        \"service_type\": \"game-server\",
        \"hostname\": \"$HOSTNAME\",
        \"address\": \"$IP_ADDRESS:$PORT\",
        \"capabilities\": [
          \"game-server\",
          \"openarena\",
          \"multiplayer\",
          \"join-leave\"
        ],
        \"metadata\": {
          \"game\": \"$GAME_TYPE\",
          \"map\": \"$MAP\",
          \"max_players\": $MAXCLIENTS,
          \"protocol\": \"quake3\",
          \"eco_native\": true
        }
      }" 2>/dev/null || echo '{"status":"fallback"}')
    
    if echo "$REGISTER_RESULT" | grep -q "success\|registered"; then
        echo "✅ Registered with Songbird!"
        echo "   Service ID: $SERVER_ID"
        echo ""
        echo "🌐 Other towers can now discover this server via:"
        echo "   ./discover_eco_game_servers.sh"
        echo "   ./join_eco_game.sh openarena"
        echo ""
    else
        echo "⚠️  Registration failed - using fallback mDNS"
        SONGBIRD_AVAILABLE=false
    fi
fi

# mDNS fallback registration
if [ "$SONGBIRD_AVAILABLE" = false ]; then
    echo "📡 Using mDNS for local discovery..."
    echo "   (Players can find via in-game LAN browser)"
    echo ""
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  🎊 Server is DISCOVERABLE! 🎊"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "NO MANUAL IP SHARING NEEDED!"
echo ""
echo "Players on other towers just run:"
echo "  ./discover_eco_game_servers.sh"
echo "  ./join_eco_game.sh openarena"
echo ""
echo "Starting game server..."
echo "  (Press Ctrl+C to stop and unregister)"
echo ""
sleep 2

# Cleanup function to unregister on exit
cleanup() {
    echo ""
    echo "🛑 Stopping server..."
    
    if [ "$SONGBIRD_AVAILABLE" = true ]; then
        echo "📡 Unregistering from Songbird..."
        curl -s -X DELETE "http://localhost:8080/api/services/unregister/$SERVER_ID" >/dev/null 2>&1
        echo "✅ Unregistered"
    fi
    
    echo "Server stopped."
    exit 0
}

trap cleanup SIGINT SIGTERM

# Launch server
if command -v openarena-server &> /dev/null; then
    openarena-server \
        +set dedicated 1 \
        +set net_port $PORT \
        +exec server.cfg \
        +map $MAP
else
    openarena \
        +set dedicated 1 \
        +set net_port $PORT \
        +exec server.cfg \
        +map $MAP
fi

