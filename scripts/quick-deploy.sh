#!/bin/bash
# ToadStool Quick Deployment Script
# Version: 1.0
# Date: November 8, 2025

set -e

echo "🍄 ToadStool Quick Deployment Script"
echo "===================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="${INSTALL_DIR:-/opt/toadstool}"
CONFIG_DIR="${CONFIG_DIR:-/etc/toadstool}"
SERVICE_USER="${SERVICE_USER:-toadstool}"
SYSTEMD_SERVICE="${SYSTEMD_SERVICE:-true}"

echo -e "${BLUE}Configuration:${NC}"
echo "  Install directory: $INSTALL_DIR"
echo "  Config directory: $CONFIG_DIR"
echo "  Service user: $SERVICE_USER"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
   echo -e "${YELLOW}Warning: Not running as root. Some operations may fail.${NC}"
   echo "  Run with: sudo $0"
   echo ""
fi

# Step 1: Build release binaries
echo -e "${BLUE}Step 1: Building release binaries...${NC}"
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Must run from project root directory"
    exit 1
fi

cargo build --release --workspace
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Step 2: Create directories
echo -e "${BLUE}Step 2: Creating directories...${NC}"
mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$CONFIG_DIR"
mkdir -p "/var/lib/toadstool"
mkdir -p "/var/log/toadstool"
echo -e "${GREEN}✓ Directories created${NC}"
echo ""

# Step 3: Copy binaries
echo -e "${BLUE}Step 3: Installing binaries...${NC}"
cp target/release/toadstool "$INSTALL_DIR/bin/" 2>/dev/null || echo "  (toadstool binary not found, skipping)"
cp target/release/toadstool-* "$INSTALL_DIR/bin/" 2>/dev/null || echo "  (no additional binaries)"
chmod +x "$INSTALL_DIR/bin/"*
echo -e "${GREEN}✓ Binaries installed${NC}"
echo ""

# Step 4: Copy configuration
echo -e "${BLUE}Step 4: Installing configuration...${NC}"
if [ -f "toadstool.toml" ]; then
    cp toadstool.toml "$CONFIG_DIR/config.toml"
    echo -e "${GREEN}✓ Configuration installed${NC}"
else
    # Create default config
    cat > "$CONFIG_DIR/config.toml" <<EOF
[service]
name = "toadstool-compute"
version = "1.0.0"
environment = "production"

[network]
bind_address = "0.0.0.0"
api_port = 8084
metrics_port = 9090

[runtime]
native_enabled = true
container_enabled = true
wasm_enabled = true

[monitoring]
metrics_enabled = true
tracing_enabled = true
EOF
    echo -e "${GREEN}✓ Default configuration created${NC}"
fi
echo ""

# Step 5: Create service user
echo -e "${BLUE}Step 5: Creating service user...${NC}"
if id "$SERVICE_USER" &>/dev/null; then
    echo "  User $SERVICE_USER already exists"
else
    useradd -r -s /bin/false "$SERVICE_USER" || echo "  (user creation skipped)"
fi
echo -e "${GREEN}✓ Service user ready${NC}"
echo ""

# Step 6: Set permissions
echo -e "${BLUE}Step 6: Setting permissions...${NC}"
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR" || echo "  (permission setting skipped)"
chown -R "$SERVICE_USER:$SERVICE_USER" "/var/lib/toadstool" || echo "  (permission setting skipped)"
chown -R "$SERVICE_USER:$SERVICE_USER" "/var/log/toadstool" || echo "  (permission setting skipped)"
chmod 755 "$INSTALL_DIR/bin/"*
echo -e "${GREEN}✓ Permissions set${NC}"
echo ""

# Step 7: Create systemd service
if [ "$SYSTEMD_SERVICE" = "true" ]; then
    echo -e "${BLUE}Step 7: Creating systemd service...${NC}"
    cat > /etc/systemd/system/toadstool.service <<EOF
[Unit]
Description=ToadStool Universal Compute Platform
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/toadstool --config $CONFIG_DIR/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=toadstool

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/toadstool

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    echo -e "${GREEN}✓ Systemd service created${NC}"
    echo ""
else
    echo -e "${BLUE}Step 7: Skipping systemd service creation${NC}"
    echo ""
fi

# Step 8: Summary
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}🎉 Deployment Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Installation Summary:"
echo "  Binaries: $INSTALL_DIR/bin/"
echo "  Config: $CONFIG_DIR/config.toml"
echo "  Data: /var/lib/toadstool/"
echo "  Logs: /var/log/toadstool/"
echo ""

if [ "$SYSTEMD_SERVICE" = "true" ]; then
    echo "To start ToadStool:"
    echo "  sudo systemctl enable toadstool"
    echo "  sudo systemctl start toadstool"
    echo ""
    echo "To check status:"
    echo "  sudo systemctl status toadstool"
    echo "  sudo journalctl -u toadstool -f"
else
    echo "To start ToadStool:"
    echo "  $INSTALL_DIR/bin/toadstool --config $CONFIG_DIR/config.toml"
fi

echo ""
echo "Health check:"
echo "  curl http://localhost:8084/health"
echo ""
echo "Metrics:"
echo "  curl http://localhost:9090/metrics"
echo ""
echo -e "${GREEN}Ready for production! 🚀${NC}"

