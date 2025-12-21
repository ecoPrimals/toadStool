# 🗼 Tower Federation Showcase - Access Remote Steam Library

**The Cool One!** Access your gaming tower's Steam library from anywhere! 🚀

---

## 🎯 What This Does

**Connect to your gaming tower and access its Steam library remotely!**

### The Setup

```
Your Laptop/Desktop          Your Gaming Tower
  (Client)                      (Server)
     ↓                             ↓
  Songbird ←──── Network ────→ Songbird
     ↓                             ↓
  Discovery                    Steam Library
  Browse games                 (150+ games!)
  Launch remotely    →→→       Game executes
  Stream gameplay    ←←←        Sends video/input
```

---

## 🚀 Quick Start

### On Your Tower (One-Time Setup)

```bash
# 1. Start Songbird
cd /home/eastgate/Development/ecoPrimals/songbird
cargo run --release --bin songbird-orchestrator

# 2. Enable Steam library sharing
curl -X POST http://localhost:8080/api/federation/advertise \
  -d '{
    "capabilities": [
      "steam-library",
      "game-execution",
      "gpu-compute"
    ],
    "steam_library_path": "/home/USER/.steam/steam/steamapps",
    "available_games": 150
  }'

# 3. Tower is now discoverable!
```

### On Your Laptop

```bash
# 1. Discover your tower
./discover_tower.sh

# 2. Connect to it
./connect_to_tower.sh gaming-tower-main

# 3. Browse games
./browse_remote_library.sh

# 4. Launch a game!
./launch_remote_game.sh 730  # Counter-Strike
```

---

## 📜 Scripts

### 1. Discover Tower

```bash
#!/bin/bash
# discover_tower.sh - Find your gaming tower on the network

echo "🔍 Discovering towers on network..."
echo ""

# Use Songbird's federation discovery
RESULT=$(curl -s -X POST http://localhost:8080/api/federation/discover \
  -H "Content-Type: application/json" \
  -d '{
    "capability": "steam-library",
    "timeout_seconds": 10
  }')

echo "Found towers:"
echo "$RESULT" | jq -r '.towers[] | "  🗼 \(.id) - \(.address) - \(.steam_games) games"'

echo ""
echo "To connect to a tower:"
echo "  ./connect_to_tower.sh <tower-id>"
```

### 2. Connect to Tower

```bash
#!/bin/bash
# connect_to_tower.sh - Establish connection to tower

TOWER_ID=${1:-"gaming-tower-main"}

echo "🔗 Connecting to tower: $TOWER_ID"
echo ""

# Establish federation connection
curl -X POST http://localhost:8080/api/federation/connect \
  -d "{
    \"tower_id\": \"$TOWER_ID\",
    \"purpose\": \"steam-library-access\",
    \"capabilities_requested\": [
      \"steam-library\",
      \"game-execution\"
    ]
  }"

echo ""
echo "✅ Connected to tower!"
echo ""
echo "Next steps:"
echo "  1. Browse library: ./browse_remote_library.sh"
echo "  2. Launch game: ./launch_remote_game.sh <app_id>"
```

### 3. Browse Remote Library

```bash
#!/bin/bash
# browse_remote_library.sh - List games on tower

TOWER_ID=${1:-"gaming-tower-main"}

echo "📚 Games available on $TOWER_ID:"
echo ""

# Get library from tower
GAMES=$(curl -s http://localhost:8080/api/federation/tower/$TOWER_ID/library)

echo "$GAMES" | jq -r '.games[] | "  \(.app_id): \(.name) (\(.size_gb) GB)"' | head -20

echo ""
echo "... and more!"
echo ""
echo "To launch a game:"
echo "  ./launch_remote_game.sh <app_id>"
```

### 4. Launch Remote Game

```bash
#!/bin/bash
# launch_remote_game.sh - Launch game on tower, play on laptop

APP_ID=$1
TOWER_ID=${2:-"gaming-tower-main"}

if [ -z "$APP_ID" ]; then
    echo "Usage: $0 <app_id> [tower_id]"
    echo ""
    echo "Examples:"
    echo "  $0 730              # Counter-Strike"
    echo "  $0 440              # Team Fortress 2"
    echo "  $0 570              # Dota 2"
    exit 1
fi

echo "🎮 Launching game $APP_ID on $TOWER_ID"
echo ""

# Submit launch request
JOB=$(curl -s -X POST http://localhost:8080/api/federation/tower/$TOWER_ID/launch \
  -d "{
    \"app_id\": $APP_ID,
    \"mode\": \"remote\",
    \"stream_video\": true,
    \"stream_input\": true
  }")

JOB_ID=$(echo "$JOB" | jq -r '.job_id')

echo "  ✅ Game launched!"
echo "  📺 Job ID: $JOB_ID"
echo ""

# Get streaming endpoint
STREAM=$(curl -s http://localhost:8080/api/federation/tower/$TOWER_ID/job/$JOB_ID/stream)
STREAM_URL=$(echo "$STREAM" | jq -r '.stream_url')

echo "  📡 Stream URL: $STREAM_URL"
echo ""
echo "Game is running on tower, streaming to you!"
echo ""
echo "To monitor:"
echo "  curl http://localhost:8080/api/federation/tower/$TOWER_ID/job/$JOB_ID/status"
```

### 5. Multiplayer Across Towers

```bash
#!/bin/bash
# multiplayer_across_towers.sh - Play together across towers!

GAME=${1:-"StarCraft"}

echo "🎮 Setting up cross-tower multiplayer: $GAME"
echo ""

# Create gaming session that spans towers
curl -X POST http://localhost:8080/api/gaming/session/create \
  -d "{
    \"game\": \"$GAME\",
    \"federation\": true,
    \"allow_cross_tower\": true,
    \"max_players\": 8
  }"

echo ""
echo "✅ Cross-tower session created!"
echo ""
echo "Players can now join from:"
echo "  • Your laptop"
echo "  • Your tower"
echo "  • Friend's computer"
echo "  • Friend's tower"
echo ""
echo "All discovered automatically!"
```

---

## 🎯 Use Cases

### Use Case 1: Game Streaming

**Problem**: Laptop not powerful enough for modern games  
**Solution**: Run on tower GPU, stream to laptop

```bash
# Tower has RTX 4090, laptop has integrated graphics
./launch_remote_game.sh 1086940  # Baldur's Gate 3

# Game runs on tower
# Video streams to laptop
# Input streams from laptop
# Perfect experience!
```

### Use Case 2: Library Unification

**Problem**: Games installed on different machines  
**Solution**: Access all games from any device

```bash
# Discover all towers
./discover_tower.sh

# Shows:
# - Gaming Tower: 150 games
# - Office PC: 50 games
# - Friend's Server: 200 games

# Access 400 games from laptop!
```

### Use Case 3: Distributed Multiplayer

**Problem**: LAN party but limited hardware  
**Solution**: Use everyone's towers together

```bash
# Create distributed session
./multiplayer_across_towers.sh StarCraft

# Players connect from:
# - Their laptops (lightweight)
# - Their towers (executing)
# - Mix and match!

# Everyone plays, resources distributed!
```

---

## 🏗️ Technical Architecture

### Discovery Flow

```
1. Laptop starts Songbird
2. Broadcasts "looking for steam-library"
3. Tower responds "I have steam-library!"
4. Laptop connects
5. Metadata syncs (game list, not files!)
6. Ready to launch
```

### Launch Flow

```
1. User clicks "Launch" on laptop
2. Request sent to tower
3. Tower executes game
4. Video/input streaming established
5. User plays on laptop
6. Game runs on tower
```

### Multiplayer Flow

```
1. Create session (advertises to all towers)
2. Players join from anywhere
3. Songbird coordinates across towers
4. NAT traversal handled automatically
5. Everyone plays together!
```

---

## 🔧 Implementation Status

### ✅ Ready Now

| Feature | Status | Notes |
|---------|--------|-------|
| **Discovery** | ✅ Ready | Songbird has federation |
| **Connection** | ✅ Ready | Protocol established |
| **API Structure** | ✅ Ready | Endpoints defined |

### ⏳ Needs Implementation (1-2 weeks)

| Feature | Time | Status |
|---------|------|--------|
| **Library Sync** | 2 days | Metadata only |
| **Remote Launch** | 3 days | Via ToadStool |
| **Streaming** | 1 week | Video/input |
| **Testing** | 2 days | End-to-end |

### 🎯 Future Enhancements

| Feature | Complexity | Value |
|---------|-----------|-------|
| **Save Sync** | Medium | High |
| **Workshop Content** | Medium | Medium |
| **Cloud Saves** | Low | High |
| **Achievement Sync** | Low | Low |

---

## 🧪 Testing Plan

### Phase 1: Discovery (Day 1)

```bash
# Terminal 1 (Tower):
./advertise_tower.sh

# Terminal 2 (Laptop):
./discover_tower.sh

# Expected: Tower discovered!
```

### Phase 2: Connection (Day 2)

```bash
./connect_to_tower.sh gaming-tower-main

# Expected: Connection established!
```

### Phase 3: Browse Library (Day 3)

```bash
./browse_remote_library.sh

# Expected: List of 150 games!
```

### Phase 4: Remote Launch (Day 4-6)

```bash
./launch_remote_game.sh 730  # CS:GO

# Expected: Game launches on tower!
```

### Phase 5: Full System (Day 7)

```bash
# Complete workflow:
# 1. Discover
# 2. Connect
# 3. Browse
# 4. Launch
# 5. Play!
```

---

## 🎉 The Vision

### What This Becomes

**Your Personal Gaming Cloud!**

- ✅ Access any game from any device
- ✅ No cloud subscription needed
- ✅ Use your own hardware
- ✅ Complete privacy
- ✅ Zero latency (LAN)
- ✅ Scales infinitely (add more towers!)

### Beyond Gaming

**This enables**:
- ML training on remote GPUs
- Video rendering on server
- Data processing distributed
- Any compute, anywhere!

---

## 🚀 Next Steps

### This Week

1. **Test tower discovery**
   - Verify Songbird federation works
   - Test on your network
   - Document your tower's specs

2. **Create initial scripts**
   - Discovery working
   - Connection working
   - Library metadata sync

3. **Document your setup**
   - Tower IP/hostname
   - Steam library location
   - Available games

### Next Week

1. **Implement remote launch**
2. **Test with real games**
3. **Add streaming (if needed)**
4. **Polish and document**

---

## 💡 Your Tower Info

**Fill this in**:

```bash
# Tower Details
TOWER_IP="192.168.1.???"
TOWER_HOSTNAME="gaming-tower"
STEAM_PATH="/home/???/.steam/steam/steamapps"
GAME_COUNT="???"

# Network
NETWORK="192.168.1.0/24"
PORTS="8080 (Songbird), 6112 (Gaming)"

# Hardware
GPU="???"
CPU="???"
RAM="???"
```

---

## 🎊 This Is Going To Be Cool!

**Imagine**:
- Sitting with laptop
- Browse tower's 150 games
- Launch Baldur's Gate 3
- Runs on tower's RTX 4090
- Streams to laptop
- Perfect experience!

**All with ecoPrimals!** 🚀✨

Ready to connect to your tower? Let's do this! 🗼🎮

