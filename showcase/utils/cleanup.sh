#!/bin/bash
# ToadStool Showcase - Cleanup

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🍄 ToadStool Showcase - Cleanup"
echo "==============================="
echo ""

# Stop any running showcase processes
echo -n "Stopping showcase processes... "
pkill -f "showcase-counter" 2>/dev/null || true
pkill -f "showcase-hello" 2>/dev/null || true
pkill -f "benchmark-" 2>/dev/null || true
echo -e "${GREEN}✓${NC}"

# Clean state directory
if [ -d "/tmp/toadstool-showcase" ]; then
    echo -n "Cleaning state directory... "
    rm -rf /tmp/toadstool-showcase
    echo -e "${GREEN}✓${NC}"
fi

# Clean test I/O directory
if [ -d "/tmp/toadstool-io-bench" ]; then
    echo -n "Cleaning test I/O directory... "
    rm -rf /tmp/toadstool-io-bench
    echo -e "${GREEN}✓${NC}"
fi

# Archive results if they exist
if [ -d "results" ] && [ "$(ls -A results 2>/dev/null)" ]; then
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    ARCHIVE_DIR="results/archive"
    mkdir -p "$ARCHIVE_DIR"
    
    echo -n "Archiving results... "
    for result in results/*.json results/*.txt 2>/dev/null; do
        if [ -f "$result" ]; then
            mv "$result" "$ARCHIVE_DIR/$(basename $result .${result##*.})_${TIMESTAMP}.${result##*.}"
        fi
    done
    echo -e "${GREEN}✓${NC}"
fi

echo ""
echo -e "${GREEN}✅ Cleanup complete!${NC}"
echo ""
echo "Showcase environment cleaned:"
echo "  • State directory removed"
echo "  • Test files removed"
echo "  • Results archived (if any)"
echo "  • Processes stopped"
echo ""
echo "Ready for fresh showcase run!"
echo ""

