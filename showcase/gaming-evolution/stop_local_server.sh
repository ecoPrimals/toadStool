#!/bin/bash
# stop_local_server.sh
# Stop the local server started by play_local.sh

echo "🛑 Stopping local OpenArena server..."

if [ -f /tmp/openarena-server.pid ]; then
    SERVER_PID=$(cat /tmp/openarena-server.pid)
    
    if kill -0 $SERVER_PID 2>/dev/null; then
        kill $SERVER_PID
        echo "✅ Server stopped (PID: $SERVER_PID)"
        rm /tmp/openarena-server.pid
    else
        echo "⚠️  Server not running (PID: $SERVER_PID)"
        rm /tmp/openarena-server.pid
    fi
else
    echo "⚠️  No server PID file found"
    echo ""
    echo "Searching for openarena processes..."
    
    # Try to find and kill any openarena server processes
    PIDS=$(pgrep -f "openarena.*dedicated")
    
    if [ -n "$PIDS" ]; then
        echo "Found server processes: $PIDS"
        kill $PIDS
        echo "✅ Servers stopped"
    else
        echo "No running servers found"
    fi
fi

echo ""
echo "Done."

