#!/bin/bash
# ToadStool Staging Deployment Script
# Auto-generated: November 12, 2025

set -e

echo "🍄 ToadStool Staging Deployment"
echo "================================"
echo ""

# Verify binary exists
if [ ! -f "target/release/toadstool-byob-server" ]; then
    echo "❌ Binary not found. Run: cargo build --release --bin toadstool-byob-server"
    exit 1
fi

echo "✅ Binary found: $(ls -lh target/release/toadstool-byob-server | awk '{print $5}')"
echo ""

# Set environment variables
echo "📋 Setting environment variables..."
export TOADSTOOL_HOST="${TOADSTOOL_HOST:-0.0.0.0}"
export TOADSTOOL_PORT="${TOADSTOOL_PORT:-9000}"
export RUST_LOG="${RUST_LOG:-info}"

echo "   TOADSTOOL_HOST=$TOADSTOOL_HOST"
echo "   TOADSTOOL_PORT=$TOADSTOOL_PORT"
echo "   RUST_LOG=$RUST_LOG"
echo ""

# Optional ecosystem integration
if [ -n "$SONGBIRD_ENDPOINT" ]; then
    echo "   SONGBIRD_ENDPOINT=$SONGBIRD_ENDPOINT"
fi
if [ -n "$BEARDOG_ENDPOINT" ]; then
    echo "   BEARDOG_ENDPOINT=$BEARDOG_ENDPOINT"
fi

echo ""
echo "🚀 Starting ToadStool server..."
echo "================================"
echo ""
echo "Health check: http://localhost:$TOADSTOOL_PORT/health"
echo "Capabilities: http://localhost:$TOADSTOOL_PORT/capabilities"
echo "Metrics: http://localhost:$TOADSTOOL_PORT/metrics"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Run the server
exec ./target/release/toadstool-byob-server
