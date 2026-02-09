#!/bin/bash
# Enable Akida NPU devices for userspace access
# Run with: sudo ./enable-akida.sh

set -e

echo "🧠 Enabling Akida NPU Devices..."
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Please run with sudo: sudo $0"
    exit 1
fi

# Find Akida devices
DEVICES=$(lspci -d 1e7c:bca1 | awk '{print $1}')

if [ -z "$DEVICES" ]; then
    echo "❌ No Akida devices found in lspci"
    echo "   Expected vendor:device = 1e7c:bca1"
    exit 1
fi

echo "✅ Found Akida devices:"
for DEV in $DEVICES; do
    echo "   - $DEV"
done
echo

# Enable each device
for DEV in $DEVICES; do
    PCIE_ADDR="0000:$DEV"
    SYSFS_PATH="/sys/bus/pci/devices/$PCIE_ADDR"
    
    echo "Enabling $PCIE_ADDR..."
    
    # Enable device in PCIe config space
    echo 1 > "$SYSFS_PATH/enable"
    
    # Set permissions on resource files
    chmod 666 "$SYSFS_PATH/resource"*
    chmod 666 "$SYSFS_PATH/enable"
    chmod 666 "$SYSFS_PATH/config"
    
    # Verify enabled
    ENABLED=$(cat "$SYSFS_PATH/enable")
    if [ "$ENABLED" = "1" ]; then
        echo "✅ $PCIE_ADDR enabled"
    else
        echo "❌ $PCIE_ADDR failed to enable"
        exit 1
    fi
done

echo
echo "🎉 All Akida devices enabled!"
echo
echo "📊 Device Status:"
lspci -vv -d 1e7c:bca1 | grep -E "(^[0-9a-f]|Region|Control|Status)" | head -20

echo
echo "💡 Next Steps:"
echo "   1. Verify BARs are active (Region lines above should show [size=4M])"
echo "   2. Test memory mapping: cargo run --example test_akida_mmap"
echo "   3. Implement register protocol based on SDK analysis"
