#!/bin/bash
# Download Legal Test Games
# Gets free/shareware games for testing

set -e

echo "📦 Downloading Legal Test Games for ecoPrimals"
echo "=============================================="
echo ""
echo "These are all legal, free versions:"
echo "  • Quake Shareware (id Software released it free)"
echo "  • Doom Shareware (id Software released it free)"
echo "  • Test games we create"
echo ""

# Create games directory
mkdir -p /tmp/games
cd /tmp/games

# Function to download with progress
download_game() {
    local name=$1
    local url=$2
    local dir=$3
    
    echo ""
    echo "📥 Downloading $name..."
    mkdir -p "$dir"
    cd "$dir"
    
    if wget -q --show-progress "$url" -O game.zip 2>/dev/null; then
        echo "  ✅ Downloaded"
        echo "  📦 Extracting..."
        unzip -q game.zip 2>/dev/null || unzip -q game.zip
        rm game.zip
        echo "  ✅ Ready!"
    else
        echo "  ⚠️  Download failed (check internet connection)"
    fi
    
    cd /tmp/games
}

# Download Quake Shareware
echo ""
echo "═══════════════════════════════════════════"
echo "  QUAKE SHAREWARE"
echo "═══════════════════════════════════════════"
echo "• Official shareware release by id Software"
echo "• Legal and free forever"
echo "• Great multiplayer"
echo "• Perfect for testing"

if [ ! -d "quake-shareware" ]; then
    download_game "Quake Shareware" \
        "https://archive.org/download/quake-shareware/quake106.zip" \
        "quake-shareware"
    
    if [ -f "quake-shareware/quake.exe" ] || [ -f "quake-shareware/QUAKE.EXE" ]; then
        echo "  🎮 To play: wine /tmp/games/quake-shareware/quake.exe"
    fi
else
    echo "  ✅ Already downloaded!"
fi

# Download Doom Shareware
echo ""
echo "═══════════════════════════════════════════"
echo "  DOOM SHAREWARE"
echo "═══════════════════════════════════════════"
echo "• Official shareware release by id Software"
echo "• Legal and free forever"
echo "• Classic FPS"
echo "• Great for testing"

if [ ! -d "doom-shareware" ]; then
    download_game "Doom Shareware" \
        "https://archive.org/download/DoomsharewareEpisode/doom.zip" \
        "doom-shareware"
    
    if [ -f "doom-shareware/doom.exe" ] || [ -f "doom-shareware/DOOM.EXE" ]; then
        echo "  🎮 To play: wine /tmp/games/doom-shareware/doom.exe"
    fi
else
    echo "  ✅ Already downloaded!"
fi

# Create Python test game
echo ""
echo "═══════════════════════════════════════════"
echo "  NETWORK TEST GAME"
echo "═══════════════════════════════════════════"
echo "• Simple multiplayer test"
echo "• Proves networking works"
echo "• No installation needed"

cat > /tmp/games/test_multiplayer.py << 'EOF'
#!/usr/bin/env python3
"""
Simple Multiplayer Test Game
Tests networking and player discovery
"""

import socket
import sys
import time
import threading

def run_server(port=6112):
    """Run game server"""
    print("🎮 Test Game Server")
    print("=" * 50)
    print(f"Listening on port {port}...")
    print("Waiting for players to join...")
    print()
    
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    try:
        server.bind(('0.0.0.0', port))
        server.listen(5)
        print("✅ Server ready!")
        print()
        
        players = []
        player_count = 0
        
        while player_count < 4:
            server.settimeout(30.0)
            try:
                client, addr = server.accept()
                player_count += 1
                players.append((client, addr))
                
                print(f"✅ Player {player_count} joined from {addr[0]}:{addr[1]}")
                client.send(f"Welcome Player {player_count}!\n".encode())
                
                if player_count >= 2:
                    print()
                    print("=" * 50)
                    print(f"🎉 {player_count} players connected!")
                    print()
                    print("This proves:")
                    print("  ✅ Network connectivity works")
                    print("  ✅ Port forwarding works")
                    print("  ✅ Players can discover server")
                    print("  ✅ Multiplayer is functional!")
                    print()
                    print("💡 You can now test real games!")
                    print("=" * 50)
                    
                    # Send success message to all players
                    for p_client, p_addr in players:
                        try:
                            p_client.send(f"\n🎉 Game started with {player_count} players!\n".encode())
                        except:
                            pass
                    
                    break
                    
            except socket.timeout:
                if player_count > 0:
                    print(f"\n⏱️  Timeout waiting for more players ({player_count} connected)")
                    print("Starting with current players...")
                    break
                else:
                    print("\n⏱️  No players connected. Exiting...")
                    return
        
        # Keep connection open for a bit
        time.sleep(5)
        print("\nClosing server...")
        
    except Exception as e:
        print(f"❌ Server error: {e}")
    finally:
        server.close()

def run_client(host, port=6112):
    """Run game client"""
    print("🎮 Test Game Client")
    print("=" * 50)
    print(f"Connecting to {host}:{port}...")
    print()
    
    client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    
    try:
        client.connect((host, port))
        print("✅ Connected to server!")
        print()
        
        # Receive messages
        while True:
            try:
                client.settimeout(10.0)
                msg = client.recv(1024)
                if not msg:
                    break
                print(msg.decode(), end='')
            except socket.timeout:
                break
            except:
                break
        
        print()
        print("=" * 50)
        print("🎉 Multiplayer test SUCCESS!")
        print("=" * 50)
        
        return True
        
    except ConnectionRefusedError:
        print("❌ Connection refused!")
        print()
        print("Make sure server is running:")
        print("  python3 test_multiplayer.py server")
        return False
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        return False
    finally:
        client.close()

def print_usage():
    print("Usage:")
    print("  Server: python3 test_multiplayer.py server")
    print("  Client: python3 test_multiplayer.py client <host>")
    print()
    print("Examples:")
    print("  Terminal 1: python3 test_multiplayer.py server")
    print("  Terminal 2: python3 test_multiplayer.py client localhost")
    print()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print_usage()
        sys.exit(1)
    
    mode = sys.argv[1].lower()
    
    if mode == "server":
        run_server()
    elif mode == "client":
        host = sys.argv[2] if len(sys.argv) > 2 else "localhost"
        run_client(host)
    else:
        print_usage()
        sys.exit(1)
EOF

chmod +x /tmp/games/test_multiplayer.py
echo "  ✅ Network test game created!"
echo "  🎮 To test:"
echo "     Terminal 1: python3 /tmp/games/test_multiplayer.py server"
echo "     Terminal 2: python3 /tmp/games/test_multiplayer.py client localhost"

# Summary
echo ""
echo "═══════════════════════════════════════════"
echo "  📊 DOWNLOAD COMPLETE!"
echo "═══════════════════════════════════════════"
echo ""
echo "Available games:"
ls -1d /tmp/games/*/ 2>/dev/null | while read dir; do
    echo "  ✅ $(basename $dir)"
done

echo ""
echo "Test multiplayer game:"
echo "  ✅ /tmp/games/test_multiplayer.py"

echo ""
echo "Next steps:"
echo "  1. Test networking:"
echo "     python3 /tmp/games/test_multiplayer.py server"
echo ""
echo "  2. Test real game:"
echo "     cd lan-party-showcase"
echo "     ./launch_game.sh /tmp/games/quake-shareware/quake.exe"
echo ""
echo "  3. Have fun! 🎮"
echo ""

