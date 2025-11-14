#!/bin/bash
# Gaming Activity Monitor
# Detects when gaming starts/stops and triggers priority changes

set -e

MONITOR_INTERVAL=5
GAMING_PROCESSES=("steam" "lutris" "wine" "wine64" "proton" "gamemode")
MANUAL_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --manual)
            MANUAL_MODE=true
            shift
            ;;
        --interval)
            MONITOR_INTERVAL="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "🔍 Gaming Activity Monitor"
echo "   Monitoring interval: ${MONITOR_INTERVAL}s"
echo "   Watching for: ${GAMING_PROCESSES[*]}"
echo ""

if [ "$MANUAL_MODE" = true ]; then
    echo "📌 Manual mode: Press G to start gaming, S to stop"
    echo ""
fi

GAMING_ACTIVE=false
LAST_STATE="idle"

# Signal file for IPC
SIGNAL_FILE="/tmp/toadstool-gaming-signal"
echo "idle" > "$SIGNAL_FILE"

detect_gaming() {
    for process in "${GAMING_PROCESSES[@]}"; do
        if pgrep -x "$process" > /dev/null 2>&1; then
            return 0  # Gaming detected
        fi
    done
    return 1  # No gaming
}

while true; do
    TIMESTAMP=$(date +"%H:%M:%S")
    
    if [ "$MANUAL_MODE" = false ]; then
        # Automatic detection
        if detect_gaming; then
            if [ "$GAMING_ACTIVE" = false ]; then
                echo "[$TIMESTAMP] 🎮 GAMING STARTED"
                echo "gaming" > "$SIGNAL_FILE"
                GAMING_ACTIVE=true
            fi
        else
            if [ "$GAMING_ACTIVE" = true ]; then
                echo "[$TIMESTAMP] 🎮 GAMING STOPPED"
                echo "idle" > "$SIGNAL_FILE"
                GAMING_ACTIVE=false
            fi
        fi
    else
        # Manual mode (for demo)
        if [ -f "/tmp/gaming-manual-start" ]; then
            if [ "$GAMING_ACTIVE" = false ]; then
                echo "[$TIMESTAMP] 🎮 GAMING STARTED (manual)"
                echo "gaming" > "$SIGNAL_FILE"
                GAMING_ACTIVE=true
                rm -f "/tmp/gaming-manual-start"
            fi
        fi
        
        if [ -f "/tmp/gaming-manual-stop" ]; then
            if [ "$GAMING_ACTIVE" = true ]; then
                echo "[$TIMESTAMP] 🎮 GAMING STOPPED (manual)"
                echo "idle" > "$SIGNAL_FILE"
                GAMING_ACTIVE=false
                rm -f "/tmp/gaming-manual-stop"
            fi
        fi
    fi
    
    sleep "$MONITOR_INTERVAL"
done

