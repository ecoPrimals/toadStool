#!/bin/bash
# ToadStool Showcase - Environment Setup

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🍄 ToadStool Showcase - Setup"
echo "=============================="
echo ""

# Create state directory
echo -n "Creating state directory... "
mkdir -p /tmp/toadstool-showcase
echo -e "${GREEN}✓${NC}"

# Create results directory
echo -n "Creating results directory... "
mkdir -p results
echo -e "${GREEN}✓${NC}"

# Check if Docker is available
if command -v docker &> /dev/null && docker info &> /dev/null 2>&1; then
    echo -n "Checking Docker... "
    echo -e "${GREEN}✓${NC} Docker available"
    DOCKER_AVAILABLE=true
else
    echo -n "Checking Docker... "
    echo -e "${YELLOW}⚠${NC} Docker not available (some demos will be skipped)"
    DOCKER_AVAILABLE=false
fi

# Check if Python is available
if command -v python3 &> /dev/null; then
    echo -n "Checking Python... "
    echo -e "${GREEN}✓${NC} Python available"
    PYTHON_AVAILABLE=true
else
    echo -n "Checking Python... "
    echo -e "${YELLOW}⚠${NC} Python not available (some demos will be skipped)"
    PYTHON_AVAILABLE=false
fi

# Create environment config
cat > /tmp/toadstool-showcase/config.env << EOF
# ToadStool Showcase Configuration
TOADSTOOL_STATE_DIR=/tmp/toadstool-showcase
TOADSTOOL_RESULTS_DIR=$(pwd)/results
DOCKER_AVAILABLE=${DOCKER_AVAILABLE}
PYTHON_AVAILABLE=${PYTHON_AVAILABLE}
SETUP_TIMESTAMP=$(date -Iseconds)
EOF

echo -n "Creating configuration... "
echo -e "${GREEN}✓${NC}"

echo ""
echo -e "${GREEN}✅ Setup complete!${NC}"
echo ""
echo "Environment ready for showcase:"
echo "  State directory:   /tmp/toadstool-showcase"
echo "  Results directory: $(pwd)/results"
echo "  Docker:            ${DOCKER_AVAILABLE}"
echo "  Python:            ${PYTHON_AVAILABLE}"
echo ""
echo "Ready to run demos!"
echo ""

