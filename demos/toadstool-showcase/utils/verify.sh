#!/bin/bash
# ToadStool Showcase - Prerequisites Verification
# Checks if all required and optional components are available

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "🍄 ToadStool Showcase - Prerequisites Check"
echo "==========================================="
echo ""

# Track what's available
REQUIRED_OK=true
OPTIONAL_AVAILABLE=()
OPTIONAL_MISSING=()

# Check required: Bash
echo -n "Checking Bash... "
if [ -n "$BASH_VERSION" ]; then
    echo -e "${GREEN}✓${NC} Bash $BASH_VERSION"
else
    echo -e "${RED}✗${NC} Bash not found"
    REQUIRED_OK=false
fi

# Check required: ToadStool (for now just check if we're in the repo)
echo -n "Checking ToadStool... "
if [ -f "../../Cargo.toml" ] && grep -q "name = \"toadstool\"" "../../Cargo.toml" 2>/dev/null; then
    echo -e "${GREEN}✓${NC} ToadStool repository found"
else
    echo -e "${YELLOW}⚠${NC} ToadStool not built yet (will use cargo run)"
fi

# Check required: Cargo/Rust
echo -n "Checking Rust/Cargo... "
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo -e "${GREEN}✓${NC} Rust $RUST_VERSION"
else
    echo -e "${RED}✗${NC} Rust/Cargo not found"
    echo "  Install from: https://rustup.rs"
    REQUIRED_OK=false
fi

# Check system resources
echo -n "Checking system resources... "
if command -v nproc &> /dev/null; then
    CPU_CORES=$(nproc)
elif command -v sysctl &> /dev/null; then
    CPU_CORES=$(sysctl -n hw.ncpu)
else
    CPU_CORES="unknown"
fi

if command -v free &> /dev/null; then
    MEMORY_GB=$(free -g | grep Mem | awk '{print $2}')
elif command -v sysctl &> /dev/null; then
    MEMORY_BYTES=$(sysctl -n hw.memsize)
    MEMORY_GB=$((MEMORY_BYTES / 1024 / 1024 / 1024))
else
    MEMORY_GB="unknown"
fi

echo -e "${GREEN}✓${NC} $CPU_CORES CPU cores, ${MEMORY_GB}GB RAM"

if [ "$CPU_CORES" != "unknown" ] && [ "$CPU_CORES" -lt 2 ]; then
    echo -e "  ${YELLOW}⚠${NC} Warning: At least 2 CPU cores recommended"
fi

if [ "$MEMORY_GB" != "unknown" ] && [ "$MEMORY_GB" -lt 4 ]; then
    echo -e "  ${YELLOW}⚠${NC} Warning: At least 4GB RAM recommended"
fi

echo ""
echo "Optional Components:"
echo "-------------------"

# Check optional: Docker
echo -n "Checking Docker... "
if command -v docker &> /dev/null; then
    if docker info &> /dev/null; then
        DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | tr -d ',')
        echo -e "${GREEN}✓${NC} Docker $DOCKER_VERSION (running)"
        OPTIONAL_AVAILABLE+=("docker")
    else
        echo -e "${YELLOW}⚠${NC} Docker installed but not running"
        echo "  Start with: sudo systemctl start docker"
        OPTIONAL_MISSING+=("docker")
    fi
else
    echo -e "${YELLOW}⚠${NC} Docker not found"
    echo "  Install from: https://docs.docker.com/get-docker/"
    echo "  (Optional: Enables container substrate demos)"
    OPTIONAL_MISSING+=("docker")
fi

# Check optional: Python
echo -n "Checking Python... "
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d'.' -f1)
    PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d'.' -f2)
    
    if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 11 ]; then
        echo -e "${GREEN}✓${NC} Python $PYTHON_VERSION"
        OPTIONAL_AVAILABLE+=("python")
    else
        echo -e "${YELLOW}⚠${NC} Python $PYTHON_VERSION (3.11+ recommended)"
        OPTIONAL_AVAILABLE+=("python")
    fi
else
    echo -e "${YELLOW}⚠${NC} Python not found"
    echo "  Install Python 3.11+ for Python runtime demos"
    echo "  (Optional: Enables Python substrate demos)"
    OPTIONAL_MISSING+=("python")
fi

# Check optional: jq (for JSON parsing)
echo -n "Checking jq... "
if command -v jq &> /dev/null; then
    echo -e "${GREEN}✓${NC} jq installed"
    OPTIONAL_AVAILABLE+=("jq")
else
    echo -e "${YELLOW}⚠${NC} jq not found (optional, for better output formatting)"
    OPTIONAL_MISSING+=("jq")
fi

echo ""
echo "Summary:"
echo "--------"

if [ "$REQUIRED_OK" = true ]; then
    echo -e "${GREEN}✓${NC} All required components available"
else
    echo -e "${RED}✗${NC} Some required components missing"
    echo ""
    echo "Please install missing required components before running the showcase."
    exit 1
fi

if [ ${#OPTIONAL_AVAILABLE[@]} -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Available optional components: ${OPTIONAL_AVAILABLE[*]}"
fi

if [ ${#OPTIONAL_MISSING[@]} -gt 0 ]; then
    echo -e "${YELLOW}⚠${NC} Missing optional components: ${OPTIONAL_MISSING[*]}"
    echo "  Some demo phases will be skipped"
fi

echo ""
echo "Demo Capabilities:"
echo "-----------------"
echo -e "  ${GREEN}✓${NC} Phase 1: Hello Universal (native always available)"

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " docker " ]]; then
    echo -e "  ${GREEN}✓${NC} Phase 1: Hello Universal (docker available)"
else
    echo -e "  ${YELLOW}⊗${NC} Phase 1: Hello Universal (docker will be skipped)"
fi

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " python " ]]; then
    echo -e "  ${GREEN}✓${NC} Phase 1: Hello Universal (python available)"
else
    echo -e "  ${YELLOW}⊗${NC} Phase 1: Hello Universal (python will be skipped)"
fi

echo -e "  ${GREEN}✓${NC} Phase 2: Intelligence (always available)"

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " docker " ]]; then
    echo -e "  ${GREEN}✓${NC} Phase 3: Live Migration (docker available - FULL DEMO)"
else
    echo -e "  ${YELLOW}⊗${NC} Phase 3: Live Migration (limited without docker)"
fi

echo -e "  ${GREEN}✓${NC} Phase 4: Substrate Diversity"
echo -e "  ${GREEN}✓${NC} Phase 5: Failover"

echo ""
echo -e "${GREEN}✅ Prerequisites check complete!${NC}"
echo ""
echo "Ready to run showcase? Execute:"
echo "  ./showcase.sh"
echo ""

