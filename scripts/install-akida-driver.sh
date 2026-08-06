#!/usr/bin/env bash
# Akida NPU Driver Installation Script
# Run once with sudo, then driver persists across boots

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="/opt/toadstool"
SYSTEMD_DIR="/etc/systemd/system"
UDEV_DIR="/etc/udev/rules.d"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (sudo)"
        exit 1
    fi
}

install_binary() {
    log_info "Installing akida-setup binary..."
    
    # Prefer plasmidBin depot binary, fall back to local build for development
    local PLASMIDBIN_PATH="/opt/toadstool/bin/akida-setup"
    if [[ -f "$PLASMIDBIN_PATH" ]]; then
        log_info "Using plasmidBin depot binary"
        mkdir -p "$INSTALL_DIR/bin"
        cp "$PLASMIDBIN_PATH" "$INSTALL_DIR/bin/"
    else
        log_info "plasmidBin binary not found, building locally (development mode)..."
        # NOTE: akida-setup is excluded from default workspace (C5: requires rustChip on biomeGate).
        # On biomeGate, uncomment neuromorphic crates in root Cargo.toml first.
        cd "$SCRIPT_DIR/.." && cargo build --release -p akida-setup
        mkdir -p "$INSTALL_DIR/bin"
        cp "$SCRIPT_DIR/../target/release/akida-setup" "$INSTALL_DIR/bin/"
    fi
    chmod 755 "$INSTALL_DIR/bin/akida-setup"
    
    log_info "Binary installed to $INSTALL_DIR/bin/akida-setup"
}

install_udev_rules() {
    log_info "Installing udev rules..."
    
    cat > "$UDEV_DIR/99-akida.rules" << 'EOF'
# Akida PCIe device permissions
# Auto-loaded on boot, no sudo required

KERNEL=="akida[0-9]*", MODE="0666", TAG+="uaccess"
KERNEL=="akd1500_[0-9]*", MODE="0666", TAG+="uaccess"

# PCIe resource access for userspace drivers
SUBSYSTEM=="pci", ATTR{vendor}=="0x1e7c", ATTR{device}=="0xbca1", \
  RUN+="/bin/chmod 666 $sys$devpath/resource*", \
  RUN+="/bin/chmod 666 $sys$devpath/enable"

# AKD1500 variant
SUBSYSTEM=="pci", ATTR{vendor}=="0x1e7c", ATTR{device}=="0xbca2", \
  RUN+="/bin/chmod 666 $sys$devpath/resource*", \
  RUN+="/bin/chmod 666 $sys$devpath/enable"
EOF

    chmod 644 "$UDEV_DIR/99-akida.rules"
    udevadm control --reload-rules
    udevadm trigger
    
    log_info "Udev rules installed and reloaded"
}

install_systemd_service() {
    log_info "Installing systemd service..."
    
    cat > "$SYSTEMD_DIR/akida-driver.service" << EOF
[Unit]
Description=Akida NPU Driver Loader
After=multi-user.target
Before=graphical.target
# Only run on systems with BrainChip PCIe hardware (vendor 0x1e7c)
ConditionPathIsDirectory=/sys/bus/pci

[Service]
Type=oneshot
ExecStart=$INSTALL_DIR/bin/akida-setup
RemainAfterExit=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    chmod 644 "$SYSTEMD_DIR/akida-driver.service"
    systemctl daemon-reload
    systemctl enable akida-driver.service
    
    log_info "Systemd service installed and enabled"
}

install_kernel_module() {
    log_info "Checking for kernel module..."
    
    # Look for akida-pcie module
    if [[ -f "/lib/modules/$(uname -r)/extra/akida-pcie.ko" ]] || \
       [[ -f "/lib/modules/$(uname -r)/kernel/drivers/misc/akida-pcie.ko" ]]; then
        log_info "Kernel module found, loading..."
        modprobe akida-pcie || log_warn "Failed to load kernel module (may not be needed)"
    else
        log_warn "Kernel module not found - will use userspace driver"
        log_info "For kernel driver support, install akida-pcie.ko to /lib/modules/$(uname -r)/extra/"
    fi
}

verify_installation() {
    log_info "Verifying installation..."
    
    # Check for Akida devices
    if lspci -d 1e7c: &> /dev/null; then
        log_info "Akida PCIe device(s) detected:"
        lspci -d 1e7c:
    else
        log_warn "No Akida PCIe devices detected"
    fi
    
    # Check if kernel driver loaded
    if lsmod | grep -q akida; then
        log_info "Kernel driver loaded:"
        lsmod | grep akida
        
        if ls /dev/akida* &> /dev/null; then
            log_info "Device nodes created:"
            ls -l /dev/akida*
        fi
    else
        log_info "Kernel driver not loaded - userspace driver will be used"
    fi
    
    # Check PCIe resources
    if ls /sys/bus/pci/devices/*/resource0 2>/dev/null | grep -q .; then
        log_info "PCIe resources accessible for userspace driver"
    fi
}

main() {
    log_info "=== Akida NPU Driver Installer ==="
    log_info "This installs the driver to run automatically on boot"
    log_info ""
    
    check_root
    install_binary
    install_udev_rules
    install_systemd_service
    install_kernel_module
    verify_installation
    
    log_info ""
    log_info "=== Installation Complete ==="
    log_info ""
    log_info "The Akida driver will now load automatically on boot."
    log_info "No sudo/pkexec required on this or other systems."
    log_info ""
    log_info "To test immediately (without reboot):"
    log_info "  sudo systemctl start akida-driver"
    log_info ""
    log_info "To check status:"
    log_info "  systemctl status akida-driver"
    log_info "  journalctl -u akida-driver"
    log_info ""
    log_info "Driver mode:"
    if lsmod | grep -q akida; then
        log_info "  ✓ Kernel driver (high performance)"
    else
        log_info "  ✓ Userspace driver (no kernel module needed)"
    fi
}

main "$@"
