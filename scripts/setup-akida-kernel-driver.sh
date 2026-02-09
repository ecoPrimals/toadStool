#!/bin/bash
# Complete Akida kernel driver installation and setup
# Run with: sudo ./setup-akida-kernel-driver.sh

set -e

echo "🧠 Akida Kernel Driver - Complete Setup"
echo "========================================"
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Please run with sudo: sudo $0"
    exit 1
fi

DRIVER_SOURCE="$HOME/Development/ecoPrimals/akida_dw_edma"
DRIVER_MODULE="$DRIVER_SOURCE/akida-pcie.ko"
UDEV_RULES="$DRIVER_SOURCE/99-akida-pcie.rules"

# Verify driver exists
if [ ! -f "$DRIVER_MODULE" ]; then
    echo "❌ Driver module not found at: $DRIVER_MODULE"
    echo "   Expected location: ~/Development/ecoPrimals/akida_dw_edma/akida-pcie.ko"
    exit 1
fi

echo "✅ Found driver module: $DRIVER_MODULE"
echo

# Check driver compatibility
echo "📋 Driver Information:"
modinfo "$DRIVER_MODULE" | grep -E "(filename|vermagic|alias|description)"
echo

KERNEL_VERSION=$(uname -r)
DRIVER_VERSION=$(modinfo "$DRIVER_MODULE" | grep vermagic | awk '{print $2}')

if [ "$KERNEL_VERSION" != "$DRIVER_VERSION" ]; then
    echo "⚠️  WARNING: Kernel version mismatch!"
    echo "   Current kernel: $KERNEL_VERSION"
    echo "   Driver built for: $DRIVER_VERSION"
    echo
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborting. Rebuild driver with: cd $DRIVER_SOURCE && make"
        exit 1
    fi
fi

# Step 1: Install udev rules
echo "📝 Step 1: Installing udev rules..."
if [ -f "$UDEV_RULES" ]; then
    cp "$UDEV_RULES" /etc/udev/rules.d/
    echo "✅ Installed udev rules: /etc/udev/rules.d/99-akida-pcie.rules"
    udevadm control --reload-rules
    echo "✅ Reloaded udev rules"
else
    echo "⚠️  udev rules not found, creating default..."
    cat > /etc/udev/rules.d/99-akida-pcie.rules <<EOF
# Akida PCIe device permissions
KERNEL=="akida[0-9]*", MODE="0666"
KERNEL=="akd1500_[0-9]*", MODE="0666"
EOF
    echo "✅ Created /etc/udev/rules.d/99-akida-pcie.rules"
    udevadm control --reload-rules
fi
echo

# Step 2: Enable PCIe devices
echo "🔌 Step 2: Enabling PCIe devices..."
DEVICES=$(lspci -d 1e7c:bca1 | awk '{print $1}')

if [ -z "$DEVICES" ]; then
    echo "❌ No Akida devices found in lspci"
    echo "   Expected vendor:device = 1e7c:bca1"
    exit 1
fi

for DEV in $DEVICES; do
    PCIE_ADDR="0000:$DEV"
    SYSFS_PATH="/sys/bus/pci/devices/$PCIE_ADDR"
    
    echo "  Enabling $PCIE_ADDR..."
    
    # Enable device
    echo 1 > "$SYSFS_PATH/enable"
    
    # Verify
    ENABLED=$(cat "$SYSFS_PATH/enable")
    if [ "$ENABLED" = "1" ]; then
        echo "  ✅ $PCIE_ADDR enabled"
    else
        echo "  ❌ Failed to enable $PCIE_ADDR"
        exit 1
    fi
done
echo

# Step 3: Load kernel module
echo "🔧 Step 3: Loading kernel module..."

# Unload if already loaded (in case of update)
if lsmod | grep -q akida_pcie; then
    echo "  Unloading existing module..."
    rmmod akida_pcie || true
fi

# Load the module
echo "  Loading akida-pcie.ko..."
insmod "$DRIVER_MODULE"

# Verify loaded
if lsmod | grep -q akida_pcie; then
    echo "✅ Module loaded successfully"
else
    echo "❌ Failed to load module"
    exit 1
fi
echo

# Step 4: Verify device nodes
echo "🔍 Step 4: Verifying device nodes..."
sleep 1  # Give udev time to create nodes

if ls /dev/akida* >/dev/null 2>&1; then
    echo "✅ Device nodes created:"
    ls -l /dev/akida* | sed 's/^/  /'
else
    echo "❌ No /dev/akida* nodes found"
    echo
    echo "Troubleshooting:"
    echo "  1. Check dmesg: sudo dmesg | tail -20"
    echo "  2. Check udev: udevadm info /sys/class/misc/akida0"
    echo "  3. Manual trigger: udevadm trigger"
    exit 1
fi
echo

# Step 5: Test device access
echo "✅ Step 5: Testing device access..."
FIRST_DEV=$(ls /dev/akida* | head -1)
if [ -r "$FIRST_DEV" ] && [ -w "$FIRST_DEV" ]; then
    echo "✅ Device $FIRST_DEV is readable and writable"
else
    echo "⚠️  Device $FIRST_DEV may need permission fix"
    chmod 666 /dev/akida*
    echo "  Fixed permissions"
fi
echo

# Summary
echo "═══════════════════════════════════════════════════════════"
echo "✅ Akida Kernel Driver Setup Complete!"
echo "═══════════════════════════════════════════════════════════"
echo
echo "📊 Status:"
lsmod | grep akida_pcie
echo
echo "🔌 Devices:"
lspci -d 1e7c:bca1 | sed 's/^/  /'
echo
echo "📂 Device Nodes:"
ls -l /dev/akida* | sed 's/^/  /'
echo
echo "═══════════════════════════════════════════════════════════"
echo
echo "🎯 Next Steps:"
echo "  1. Test detection: cd showcase/neuromorphic/01-akida-detection"
echo "                     cargo run --example detect_akida_real"
echo
echo "  2. Run validation: cd showcase/barracuda-validation"
echo "                     cargo run --bin cross_platform_homomorphic --release"
echo
echo "  3. Check dmesg: sudo dmesg | grep -i akida"
echo
echo "💡 To make module load at boot:"
echo "   echo 'akida_pcie' | sudo tee /etc/modules-load.d/akida.conf"
echo
