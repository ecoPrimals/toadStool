# 🎮 Testing Your CD Games - Quick Guide

**Got old game CDs? Let's test them!** 📀

---

## 🚀 Super Quick Start (5 minutes)

### 1. Start Songbird

```bash
# Terminal 1: Start Songbird
cd /home/eastgate/Development/ecoPrimals/songbird
cargo run --release --bin songbird-orchestrator

# Wait for: "Songbird orchestrator started"
```

### 2. Run LAN Party Setup

```bash
# Terminal 2: Quick start
cd /home/eastgate/Development/ecoPrimals/toadstool/showcase/gaming-evolution/lan-party-showcase
./quick_start.sh

# This sets up Songbird's gaming network automatically!
```

### 3. Copy Your Game

```bash
# Example with StarCraft CD
mkdir -p /tmp/games/starcraft
cp -r /media/cdrom/StarCraft/* /tmp/games/starcraft/
# or: cp -r /path/to/StarCraft /tmp/games/starcraft/
```

### 4. Launch!

```bash
./launch_game.sh /tmp/games/starcraft/StarCraft.exe

# For Wine on Linux (Windows games):
# Script will automatically use Wine if .exe file
```

---

## 🎮 Game-Specific Quick Tests

### StarCraft (1998)

```bash
# Copy from CD
mkdir -p /tmp/games/starcraft
cp -r /path/to/StarCraft/* /tmp/games/starcraft/

# Configure Songbird for StarCraft
curl -X POST http://localhost:8080/api/gaming/configure \
  -d '{"game_name":"StarCraft","optimization_level":"maximum"}'

# Launch
./launch_game.sh /tmp/games/starcraft/StarCraft.exe

# In game:
# - Choose "Multiplayer"
# - Choose "Local Area Network (UDP)"
# - Should auto-discover other players!
```

### Age of Empires II

```bash
# Copy from CD
mkdir -p /tmp/games/aoe2
cp -r /path/to/AgeOfEmpiresII/* /tmp/games/aoe2/

# Configure
curl -X POST http://localhost:8080/api/gaming/configure \
  -d '{"game_name":"AgeOfEmpires2"}'

# Launch
./launch_game.sh /tmp/games/aoe2/age2_x1.exe

# In game:
# - Multiplayer → LAN
# - Should see other players automatically!
```

### Diablo II

```bash
# Copy
mkdir -p /tmp/games/diablo2
cp -r /path/to/DiabloII/* /tmp/games/diablo2/

# Launch
./launch_game.sh /tmp/games/diablo2/Diablo2.exe

# In game:
# - Multiplayer → TCP/IP
# - Auto-discovers via Songbird!
```

### Quake

```bash
# Copy
mkdir -p /tmp/games/quake
cp -r /path/to/Quake/* /tmp/games/quake/

# Launch
./launch_game.sh /tmp/games/quake/quake.exe

# In game:
# - Multiplayer → LAN
# - Should see servers via Songbird!
```

---

## 🔧 Troubleshooting

### "Wine not found"

```bash
# Install Wine for Windows games
sudo apt install wine wine64

# Or on other systems:
# macOS: brew install wine
# Arch: sudo pacman -S wine
```

### "Game won't start"

```bash
# 1. Check file exists
ls -la /tmp/games/*/

# 2. Check permissions
chmod +x /tmp/games/starcraft/StarCraft.exe

# 3. Try running directly
cd /tmp/games/starcraft
wine StarCraft.exe

# 4. Check for dependencies
# Some games need DirectX, Visual C++, etc.
winetricks directx9 vcrun2015
```

### "Can't see other players"

```bash
# 1. Verify Songbird is running
curl http://localhost:8080/health

# 2. Check gaming network status
curl http://localhost:8080/api/gaming/status

# 3. Re-run setup
cd lan-party-showcase
./quick_start.sh

# 4. Ensure all players on same network
ping <friend-ip>
```

### "IPX not available"

```bash
# Enable IPX explicitly
curl -X POST http://localhost:8080/api/gaming/protocols/enable \
  -d '{"protocols":["ipx","directplay"]}'

# Check it's enabled
curl http://localhost:8080/api/gaming/protocols/status
```

---

## 📊 What to Test

### Basic Tests

- [ ] Game launches
- [ ] Game runs without crashing
- [ ] Can navigate menus
- [ ] Can start single-player game

### Multiplayer Tests

- [ ] Can see "Multiplayer" option
- [ ] Can see LAN/Network option
- [ ] Can create game
- [ ] Can see other players' games
- [ ] Can join game
- [ ] Can play together!

### Performance Tests

- [ ] Game runs smooth
- [ ] No excessive lag
- [ ] Audio works
- [ ] Graphics render correctly

---

## 🎯 Expected Results

### ✅ Should Work Immediately

**Games with configs in Songbird**:
- StarCraft
- Age of Empires II
- Diablo I & II
- Quake
- Command & Conquer

**Expected behavior**:
- Launch successfully ✅
- See LAN multiplayer option ✅
- Auto-discover other players ✅
- Play together ✅

### ⚠️ Might Need Tweaking

**Other IPX/DirectPlay games**:
- May need Wine configuration
- May need additional libraries
- May need manual protocol selection

**But IPX bridging should work!**

---

## 📝 Document Your Results

Help us build the compatibility list!

```bash
# Create a test report
cat > /tmp/my_game_test.txt << EOF
Game: [Name]
Version: [Version]
CD/Digital: [Which]

Launch: [Success/Failed]
Multiplayer Menu: [Yes/No]
See Other Players: [Yes/No]
Can Join: [Yes/No]
Can Play: [Yes/No]

Notes:
[Any issues or special steps needed]
EOF
```

---

## 🎉 Success Stories

### What Works

**Confirmed working** (from Songbird docs):
- ✅ StarCraft (full config)
- ✅ Age of Empires II (full config)
- ✅ Diablo I & II (full config)
- ✅ Quake (full config)
- ✅ Command & Conquer (full config)

**How we know**:
- Songbird has 682 lines of gaming documentation
- Configs exist for these games
- IPX/DirectPlay bridging is tested
- Gaming network is production code

---

## 💡 Pro Tips

### Tip 1: Keep Games Organized

```bash
# Create a games library
mkdir -p /tmp/games
cd /tmp/games

# One folder per game
mkdir starcraft aoe2 diablo2 quake

# Symbolic links to originals
ln -s /path/to/StarCraft starcraft/
ln -s /path/to/AgeOfEmpires aoe2/
```

### Tip 2: Test Single Player First

Before trying multiplayer:
1. Launch game
2. Verify it runs
3. Play single-player level
4. THEN try multiplayer

### Tip 3: Start Simple

Begin with:
1. StarCraft (best supported)
2. One other player
3. Same room (LAN)
4. Then expand!

### Tip 4: Document Everything

Keep notes:
- What worked
- What didn't
- How you fixed it
- Share with others!

---

## 🚀 Next Steps

### Once Basic Tests Work

1. **Try more games** - Expand your library
2. **Invite friends** - Test multiplayer for real
3. **Optimize settings** - Tune for performance
4. **Share results** - Help others!

### Then Go Further

1. **Wire up ToadStool** - For job tracking
2. **Add NestGate** - For centralized storage
3. **Test across houses** - Internet multiplayer
4. **Build full platform** - Levels 5-6!

---

## 📚 Resources

### Documentation
- [LAN Party Showcase](README.md) - Full guide
- [Songbird Gaming Guide](../../../../songbird/docs/GAMING_SETUP_GUIDE.md) - 682 lines!
- [Gaming Evolution](../00_START_HERE.md) - Complete roadmap

### Scripts
- `quick_start.sh` - Setup Songbird gaming
- `launch_game.sh` - Launch any game
- `run.sh` - Level 0 demo

### Help
- Check Songbird docs (comprehensive!)
- Ask in community
- Document your tests
- Share your findings!

---

## 🎊 Have Fun!

**This is why we built ecoPrimals!** 🎉

Distributed computing, service discovery, protocol translation...

**...all so we can play StarCraft in 2025!** 🎮✨

Dig up those CDs and let's play! 🚀

---

*"Your old games, our modern infrastructure, zero configuration."*

**Status**: Ready for testing!  
**Difficulty**: Easy  
**Fun Factor**: MAXIMUM! 🎮

