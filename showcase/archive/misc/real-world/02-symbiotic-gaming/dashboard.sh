#!/bin/bash
# Launch the Symbiotic GPU Manager Dashboard

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 not found"
    echo "Please install Python 3 to run the dashboard"
    exit 1
fi

# Make sure signal file exists
touch /tmp/toadstool-gaming-signal
echo "idle" > /tmp/toadstool-gaming-signal

echo "🎮 Launching Symbiotic GPU Manager Dashboard..."
echo ""
echo "Controls:"
echo "  Q - Quit"
echo "  G - Toggle gaming simulation"
echo ""
sleep 2

# Run the dashboard
python3 "$SCRIPT_DIR/dashboard.py"

