#!/bin/bash
# ToadStool Showcase - Prerequisites Verification

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🍄 ToadStool Showcase - Prerequisites Check"
echo "==========================================="
echo ""

REQUIRED_OK=true
OPTIONAL_AVAILABLE=()
OPTIONAL_MISSING=()

# Required: Bash
echo -n "Checking Bash... "
if [ -n "$BASH_VERSION" ]; then
    echo -e "${GREEN}✓${NC} Bash $BASH_VERSION"
else
    echo -e "${RED}✗${NC} Bash not found"
    REQUIRED_OK=false
fi

# Required: Rust/Cargo
echo -n "Checking Rust/Cargo... "
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo -e "${GREEN}✓${NC} Rust $RUST_VERSION"
else
    echo -e "${RED}✗${NC} Rust/Cargo not found"
    echo "  Install from: https://rustup.rs"
    REQUIRED_OK=false
fi

# Required: ToadStool
echo -n "Checking ToadStool... "
if [ -f "../Cargo.toml" ] && grep -q "name = \"toadstool\"" "../Cargo.toml" 2>/dev/null; then
    echo -e "${GREEN}✓${NC} ToadStool repository found"
else
    echo -e "${YELLOW}⚠${NC} ToadStool repository not found (run from showcase/ directory)"
fi

# System resources
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
    echo -e "  ${YELLOW}⚠${NC} Warning: At least 2 CPU cores recommended for benchmarks"
fi

if [ "$MEMORY_GB" != "unknown" ] && [ "$MEMORY_GB" -lt 4 ]; then
    echo -e "  ${YELLOW}⚠${NC} Warning: At least 4GB RAM recommended"
fi

echo ""
echo "Optional Components:"
echo "-------------------"

# Optional: Docker
echo -n "Checking Docker... "
if command -v docker &> /dev/null; then
    if docker info &> /dev/null 2>&1; then
        DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | tr -d ',')
        echo -e "${GREEN}✓${NC} Docker $DOCKER_VERSION (running)"
        OPTIONAL_AVAILABLE+=("docker")
    else
        echo -e "${YELLOW}⚠${NC} Docker installed but not running"
        echo "  Start with: sudo systemctl start docker"
        OPTIONAL_MISSING+=("docker")
    fi
else
    echo -e "${YELLOW}⚠${NC} Docker not found (optional)"
    echo "  Install from: https://docs.docker.com/get-docker/"
    OPTIONAL_MISSING+=("docker")
fi

# Optional: Python
echo -n "Checking Python... "
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d'.' -f1)
    PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d'.' -f2)
    
    if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 11 ]; then
        echo -e "${GREEN}✓${NC} Python $PYTHON_VERSION"
        OPTIONAL_AVAILABLE+=("python")
    else
        echo -e "${YELLOW}⚠${NC} Python $PYTHON_VERSION (3.11+ recommended for best performance)"
        OPTIONAL_AVAILABLE+=("python")
    fi
else
    echo -e "${YELLOW}⚠${NC} Python not found (optional)"
    echo "  Install Python 3.11+ for Python runtime demos"
    OPTIONAL_MISSING+=("python")
fi

# Optional: jq (for JSON)
echo -n "Checking jq... "
if command -v jq &> /dev/null; then
    echo -e "${GREEN}✓${NC} jq installed (for JSON formatting)"
    OPTIONAL_AVAILABLE+=("jq")
else
    echo -e "${YELLOW}⚠${NC} jq not found (optional, for better result formatting)"
    OPTIONAL_MISSING+=("jq")
fi

# Optional: gnuplot (for charts)
echo -n "Checking gnuplot... "
if command -v gnuplot &> /dev/null; then
    echo -e "${GREEN}✓${NC} gnuplot installed (for charts)"
    OPTIONAL_AVAILABLE+=("gnuplot")
else
    echo -e "${YELLOW}⚠${NC} gnuplot not found (optional, for benchmark charts)"
    OPTIONAL_MISSING+=("gnuplot")
fi

# Optional: asciinema (for recording)
echo -n "Checking asciinema... "
if command -v asciinema &> /dev/null; then
    echo -e "${GREEN}✓${NC} asciinema installed (for demo recording)"
    OPTIONAL_AVAILABLE+=("asciinema")
else
    echo -e "${YELLOW}⚠${NC} asciinema not found (optional, for demo recording)"
    OPTIONAL_MISSING+=("asciinema")
fi

echo ""
echo "Summary:"
echo "--------"

if [ "$REQUIRED_OK" = true ]; then
    echo -e "${GREEN}✓${NC} All required components available"
else
    echo -e "${RED}✗${NC} Some required components missing"
    echo ""
    echo "Please install missing required components before running showcase."
    exit 1
fi

if [ ${#OPTIONAL_AVAILABLE[@]} -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Available: ${OPTIONAL_AVAILABLE[*]}"
fi

if [ ${#OPTIONAL_MISSING[@]} -gt 0 ]; then
    echo -e "${YELLOW}⚠${NC} Missing: ${OPTIONAL_MISSING[*]}"
    echo "  (Some features will be limited)"
fi

echo ""
echo "Showcase Capabilities:"
echo "---------------------"
echo -e "  ${GREEN}✓${NC} Multi-substrate hello (native always available)"

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " docker " ]]; then
    echo -e "  ${GREEN}✓${NC} Docker substrate available"
    echo -e "  ${GREEN}✓${NC} Live migration demo (FULL)"
else
    echo -e "  ${YELLOW}⊗${NC} Docker substrate unavailable"
    echo -e "  ${YELLOW}⊗${NC} Live migration demo (LIMITED)"
fi

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " python " ]]; then
    echo -e "  ${GREEN}✓${NC} Python substrate available"
    echo -e "  ${GREEN}✓${NC} Benchmarks available"
else
    echo -e "  ${YELLOW}⊗${NC} Python substrate unavailable"
    echo -e "  ${YELLOW}⊗${NC} Benchmarks limited"
fi

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " jq " ]]; then
    echo -e "  ${GREEN}✓${NC} JSON result formatting"
fi

if [[ " ${OPTIONAL_AVAILABLE[*]} " =~ " gnuplot " ]]; then
    echo -e "  ${GREEN}✓${NC} Benchmark charts"
fi

echo ""
echo -e "${GREEN}✅ Prerequisites check complete!${NC}"
echo ""
echo "Ready to run showcase:"
echo "  ./showcase.sh"
echo ""

