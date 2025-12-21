#!/bin/bash
# install_and_launch_openarena.sh
# Quick script to install and launch OpenArena!

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║          🎮 Installing & Launching OpenArena! 🎮             ║"
echo "║                                                              ║"
echo "║     (Quake 3 Arena style FPS - 16 player multiplayer!)      ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check if already installed
if command -v openarena &> /dev/null; then
    echo "✅ OpenArena is already installed!"
    echo ""
else
    echo "📥 Installing OpenArena..."
    echo "   (This will ask for your sudo password)"
    echo ""
    
    sudo apt update
    sudo apt install -y openarena
    
    echo ""
    echo "✅ OpenArena installed successfully!"
    echo ""
fi

# Show game info
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  OPENARENA - Quake 3 Arena Style FPS                         ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "  📊 Quality: ⭐⭐⭐⭐⭐"
echo "  👥 Players: 16 multiplayer"
echo "  📦 Size: ~500MB"
echo "  🎯 Genre: Fast-paced arena FPS"
echo ""
echo "  Controls:"
echo "    WASD - Move"
echo "    Mouse - Aim"
echo "    Left Click - Shoot"
echo "    Space - Jump"
echo "    Numbers - Switch weapons"
echo ""
echo "  Quick Start:"
echo "    1. Main Menu → Multiplayer → Create Server"
echo "    2. Choose map (dm17 is great!)"
echo "    3. Start playing!"
echo ""
echo "  For LAN multiplayer:"
echo "    - Host creates server"
echo "    - Friends choose 'Join Server'"
echo "    - Auto-discovery on LAN!"
echo ""

read -p "🚀 Ready to launch OpenArena? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "🎮 Launching OpenArena..."
    echo ""
    echo "   Tip: Press ~ for console, type /quit to exit"
    echo ""
    sleep 2
    
    # Launch OpenArena
    openarena
    
    echo ""
    echo "✅ Thanks for playing!"
else
    echo ""
    echo "No problem! Launch anytime with: openarena"
    echo ""
    echo "Or try other games:"
    echo "  cd showcase/gaming-evolution/opensource-library"
    echo "  ./download_all_games.sh"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  🎊 ecoPrimals Gaming Showcase - WORKING! 🎊"
echo "═══════════════════════════════════════════════════════════════"

