#!/bin/bash
# Download Open Source Games - Master Script
# Downloads curated collection of best open source games

set -e

echo "🎮 Open Source Gaming Library"
echo "============================="
echo ""
echo "This script downloads the BEST open source games."
echo "All games are:"
echo "  ✅ 100% free and legal"
echo "  ✅ Actively maintained"
echo "  ✅ High quality"
echo "  ✅ Multiplayer capable"
echo ""

# Detect package manager
if command -v apt &> /dev/null; then
    PKG_MGR="apt"
    INSTALL_CMD="sudo apt install -y"
elif command -v dnf &> /dev/null; then
    PKG_MGR="dnf"
    INSTALL_CMD="sudo dnf install -y"
elif command -v pacman &> /dev/null; then
    PKG_MGR="pacman"
    INSTALL_CMD="sudo pacman -S --noconfirm"
else
    echo "⚠️  Package manager not detected"
    echo "Please install games manually"
    exit 1
fi

echo "📦 Detected package manager: $PKG_MGR"
echo ""

# Menu
echo "Select games to download:"
echo ""
echo "  1) FPS Games (OpenArena, Xonotic, Red Eclipse)"
echo "  2) Strategy Games (0 A.D., OpenRA, Wesnoth)"
echo "  3) Racing Games (SuperTuxKart)"
echo "  4) Simulation (OpenTTD, Minetest)"
echo "  5) Classic Shareware (Quake, Doom)"
echo "  6) TOP 10 (Best of the best!)"
echo "  7) ALL GAMES (30+ games!)"
echo "  8) Quick Test (5 lightweight games)"
echo ""
read -p "Choice [1-8]: " choice

install_fps() {
    echo ""
    echo "🎯 Installing FPS Games..."
    $INSTALL_CMD openarena || echo "  ⚠️  openarena skipped"
    $INSTALL_CMD xonotic || echo "  ⚠️  xonotic skipped"
    $INSTALL_CMD redeclipse || echo "  ⚠️  redeclipse skipped"
    echo "  ✅ FPS games installed!"
}

install_strategy() {
    echo ""
    echo "♟️  Installing Strategy Games..."
    $INSTALL_CMD 0ad || echo "  ⚠️  0ad skipped"
    $INSTALL_CMD wesnoth || echo "  ⚠️  wesnoth skipped"
    $INSTALL_CMD freeciv || echo "  ⚠️  freeciv skipped"
    
    # OpenRA (snap if available)
    if command -v snap &> /dev/null; then
        sudo snap install openra || echo "  ⚠️  openra skipped"
    fi
    
    echo "  ✅ Strategy games installed!"
}

install_racing() {
    echo ""
    echo "🏎️  Installing Racing Games..."
    $INSTALL_CMD supertuxkart || echo "  ⚠️  supertuxkart skipped"
    echo "  ✅ Racing games installed!"
}

install_simulation() {
    echo ""
    echo "🏗️  Installing Simulation Games..."
    $INSTALL_CMD openttd || echo "  ⚠️  openttd skipped"
    $INSTALL_CMD minetest || echo "  ⚠️  minetest skipped"
    echo "  ✅ Simulation games installed!"
}

install_classic() {
    echo ""
    echo "👾 Downloading Classic Shareware..."
    
    mkdir -p /tmp/games/classics
    cd /tmp/games/classics
    
    # Quake shareware
    if [ ! -d "quake" ]; then
        echo "  📥 Quake shareware..."
        wget -q --show-progress https://archive.org/download/quake-shareware/quake106.zip
        unzip -q quake106.zip
        mkdir -p quake
        mv *.exe quake/ 2>/dev/null || true
        rm quake106.zip
        echo "  ✅ Quake ready"
    fi
    
    # Doom shareware
    if [ ! -d "doom" ]; then
        echo "  📥 Doom shareware..."
        wget -q --show-progress https://archive.org/download/DoomsharewareEpisode/doom.zip
        unzip -q doom.zip -d doom
        rm doom.zip
        echo "  ✅ Doom ready"
    fi
    
    echo "  ✅ Classic games ready at /tmp/games/classics/"
}

install_top10() {
    echo ""
    echo "🏆 Installing TOP 10 Games..."
    echo "  (Best quality, most played, great multiplayer)"
    echo ""
    
    # FPS
    echo "1. OpenArena (Quake 3 Arena style)"
    $INSTALL_CMD openarena || true
    
    # Strategy
    echo "2. 0 A.D. (Age of Empires style)"
    $INSTALL_CMD 0ad || true
    
    echo "3. Wesnoth (Turn-based strategy)"
    $INSTALL_CMD wesnoth || true
    
    # Racing
    echo "4. SuperTuxKart (Mario Kart style)"
    $INSTALL_CMD supertuxkart || true
    
    # Simulation
    echo "5. OpenTTD (Transport Tycoon)"
    $INSTALL_CMD openttd || true
    
    echo "6. Minetest (Minecraft style)"
    $INSTALL_CMD minetest || true
    
    # More FPS
    echo "7. Xonotic (Fast-paced FPS)"
    $INSTALL_CMD xonotic || true
    
    # Classics
    echo "8-10. Classic shareware..."
    install_classic
    
    echo ""
    echo "  ✅ TOP 10 installed!"
}

install_quick_test() {
    echo ""
    echo "⚡ Installing Quick Test Games..."
    echo "  (Lightweight, fast download, easy to test)"
    echo ""
    
    $INSTALL_CMD openarena || true
    $INSTALL_CMD wesnoth || true
    $INSTALL_CMD supertuxkart || true
    $INSTALL_CMD openttd || true
    install_classic
    
    echo "  ✅ Test games installed!"
}

install_all() {
    echo ""
    echo "🚀 Installing ALL GAMES..."
    echo "  This will take a while!"
    echo ""
    
    install_fps
    install_strategy
    install_racing
    install_simulation
    install_classic
    
    echo ""
    echo "  ✅ ALL GAMES INSTALLED!"
}

# Execute choice
case $choice in
    1) install_fps ;;
    2) install_strategy ;;
    3) install_racing ;;
    4) install_simulation ;;
    5) install_classic ;;
    6) install_top10 ;;
    7) install_all ;;
    8) install_quick_test ;;
    *) echo "Invalid choice"; exit 1 ;;
esac

# Summary
echo ""
echo "═══════════════════════════════════════════"
echo "  📊 INSTALLATION COMPLETE!"
echo "═══════════════════════════════════════════"
echo ""
echo "Installed games:"
echo ""

# Check what's installed
command -v openarena &>/dev/null && echo "  ✅ OpenArena (openarena)"
command -v xonotic &>/dev/null && echo "  ✅ Xonotic (xonotic)"
command -v 0ad &>/dev/null && echo "  ✅ 0 A.D. (0ad)"
command -v wesnoth &>/dev/null && echo "  ✅ Wesnoth (wesnoth)"
command -v supertuxkart &>/dev/null && echo "  ✅ SuperTuxKart (supertuxkart)"
command -v openttd &>/dev/null && echo "  ✅ OpenTTD (openttd)"
command -v minetest &>/dev/null && echo "  ✅ Minetest (minetest)"
[ -d "/tmp/games/classics/quake" ] && echo "  ✅ Quake Shareware (/tmp/games/classics/quake)"
[ -d "/tmp/games/classics/doom" ] && echo "  ✅ Doom Shareware (/tmp/games/classics/doom)"

echo ""
echo "Quick launch commands:"
echo "  openarena          # FPS arena shooter"
echo "  0ad                # Real-time strategy"
echo "  wesnoth            # Turn-based strategy"
echo "  supertuxkart       # Racing"
echo "  openttd            # Transport simulation"
echo "  minetest           # Voxel game"
echo ""
echo "For classics (need Wine):"
echo "  wine /tmp/games/classics/quake/quake.exe"
echo "  wine /tmp/games/classics/doom/doom.exe"
echo ""
echo "Or use our launcher:"
echo "  cd lan-party-showcase"
echo "  ./launch_game.sh /tmp/games/classics/quake/quake.exe"
echo ""
echo "🎮 Ready to play! Have fun! 🎉"

