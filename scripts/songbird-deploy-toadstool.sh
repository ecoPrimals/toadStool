#!/bin/bash
# Deploy ToadStool binaries via Songbird to remote towers
# Usage: ./songbird-deploy-toadstool.sh <target-tower>

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

TARGET_TOWER="${1:-tower-b}"
SONGBIRD_ENDPOINT="${SONGBIRD_ENDPOINT:-http://localhost:8080}"

echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║    🍄 ToadStool Deployment via Songbird                  ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check binaries exist
echo -e "${BLUE}📦 Checking ToadStool binaries...${NC}"
if [ ! -f "target/release/toadstool-cli" ]; then
    echo -e "${RED}❌ toadstool-cli not found. Run: cargo build --release${NC}"
    exit 1
fi

if [ ! -f "target/release/toadstool-showcase-distributed" ]; then
    echo -e "${YELLOW}⚠️  Showcase demo not found (optional)${NC}"
fi

echo -e "${GREEN}✅ Binaries found:${NC}"
ls -lh target/release/toadstool-cli target/release/toadstool-showcase-distributed 2>/dev/null | awk '{print "   "$9 ": " $5}'
echo ""

# Create deployment package
echo -e "${BLUE}📦 Creating deployment package...${NC}"
DEPLOY_DIR="/tmp/toadstool-deploy-$$"
mkdir -p "$DEPLOY_DIR"

# Copy binaries
cp target/release/toadstool-cli "$DEPLOY_DIR/"
[ -f target/release/toadstool-showcase-distributed ] && cp target/release/toadstool-showcase-distributed "$DEPLOY_DIR/"

# Copy essential configs
cp toadstool.toml "$DEPLOY_DIR/" 2>/dev/null || echo "# Default config" > "$DEPLOY_DIR/toadstool.toml"
cp toadstool-songbird-network.toml "$DEPLOY_DIR/" 2>/dev/null || true

# Copy showcase if available
if [ -d "showcase" ]; then
    echo -e "${BLUE}📁 Packaging showcase...${NC}"
    tar czf "$DEPLOY_DIR/showcase.tar.gz" showcase/ 2>/dev/null || true
fi

# Create deployment script for remote tower
cat > "$DEPLOY_DIR/deploy-on-tower.sh" << 'EODEPLOY'
#!/bin/bash
# Auto-generated deployment script for remote tower

set -e

echo "🍄 ToadStool Remote Deployment"
echo ""

# Create installation directory
INSTALL_DIR="${HOME}/toadstool"
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Extract and install
echo "📦 Installing binaries..."
chmod +x toadstool-cli toadstool-showcase-distributed 2>/dev/null || true
mkdir -p bin
mv toadstool-cli toadstool-showcase-distributed bin/ 2>/dev/null || true

# Extract showcase if present
if [ -f showcase.tar.gz ]; then
    echo "📦 Extracting showcase..."
    tar xzf showcase.tar.gz
    rm showcase.tar.gz
fi

# Setup PATH
echo ""
echo "✅ ToadStool installed to: $INSTALL_DIR"
echo ""
echo "Add to your PATH:"
echo "  export PATH=\"$INSTALL_DIR/bin:\$PATH\""
echo ""
echo "Test installation:"
echo "  toadstool-cli --version"
echo "  cd $INSTALL_DIR/showcase && ./showcase.sh"
echo ""
EODEPLOY

chmod +x "$DEPLOY_DIR/deploy-on-tower.sh"

echo -e "${GREEN}✅ Package created at: $DEPLOY_DIR${NC}"
du -sh "$DEPLOY_DIR"
echo ""

# Create tarball
echo -e "${BLUE}📦 Creating transfer package...${NC}"
PACKAGE_FILE="/tmp/toadstool-deploy-$TARGET_TOWER.tar.gz"
tar czf "$PACKAGE_FILE" -C "$DEPLOY_DIR" .

PACKAGE_SIZE=$(du -h "$PACKAGE_FILE" | cut -f1)
echo -e "${GREEN}✅ Package ready: $PACKAGE_FILE ($PACKAGE_SIZE)${NC}"
echo ""

# Transfer via Songbird
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}🐦 Transferring via Songbird to $TARGET_TOWER${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Encode package as base64 for transport
echo -e "${BLUE}📡 Encoding package for transport...${NC}"
PACKAGE_B64=$(base64 -w 0 "$PACKAGE_FILE")
PACKAGE_B64_SIZE=$(echo -n "$PACKAGE_B64" | wc -c)
echo -e "${GREEN}✅ Encoded size: $PACKAGE_B64_SIZE bytes${NC}"
echo ""

# Create Songbird job payload
cat > "/tmp/songbird-deploy-job-$$.json" << EOJSON
{
  "job_type": "deploy_toadstool",
  "target_node": "$TARGET_TOWER",
  "priority": "high",
  "payload": {
    "package_type": "toadstool_binary",
    "package_data": "$PACKAGE_B64",
    "deployment_script": "deploy-on-tower.sh",
    "auto_extract": true,
    "auto_install": true
  },
  "execution": {
    "command": "bash",
    "args": ["deploy-on-tower.sh"],
    "working_dir": "/tmp/toadstool-deployment"
  }
}
EOJSON

echo -e "${BLUE}🚀 Submitting deployment job to Songbird...${NC}"
echo -e "${YELLOW}Endpoint: $SONGBIRD_ENDPOINT${NC}"
echo ""

# Submit via Songbird API
if command -v curl &> /dev/null; then
    RESPONSE=$(curl -s -X POST "$SONGBIRD_ENDPOINT/api/v1/jobs/submit" \
        -H "Content-Type: application/json" \
        -d @"/tmp/songbird-deploy-job-$$.json" 2>&1)
    
    if echo "$RESPONSE" | grep -q "job_id"; then
        JOB_ID=$(echo "$RESPONSE" | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)
        echo -e "${GREEN}✅ Job submitted successfully!${NC}"
        echo -e "${GREEN}   Job ID: $JOB_ID${NC}"
        echo ""
        
        # Monitor job status
        echo -e "${BLUE}📊 Monitoring deployment...${NC}"
        for i in {1..30}; do
            STATUS=$(curl -s "$SONGBIRD_ENDPOINT/api/v1/jobs/$JOB_ID/status" 2>&1 || echo "pending")
            echo -e "   Status check $i/30: $STATUS"
            
            if echo "$STATUS" | grep -q "completed"; then
                echo -e "${GREEN}✅ Deployment completed successfully!${NC}"
                break
            elif echo "$STATUS" | grep -q "failed"; then
                echo -e "${RED}❌ Deployment failed${NC}"
                break
            fi
            
            sleep 2
        done
    else
        echo -e "${YELLOW}⚠️  Songbird API not available or response unexpected${NC}"
        echo -e "${YELLOW}Response: $RESPONSE${NC}"
        echo ""
        echo -e "${BLUE}📦 Package available for manual transfer:${NC}"
        echo -e "   File: $PACKAGE_FILE"
        echo -e "   Size: $PACKAGE_SIZE"
        echo ""
        echo -e "${CYAN}Manual transfer via Songbird:${NC}"
        echo -e "   1. Copy package to target tower:"
        echo -e "      ${YELLOW}scp $PACKAGE_FILE $TARGET_TOWER:/tmp/${NC}"
        echo ""
        echo -e "   2. On $TARGET_TOWER, extract and run:"
        echo -e "      ${YELLOW}cd /tmp && tar xzf toadstool-deploy-$TARGET_TOWER.tar.gz${NC}"
        echo -e "      ${YELLOW}bash deploy-on-tower.sh${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  curl not available${NC}"
    echo -e "${BLUE}📦 Package ready for manual transfer:${NC}"
    echo -e "   $PACKAGE_FILE ($PACKAGE_SIZE)"
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Cleanup
echo -e "${BLUE}🧹 Cleaning up temporary files...${NC}"
rm -rf "$DEPLOY_DIR"
rm -f "/tmp/songbird-deploy-job-$$.json"

echo ""
echo -e "${GREEN}✅ Deployment process complete!${NC}"
echo ""
echo -e "${CYAN}Next steps:${NC}"
echo "  1. Verify deployment on $TARGET_TOWER"
echo "  2. Test ToadStool connection between towers"
echo "  3. Run distributed showcase demo"
echo ""
echo -e "${YELLOW}Test distributed execution:${NC}"
echo "  ${CYAN}# On Tower A (this tower):${NC}"
echo "  ./showcase/showcase.sh  # Select option 2"
echo ""
echo "  ${CYAN}# Should see subtasks distribute to $TARGET_TOWER${NC}"
echo ""

