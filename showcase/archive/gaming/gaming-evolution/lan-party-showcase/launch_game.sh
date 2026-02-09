#!/bin/bash
# Launch Game - Simple game launcher script

GAME_PATH="$1"

if [ -z "$GAME_PATH" ]; then
    echo "Usage: ./launch_game.sh /path/to/game.exe"
    echo ""
    echo "Examples:"
    echo "  ./launch_game.sh /tmp/games/starcraft/StarCraft.exe"
    echo "  ./launch_game.sh /tmp/games/aoe2/age2_x1.exe"
    echo "  ./launch_game.sh /tmp/games/diablo2/Diablo2.exe"
    exit 1
fi

if [ ! -f "$GAME_PATH" ]; then
    echo "❌ Error: Game not found at $GAME_PATH"
    exit 1
fi

GAME_DIR=$(dirname "$GAME_PATH")
GAME_NAME=$(basename "$GAME_PATH")

echo "🎮 Launching Game"
echo "================"
echo "  Game: $GAME_NAME"
echo "  Path: $GAME_PATH"
echo "  Dir:  $GAME_DIR"
echo ""

# Check if it's a Windows .exe and we need Wine
if [[ "$GAME_PATH" == *.exe ]]; then
    if command -v wine &> /dev/null; then
        echo "🍷 Using Wine for Windows game..."
        cd "$GAME_DIR"
        wine "$GAME_NAME"
    else
        echo "⚠️  Wine not found. Install wine to run Windows games:"
        echo "  sudo apt install wine"
        exit 1
    fi
else
    # Native Linux game
    echo "🐧 Running native Linux game..."
    cd "$GAME_DIR"
    "./$GAME_NAME"
fi

echo ""
echo "✅ Game finished!"

