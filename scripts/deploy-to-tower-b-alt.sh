#!/bin/bash
# Alternative deployment to Tower B - works without SSH
# Uses manual transfer instructions or Songbird if SSH isn't available

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║    🍄 Deploy ToadStool to Tower B (Alternative)         ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${YELLOW}This method creates a portable package you can transfer manually.${NC}"
echo ""

# Check binaries
if [ ! -f "target/release/toadstool-showcase-distributed" ]; then
    echo -e "${RED}❌ Binary not found!${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

# Create package
echo -e "${BLUE}📦 Creating portable deployment package...${NC}"
DEPLOY_DIR="/tmp/toadstool-portable-$$"
mkdir -p "$DEPLOY_DIR"

# Copy binary
cp target/release/toadstool-showcase-distributed "$DEPLOY_DIR/"

# Copy showcase
if [ -d "showcase" ]; then
    cp -r showcase "$DEPLOY_DIR/"
fi

# Create README for Tower B
cat > "$DEPLOY_DIR/README-TOWER-B.txt" << 'EOREADME'
ToadStool Portable Package for Tower B
=======================================

This package contains:
  - toadstool-showcase-distributed (2.2MB binary)
  - showcase/ directory (demos and workloads)

INSTALLATION ON TOWER B:
========================

1. Extract this package:
   tar xzf toadstool-portable.tar.gz
   cd toadstool-portable/

2. Make binary executable:
   chmod +x toadstool-showcase-distributed

3. Test it works:
   ./toadstool-showcase-distributed

4. Run showcase demos:
   cd showcase/
   ./showcase.sh
   # Select option 2: Distributed Compute Demo

EXPECTED OUTPUT:
===============
You should see:
  ✅ 10 subtasks spawn and execute
  ✅ 100% success rate
  ✅ Execution time < 0.1s
  ✅ Beautiful colored output

NEXT STEPS:
==========
Once working on Tower B, you can test distributed execution:
  1. On Tower A, update toadstool-songbird-network.toml with Tower B IP
  2. Run showcase from Tower A
  3. Subtasks should distribute to Tower B via Songbird!

TROUBLESHOOTING:
===============
If binary doesn't run:
  - Check it's executable: chmod +x toadstool-showcase-distributed
  - Check architecture matches: file toadstool-showcase-distributed
  - Try running with: ./toadstool-showcase-distributed --version

Need help? Check the logs or reach out!

Happy distributed computing! 🍄
EOREADME

# Create quick test script
cat > "$DEPLOY_DIR/test.sh" << 'EOTEST'
#!/bin/bash
echo "🍄 Testing ToadStool on Tower B..."
echo ""
chmod +x toadstool-showcase-distributed
./toadstool-showcase-distributed
echo ""
echo "✅ If you see the demo output above, ToadStool is working!"
echo ""
EOTEST

chmod +x "$DEPLOY_DIR/test.sh"

# Create tarball
PACKAGE="$HOME/toadstool-portable.tar.gz"
tar czf "$PACKAGE" -C "$DEPLOY_DIR" .
PACKAGE_SIZE=$(ls -lh "$PACKAGE" | awk '{print $5}')

echo -e "${GREEN}✅ Package created!${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}📦 Portable Package Ready${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  Location: $PACKAGE"
echo "  Size:     $PACKAGE_SIZE"
echo ""

# Show transfer options
echo -e "${YELLOW}Transfer Options:${NC}"
echo ""

echo -e "${BLUE}Option 1: USB Drive / Physical Media${NC}"
echo "  cp $PACKAGE /media/usb-drive/"
echo "  # Move USB to Tower B, then extract"
echo ""

echo -e "${BLUE}Option 2: SCP (if you know Tower B's IP)${NC}"
echo "  scp $PACKAGE user@<TOWER_B_IP>:/tmp/"
echo "  # On Tower B: cd /tmp && tar xzf toadstool-portable.tar.gz"
echo ""

echo -e "${BLUE}Option 3: HTTP Server${NC}"
echo "  python3 -m http.server 8000"
echo "  # On Tower B: wget http://<TOWER_A_IP>:8000/toadstool-portable.tar.gz"
echo ""

echo -e "${BLUE}Option 4: Shared Network Drive${NC}"
echo "  cp $PACKAGE /mnt/shared/"
echo "  # Access from Tower B"
echo ""

echo -e "${BLUE}Option 5: Via Songbird (if configured for file transfer)${NC}"
echo "  # Use Songbird API to send package"
echo "  # (Requires Songbird file transfer capability)"
echo ""

# Try to determine Tower B IP from Songbird
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}Checking Songbird for Tower B info...${NC}"
echo ""

TOWER_B_INFO=$(curl -s http://localhost:8080/api/v1/nodes 2>/dev/null || echo "")

if [ ! -z "$TOWER_B_INFO" ]; then
    echo "Songbird response:"
    echo "$TOWER_B_INFO" | head -20
    echo ""
    
    # Try to extract Tower B IP if present
    TOWER_B_IP=$(echo "$TOWER_B_INFO" | grep -oP '\d+\.\d+\.\d+\.\d+' | grep -v "127.0.0.1" | head -1)
    
    if [ ! -z "$TOWER_B_IP" ]; then
        echo -e "${GREEN}Found potential Tower B IP: $TOWER_B_IP${NC}"
        echo ""
        echo "Try SCP with this IP:"
        echo -e "  ${CYAN}scp $PACKAGE eastgate@$TOWER_B_IP:/tmp/${NC}"
        echo ""
    fi
else
    echo "Songbird not responding or no nodes registered"
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${YELLOW}ON TOWER B, after transfer:${NC}"
echo ""
echo "  1. Extract package:"
echo "     tar xzf toadstool-portable.tar.gz"
echo ""
echo "  2. Test it:"
echo "     cd toadstool-portable/"
echo "     ./test.sh"
echo ""
echo "  3. If successful, you're ready for distributed testing!"
echo ""

# Cleanup
rm -rf "$DEPLOY_DIR"

echo -e "${GREEN}✅ Package ready for transfer!${NC}"
echo ""

