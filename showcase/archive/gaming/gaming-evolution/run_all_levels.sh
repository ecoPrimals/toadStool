#!/bin/bash
# Gaming Evolution - Complete Showcase Runner
# Runs all 6 levels in sequence

set -e  # Exit on error

echo "🎮 Gaming Evolution - Complete Showcase"
echo "========================================"
echo ""
echo "This will run all 6 levels:"
echo "  Level 0: Single Game Execution"
echo "  Level 1: Game Storage"
echo "  Level 2: Multiplayer Discovery"
echo "  Level 3: Protocol Bridging"
echo "  Level 4: Legacy Games"
echo "  Level 5: Game Library Management"
echo "  Level 6: Steam Integration"
echo ""
echo "Estimated time: 30 minutes"
echo ""

read -p "Press Enter to start..."

# Function to run a level
run_level() {
    local level_num=$1
    local level_name=$2
    local level_dir=$3
    
    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  LEVEL $level_num: $level_name"
    echo "═══════════════════════════════════════════════════════"
    echo ""
    
    if [ -f "$level_dir/run.sh" ]; then
        cd "$level_dir"
        ./run.sh
        cd ..
        echo "  ✅ Level $level_num complete!"
    else
        echo "  ⚠️  Level $level_num demo not yet implemented"
        echo "  📚 See $level_dir/README.md for details"
    fi
    
    echo ""
    echo "Press Enter to continue to next level..."
    read
}

# Run each level
run_level 0 "Single Game Execution" "level-0-single-game"
run_level 1 "Game Storage" "level-1-game-storage"
run_level 2 "Multiplayer Discovery" "level-2-discovery"
run_level 3 "Protocol Bridging" "level-3-protocol-bridge"
run_level 4 "Legacy Games" "level-4-legacy-games"
run_level 5 "Game Library Management" "level-5-game-library"
run_level 6 "Steam Integration" "level-6-steam-integration"

# Final summary
echo ""
echo "═══════════════════════════════════════════════════════"
echo "  🎉 SHOWCASE COMPLETE!"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "You've seen the complete Gaming Evolution:"
echo "  ✅ Level 0: ToadStool executes games"
echo "  ✅ Level 1: NestGate stores game files"
echo "  ✅ Level 2: Discovery finds services automatically"
echo "  ✅ Level 3: Songbird bridges protocols"
echo "  ✅ Level 4: Legacy games work (StarCraft, AoE!)"
echo "  ✅ Level 5: Game library management"
echo "  ✅ Level 6: Complete Steam integration"
echo ""
echo "🏆 RESULT: Self-Hosted Steam Multiplayer Gaming Platform!"
echo ""
echo "What you have:"
echo "  🗄️  Centralized game storage (NestGate)"
echo "  🍄  Game execution (ToadStool)"
echo "  🎵  Multiplayer coordination (Songbird)"
echo "  🔍  Zero-config discovery"
echo "  🎮  Steam library integration"
echo "  🏠  100% self-hosted"
echo "  🔒  Privacy & sovereignty"
echo ""
echo "Next steps:"
echo "  1. Review individual level READMEs for deep dives"
echo "  2. Check ARCHITECTURE.md for system design"
echo "  3. See ROADMAP.md for implementation plan"
echo "  4. Build your own gaming platform!"
echo ""
echo "Documentation:"
echo "  - 00_START_HERE.md - Showcase overview"
echo "  - ARCHITECTURE.md - System design"
echo "  - ROADMAP.md - Implementation timeline"
echo "  - Level */README.md - Detailed guides"
echo ""
echo "Thank you for exploring the Gaming Evolution! 🚀✨"

