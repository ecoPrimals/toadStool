#!/bin/bash
# ToadStool Staging Deployment Script
# Date: November 15, 2025
# Status: Ready for deployment

set -e  # Exit on error

echo "🍄 ToadStool Staging Deployment"
echo "================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Step 1: Verify we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo -e "${RED}❌ Error: Must run from ToadStool root directory${NC}"
    exit 1
fi

echo -e "${GREEN}✅ In ToadStool directory${NC}"

# Step 2: Run final tests
echo ""
echo "Running final test verification..."
if cargo test --workspace --lib --quiet 2>&1 | grep -q "test result: ok"; then
    echo -e "${GREEN}✅ All tests passing${NC}"
else
    echo -e "${RED}❌ Tests failed - aborting deployment${NC}"
    exit 1
fi

# Step 3: Build release binary
echo ""
echo "Building release binary..."
if cargo build --release --quiet; then
    echo -e "${GREEN}✅ Release build successful${NC}"
else
    echo -e "${RED}❌ Build failed - aborting deployment${NC}"
    exit 1
fi

# Step 4: Verify binary exists
if [ ! -f "target/release/toadstool-cli" ]; then
    echo -e "${RED}❌ Binary not found - aborting deployment${NC}"
    exit 1
fi

BINARY_SIZE=$(du -h target/release/toadstool-cli | cut -f1)
echo -e "${GREEN}✅ Binary ready (${BINARY_SIZE})${NC}"

# Step 5: Test binary
echo ""
echo "Testing binary..."
if ./target/release/toadstool-cli --version > /dev/null 2>&1; then
    VERSION=$(./target/release/toadstool-cli --version)
    echo -e "${GREEN}✅ Binary functional: ${VERSION}${NC}"
else
    echo -e "${RED}❌ Binary test failed - aborting deployment${NC}"
    exit 1
fi

# Step 6: Backup existing staging binary (if exists)
echo ""
if [ -f "/usr/local/bin/toadstool-staging" ]; then
    echo "Backing up existing staging binary..."
    BACKUP_NAME="/usr/local/bin/toadstool-staging.backup.$(date +%Y%m%d_%H%M%S)"
    sudo cp /usr/local/bin/toadstool-staging "$BACKUP_NAME"
    echo -e "${GREEN}✅ Backup created: ${BACKUP_NAME}${NC}"
fi

# Step 7: Deploy to staging
echo ""
echo "Deploying to staging..."
if sudo cp target/release/toadstool-cli /usr/local/bin/toadstool-staging; then
    echo -e "${GREEN}✅ Binary deployed to /usr/local/bin/toadstool-staging${NC}"
else
    echo -e "${RED}❌ Deployment failed${NC}"
    exit 1
fi

# Step 8: Set permissions
sudo chmod +x /usr/local/bin/toadstool-staging
echo -e "${GREEN}✅ Permissions set${NC}"

# Step 9: Verify staging deployment
echo ""
echo "Verifying staging deployment..."
if /usr/local/bin/toadstool-staging --version > /dev/null 2>&1; then
    STAGING_VERSION=$(/usr/local/bin/toadstool-staging --version)
    echo -e "${GREEN}✅ Staging binary verified: ${STAGING_VERSION}${NC}"
else
    echo -e "${RED}❌ Staging binary verification failed${NC}"
    exit 1
fi

# Step 10: Test capabilities
echo ""
echo "Testing capabilities..."
if /usr/local/bin/toadstool-staging capabilities > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Capabilities working${NC}"
else
    echo -e "${YELLOW}⚠️  Capabilities command not available (may be normal)${NC}"
fi

# Success!
echo ""
echo "================================"
echo -e "${GREEN}🎉 DEPLOYMENT COMPLETE${NC}"
echo "================================"
echo ""
echo "Staging binary: /usr/local/bin/toadstool-staging"
echo "Version: ${STAGING_VERSION}"
echo "Size: ${BINARY_SIZE}"
echo ""
echo "Next steps:"
echo "1. Monitor logs for 48-72 hours"
echo "2. Test critical workflows"
echo "3. Check metrics and performance"
echo "4. Review error rates"
echo ""
echo "Monitoring commands:"
echo "  toadstool-staging --version"
echo "  toadstool-staging capabilities"
echo "  journalctl -u toadstool-staging -f  # if running as service"
echo ""
echo -e "${GREEN}✅ Ready for production in 1-2 weeks if staging is stable${NC}"
echo ""

