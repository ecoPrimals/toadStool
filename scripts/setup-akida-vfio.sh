#!/bin/bash
# Setup Akida AKD1000 for VFIO access
#
# Run with: sudo ./scripts/setup-akida-vfio.sh
#
# Requirements:
# - IOMMU enabled in BIOS (AMD: usually default on EPYC)
# - vfio-pci module available

set -e

VENDOR_ID="1e7c"
DEVICE_ID="bca1"
NPU1_ADDR="0000:a1:00.0"
NPU2_ADDR="0000:e2:00.0"

echo "=== Akida AKD1000 VFIO Setup ==="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "ERROR: Must run as root (sudo)"
    exit 1
fi

# Check IOMMU
echo "1. Checking IOMMU..."
if [ -d "/sys/kernel/iommu_groups" ]; then
    GROUPS=$(ls /sys/kernel/iommu_groups | wc -l)
    echo "   ✓ IOMMU enabled ($GROUPS groups)"
else
    echo "   ✗ IOMMU not enabled"
    echo "   Add 'amd_iommu=on' to kernel cmdline and reboot"
    exit 1
fi

# Load vfio-pci module
echo ""
echo "2. Loading VFIO modules..."
modprobe vfio
modprobe vfio_pci
modprobe vfio_iommu_type1
echo "   ✓ VFIO modules loaded"

# Function to bind device to vfio-pci
bind_to_vfio() {
    local ADDR=$1
    local NAME=$2
    
    echo ""
    echo "3. Binding $NAME ($ADDR) to vfio-pci..."
    
    # Check if device exists
    if [ ! -d "/sys/bus/pci/devices/$ADDR" ]; then
        echo "   ✗ Device $ADDR not found"
        return 1
    fi
    
    # Get IOMMU group
    IOMMU_GROUP=$(readlink /sys/bus/pci/devices/$ADDR/iommu_group | xargs basename)
    echo "   IOMMU group: $IOMMU_GROUP"
    
    # Unbind from current driver (if any)
    if [ -L "/sys/bus/pci/devices/$ADDR/driver" ]; then
        CURRENT_DRIVER=$(readlink /sys/bus/pci/devices/$ADDR/driver | xargs basename)
        echo "   Unbinding from $CURRENT_DRIVER..."
        echo "$ADDR" > /sys/bus/pci/devices/$ADDR/driver/unbind 2>/dev/null || true
    fi
    
    # Enable device
    echo "   Enabling device..."
    echo 1 > /sys/bus/pci/devices/$ADDR/enable 2>/dev/null || true
    
    # Bind to vfio-pci
    echo "   Binding to vfio-pci..."
    echo "$VENDOR_ID $DEVICE_ID" > /sys/bus/pci/drivers/vfio-pci/new_id 2>/dev/null || true
    echo "$ADDR" > /sys/bus/pci/drivers/vfio-pci/bind 2>/dev/null || true
    
    # Verify
    if [ -L "/sys/bus/pci/devices/$ADDR/driver" ]; then
        NEW_DRIVER=$(readlink /sys/bus/pci/devices/$ADDR/driver | xargs basename)
        if [ "$NEW_DRIVER" = "vfio-pci" ]; then
            echo "   ✓ Bound to vfio-pci"
            
            # Set permissions on VFIO group
            if [ -e "/dev/vfio/$IOMMU_GROUP" ]; then
                chmod 666 /dev/vfio/$IOMMU_GROUP
                echo "   ✓ /dev/vfio/$IOMMU_GROUP accessible"
            fi
            return 0
        fi
    fi
    
    echo "   ✗ Failed to bind"
    return 1
}

# Bind both NPUs
bind_to_vfio "$NPU1_ADDR" "Akida NPU #1" || true
bind_to_vfio "$NPU2_ADDR" "Akida NPU #2" || true

# Summary
echo ""
echo "=== Summary ==="
echo ""
echo "VFIO groups:"
ls -la /dev/vfio/ 2>/dev/null || echo "  No VFIO groups"
echo ""
echo "NPU status:"
for ADDR in $NPU1_ADDR $NPU2_ADDR; do
    if [ -L "/sys/bus/pci/devices/$ADDR/driver" ]; then
        DRIVER=$(readlink /sys/bus/pci/devices/$ADDR/driver | xargs basename)
        echo "  $ADDR: $DRIVER"
    else
        echo "  $ADDR: no driver"
    fi
done

echo ""
echo "To test:"
echo "  cargo test -p akida-driver test_vfio_backend_init -- --nocapture"
echo ""
