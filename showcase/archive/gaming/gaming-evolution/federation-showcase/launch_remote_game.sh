#!/bin/bash
# launch_remote_game.sh - Launch a game on remote tower

set -e

APP_ID=$1
TOWER_ID=${2:-"gaming-tower-main"}

if [ -z "$APP_ID" ]; then
    echo "Usage: $0 <app_id> [tower_id]"
    echo ""
    echo "Examples:"
    echo "  $0 730              # Counter-Strike"
    echo "  $0 440              # Team Fortress 2"
    echo "  $0 570              # Dota 2"
    echo "  $0 1086940          # Baldur's Gate 3"
    echo ""
    echo "To browse available games:"
    echo "  ./browse_remote_library.sh $TOWER_ID"
    exit 1
fi

echo "🎮 Launching Game on Tower"
echo "=========================="
echo ""
echo "  App ID: $APP_ID"
echo "  Tower: $TOWER_ID"
echo ""

# Check if Songbird is running
if ! curl -s http://localhost:8080/health &>/dev/null; then
    echo "❌ Songbird not running!"
    exit 1
fi

# Submit launch request
echo "Submitting launch request..."
JOB=$(curl -s -X POST http://localhost:8080/api/federation/tower/$TOWER_ID/launch \
  -H "Content-Type: application/json" \
  -d "{
    \"app_id\": $APP_ID,
    \"mode\": \"remote\",
    \"stream_video\": true,
    \"stream_input\": true
  }")

# Check result
STATUS=$(echo "$JOB" | jq -r '.status' 2>/dev/null || echo "error")

if [ "$STATUS" = "error" ]; then
    echo "❌ Failed to launch game"
    echo ""
    echo "Error: $(echo "$JOB" | jq -r '.error')"
    exit 1
fi

JOB_ID=$(echo "$JOB" | jq -r '.job_id')

echo "✅ Game launch initiated!"
echo ""
echo "  Job ID: $JOB_ID"
echo "  Status: $STATUS"
echo ""

# Get streaming info
echo "Getting streaming endpoint..."
STREAM=$(curl -s http://localhost:8080/api/federation/tower/$TOWER_ID/job/$JOB_ID/stream)
STREAM_URL=$(echo "$STREAM" | jq -r '.stream_url' 2>/dev/null)

if [ -n "$STREAM_URL" ] && [ "$STREAM_URL" != "null" ]; then
    echo "  📡 Stream URL: $STREAM_URL"
    echo ""
    echo "Game is running on tower!"
    echo ""
    echo "To view stream:"
    echo "  mpv $STREAM_URL"
    echo "  # or use any video player that supports streaming"
else
    echo "Game is running on tower in local mode"
    echo ""
fi

echo ""
echo "To monitor job status:"
echo "  curl http://localhost:8080/api/federation/tower/$TOWER_ID/job/$JOB_ID/status | jq"
echo ""
echo "To stop the game:"
echo "  curl -X DELETE http://localhost:8080/api/federation/tower/$TOWER_ID/job/$JOB_ID"

