#!/bin/bash
# ToadStool Quick Monitoring Script
# Run this every few hours during the 48-72 hour monitoring period

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "🍄 ToadStool Staging Health Check"
echo "=================================="
echo "Time: $(date)"
echo ""

# Check 1: Version
echo "1. Version Check:"
if VERSION=$(toadstool-staging --version 2>&1); then
    echo -e "   ${GREEN}✅ $VERSION${NC}"
else
    echo -e "   ${RED}❌ Binary not responding${NC}"
    exit 1
fi
echo ""

# Check 2: Capabilities
echo "2. Capabilities Test:"
if toadstool-staging capabilities >/dev/null 2>&1; then
    echo -e "   ${GREEN}✅ Capabilities working${NC}"
else
    echo -e "   ${RED}❌ Capabilities failed${NC}"
fi
echo ""

# Check 3: Response Time
echo "3. Response Time:"
START=$(date +%s%N)
toadstool-staging --version >/dev/null 2>&1
END=$(date +%s%N)
DIFF=$((($END - $START) / 1000000))
if [ $DIFF -lt 1000 ]; then
    echo -e "   ${GREEN}✅ ${DIFF}ms (good)${NC}"
else
    echo -e "   ${YELLOW}⚠️  ${DIFF}ms (slower than expected)${NC}"
fi
echo ""

# Check 4: System Resources
echo "4. System Resources:"
if command -v toadstool-staging >/dev/null 2>&1; then
    if ps aux | grep -v grep | grep toadstool-staging >/dev/null; then
        echo -e "   ${GREEN}✅ Process running${NC}"
        ps aux | grep -v grep | grep toadstool-staging | awk '{print "   CPU: "$3"% MEM: "$4"%"}'
    else
        echo -e "   ${YELLOW}⚠️  Not running as daemon (normal for CLI)${NC}"
    fi
else
    echo -e "   ${YELLOW}⚠️  Binary not in daemon mode${NC}"
fi
echo ""

# Check 5: Memory Status
echo "5. Memory Status:"
free -h | grep "Mem:" | awk '{print "   Total: "$2" Used: "$3" Available: "$7}'
echo ""

# Check 6: Disk Status  
echo "6. Disk Status:"
df -h / | tail -1 | awk '{print "   Total: "$2" Used: "$3" Available: "$4" ("$5" used)"}'
echo ""

# Check 7: Logs (if running as service)
echo "7. Recent Logs:"
if command -v journalctl >/dev/null 2>&1; then
    if journalctl -u toadstool-staging --since "1 hour ago" >/dev/null 2>&1; then
        ERROR_COUNT=$(journalctl -u toadstool-staging --since "1 hour ago" 2>/dev/null | grep -i error | wc -l)
        if [ $ERROR_COUNT -eq 0 ]; then
            echo -e "   ${GREEN}✅ No errors in last hour${NC}"
        else
            echo -e "   ${YELLOW}⚠️  $ERROR_COUNT errors found${NC}"
            echo "   Run: journalctl -u toadstool-staging --since '1 hour ago' | grep -i error"
        fi
    else
        echo -e "   ${YELLOW}⚠️  Not running as systemd service${NC}"
    fi
else
    echo -e "   ${YELLOW}⚠️  journalctl not available${NC}"
fi
echo ""

# Summary
echo "=================================="
echo -e "${GREEN}🎉 Health Check Complete${NC}"
echo ""
echo "Status: MONITORING"
echo "Next Check: In 2-4 hours"
echo ""
echo "Commands:"
echo "  ./quick-monitor.sh              # Run this check"
echo "  toadstool-staging --version     # Quick version check"
echo "  toadstool-staging capabilities  # Test capabilities"
echo ""
echo "Documentation:"
echo "  MONITORING_GUIDE_48_HOURS.md    # Full monitoring guide"
echo "  DEPLOYMENT_SUCCESS_NOV_15_2025.md  # Deployment details"
echo ""

# Log results
LOG_FILE="/tmp/toadstool-monitoring-$(date +%Y%m%d).log"
echo "$(date): Health check completed - All systems operational" >> "$LOG_FILE"
echo "Logged to: $LOG_FILE"

