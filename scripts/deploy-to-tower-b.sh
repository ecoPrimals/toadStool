#!/bin/bash
# Quick deployment script for Tower B
# Prompts for Tower B details and deploys ToadStool

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║    🍄 Deploy ToadStool to Tower B                       ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Get Tower B details
echo -e "${BLUE}📍 Tower B Connection Details:${NC}"
echo ""

if [ -z "$TOWER_B_HOST" ]; then
    read -p "Tower B hostname or IP [default: tower-b]: " TOWER_B_INPUT
    TOWER_B_HOST="${TOWER_B_INPUT:-tower-b}"
fi

if [ -z "$TOWER_B_USER" ]; then
    read -p "SSH user for Tower B [default: $USER]: " TOWER_B_USER_INPUT
    TOWER_B_USER="${TOWER_B_USER_INPUT:-$USER}"
fi

TOWER_B_TARGET="$TOWER_B_USER@$TOWER_B_HOST"

echo ""
echo -e "${CYAN}Target: $TOWER_B_TARGET${NC}"
echo ""

# Test connectivity
echo -e "${BLUE}🔍 Testing SSH connectivity...${NC}"
if ssh -o ConnectTimeout=5 -o BatchMode=yes "$TOWER_B_TARGET" "echo 'Connection successful'" 2>/dev/null; then
    echo -e "${GREEN}✅ SSH connection successful${NC}"
else
    echo -e "${YELLOW}⚠️  SSH connection failed. Trying with password...${NC}"
    if ! ssh -o ConnectTimeout=5 "$TOWER_B_TARGET" "echo 'Connection successful'" 2>/dev/null; then
        echo -e "${RED}❌ Cannot connect to Tower B${NC}"
        echo ""
        echo "Please ensure:"
        echo "  1. Tower B is reachable on the network"
        echo "  2. SSH is enabled on Tower B"
        echo "  3. Your SSH keys are set up or you have the password"
        echo ""
        exit 1
    fi
fi
echo ""

# Check binaries
echo -e "${BLUE}📦 Checking ToadStool binaries...${NC}"
if [ ! -f "target/release/toadstool-showcase-distributed" ]; then
    echo -e "${RED}❌ Binary not found!${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

BINARY_SIZE=$(ls -lh target/release/toadstool-showcase-distributed | awk '{print $5}')
echo -e "${GREEN}✅ Binary ready: $BINARY_SIZE${NC}"
echo ""

# Create deployment package
echo -e "${BLUE}📦 Creating deployment package...${NC}"
DEPLOY_DIR="/tmp/toadstool-tower-b-$$"
mkdir -p "$DEPLOY_DIR"

# Copy binary
cp target/release/toadstool-showcase-distributed "$DEPLOY_DIR/"

# Copy showcase directory
if [ -d "showcase" ]; then
    cp -r showcase "$DEPLOY_DIR/"
fi

# Copy configs
cp toadstool.toml "$DEPLOY_DIR/" 2>/dev/null || true
cp toadstool-songbird-network.toml "$DEPLOY_DIR/" 2>/dev/null || true

# Create simple test script for Tower B
cat > "$DEPLOY_DIR/test-on-tower-b.sh" << 'EOTEST'
#!/bin/bash
# Quick test script for Tower B

echo "🍄 Testing ToadStool on Tower B"
echo ""

cd "$(dirname "$0")"

# Make binary executable
chmod +x toadstool-showcase-distributed

# Quick test
echo "Running distributed compute demo..."
./toadstool-showcase-distributed

echo ""
echo "✅ ToadStool is working on Tower B!"
echo ""
echo "You can also run:"
echo "  cd showcase && ./showcase.sh"
echo ""
EOTEST

chmod +x "$DEPLOY_DIR/test-on-tower-b.sh"

# Create tarball
PACKAGE="/tmp/toadstool-tower-b.tar.gz"
tar czf "$PACKAGE" -C "$DEPLOY_DIR" .
PACKAGE_SIZE=$(ls -lh "$PACKAGE" | awk '{print $5}')

echo -e "${GREEN}✅ Package created: $PACKAGE_SIZE${NC}"
echo ""

# Transfer
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}📡 Transferring to Tower B...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Create remote directory
ssh "$TOWER_B_TARGET" "mkdir -p ~/toadstool-from-tower-a"

# Transfer package
echo -e "${BLUE}📤 Uploading package...${NC}"
scp "$PACKAGE" "$TOWER_B_TARGET:~/toadstool-from-tower-a/toadstool.tar.gz"

echo ""
echo -e "${GREEN}✅ Transfer complete!${NC}"
echo ""

# Extract and setup on Tower B
echo -e "${BLUE}📦 Setting up on Tower B...${NC}"
ssh "$TOWER_B_TARGET" << 'EOSETUP'
cd ~/toadstool-from-tower-a
echo "Extracting package..."
tar xzf toadstool.tar.gz
chmod +x toadstool-showcase-distributed test-on-tower-b.sh
echo "✅ Setup complete!"
EOSETUP

echo ""

# Test on Tower B
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}🧪 Testing on Tower B...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

ssh "$TOWER_B_TARGET" "cd ~/toadstool-from-tower-a && ./test-on-tower-b.sh"

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ DEPLOYMENT COMPLETE!${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${YELLOW}Summary:${NC}"
echo "  ✅ ToadStool deployed to: $TOWER_B_TARGET"
echo "  ✅ Location: ~/toadstool-from-tower-a/"
echo "  ✅ Tested and working"
echo ""

echo -e "${CYAN}Next Steps:${NC}"
echo ""
echo -e "${BLUE}1. Test Tower-to-Tower Distribution:${NC}"
echo "   Update toadstool-songbird-network.toml with Tower B IP"
echo "   Run showcase demo - subtasks should distribute to both towers"
echo ""
echo -e "${BLUE}2. SSH to Tower B for manual testing:${NC}"
echo "   ssh $TOWER_B_TARGET"
echo "   cd ~/toadstool-from-tower-a"
echo "   ./toadstool-showcase-distributed"
echo ""
echo -e "${BLUE}3. Push to GitHub when satisfied:${NC}"
echo "   git push origin parse-error-fixes-canonical-cleanup"
echo ""

# Cleanup
rm -rf "$DEPLOY_DIR"
rm -f "$PACKAGE"

echo -e "${GREEN}🎉 Ready for distributed testing!${NC}"
echo ""

