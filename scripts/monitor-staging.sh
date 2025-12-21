#!/usr/bin/env bash
# Continuous Monitoring Script for ToadStool Staging
# Run this in a terminal to monitor staging deployment

set -e

# Configuration
STAGING_HOST="${STAGING_HOST:-localhost}"
STAGING_PORT="${STAGING_PORT:-8080}"
BASE_URL="http://${STAGING_HOST}:${STAGING_PORT}"
CHECK_INTERVAL="${CHECK_INTERVAL:-5}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 ToadStool Staging Monitor"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Target: ${BASE_URL}"
echo "Interval: ${CHECK_INTERVAL}s"
echo "Started: $(date)"
echo ""
echo "Press Ctrl+C to stop monitoring"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Counters
CHECKS=0
SUCCESSES=0
FAILURES=0

# Monitoring loop
while true; do
    CHECKS=$((CHECKS + 1))
    TIMESTAMP=$(date '+%H:%M:%S')
    
    # Check health
    if health=$(curl -s "${BASE_URL}/health" 2>&1); then
        status="✅"
        SUCCESSES=$((SUCCESSES + 1))
    else
        status="❌"
        FAILURES=$((FAILURES + 1))
    fi
    
    # Check metrics (if available)
    if metrics=$(curl -s "${BASE_URL}/metrics" 2>&1 | head -5); then
        metrics_status="✅"
    else
        metrics_status="❌"
    fi
    
    # Calculate uptime percentage
    if [ $CHECKS -gt 0 ]; then
        uptime_pct=$(awk "BEGIN {printf \"%.2f\", ($SUCCESSES/$CHECKS)*100}")
    else
        uptime_pct="0.00"
    fi
    
    # Display status
    echo "[${TIMESTAMP}] Health: ${status} | Metrics: ${metrics_status} | Checks: ${CHECKS} | Uptime: ${uptime_pct}% | Failures: ${FAILURES}"
    
    # Alert on consecutive failures
    if [ $FAILURES -gt 5 ]; then
        echo ""
        echo "🚨 ALERT: Multiple failures detected!"
        echo "   Total checks: ${CHECKS}"
        echo "   Failures: ${FAILURES}"
        echo "   Consider investigating or rolling back."
        echo ""
    fi
    
    sleep $CHECK_INTERVAL
done

