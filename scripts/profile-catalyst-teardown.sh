#!/usr/bin/env bash
# Profile the nvidia RM teardown during catalyst warm_swap.
#
# Usage: sudo ./profile-catalyst-teardown.sh [BDF]
#   BDF defaults to 0000:02:00.0
#
# This script arms function_graph tracing filtered to nvsov/nvidia PCI
# functions, waits for the catalyst handoff to run, then dumps the trace.

set -euo pipefail

BDF="${1:-0000:02:00.0}"
TRACEFS="/sys/kernel/tracing"
OUTDIR="/tmp/catalyst-profile-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUTDIR"

echo "=== Catalyst Teardown Profiler ==="
echo "BDF: $BDF"
echo "Output: $OUTDIR"
echo ""

# Phase 1: Capture the nvsov module's key function addresses
echo "[1] Checking nvsov module state..."
if grep -q nvsov /proc/modules 2>/dev/null; then
    echo "  nvsov currently loaded — capturing symbol map"
    grep nvsov /proc/modules > "$OUTDIR/nvsov-modules.txt"
    cat /proc/kallsyms | grep nvsov > "$OUTDIR/nvsov-kallsyms.txt" 2>/dev/null || true
else
    echo "  nvsov not loaded (will capture during handoff)"
fi

# Phase 2: Set up function_graph tracing
echo "[2] Configuring ftrace..."
echo nop > "$TRACEFS/current_tracer"
echo 0 > "$TRACEFS/tracing_on"

# Increase buffer
echo 32768 > "$TRACEFS/buffer_size_kb"

# Filter to PCI and device-related functions
cat > "$TRACEFS/set_ftrace_filter" <<'EOF'
pci_device_remove
device_release_driver_internal
driver_detach
__device_release_driver
pci_disable_device
pci_release_regions
pci_release_selected_regions
pcibios_release_device
pci_set_power_state
pci_raw_set_power_state
pci_save_state
pci_restore_state
pci_set_master
pci_clear_master
sysfs_remove_bin_file
pci_iounmap
iounmap
__iounmap
vfio_pci_core_enable
vfio_pci_core_disable
sysfs_kf_bin_mmap
sysfs_kf_bin_read
EOF

echo function_graph > "$TRACEFS/current_tracer"
echo 8 > "$TRACEFS/max_graph_depth"

echo "[3] Tracing armed. Run the catalyst handoff now."
echo "    Press Ctrl-C or wait for nvsov rmmod to stop tracing."
echo ""

# Phase 3: Enable tracing and monitor
echo 1 > "$TRACEFS/tracing_on"

# Monitor for nvsov load/unload events
while true; do
    if grep -q nvsov /proc/modules 2>/dev/null; then
        echo "  [$(date +%H:%M:%S)] nvsov loaded — capturing kallsyms..."
        cat /proc/kallsyms | grep ' nvsov' > "$OUTDIR/nvsov-kallsyms-live.txt" 2>/dev/null || true
        break
    fi
    sleep 1
done

echo "  [$(date +%H:%M:%S)] Waiting for nvsov unbind (monitoring driver symlink)..."
while [ -L "/sys/bus/pci/devices/$BDF/driver" ]; do
    DRIVER=$(basename "$(readlink "/sys/bus/pci/devices/$BDF/driver")" 2>/dev/null || echo "?")
    if [ "$DRIVER" = "nvsov" ]; then
        sleep 0.5
        continue
    fi
    break
done
echo "  [$(date +%H:%M:%S)] Driver symlink cleared."

echo "  [$(date +%H:%M:%S)] Monitoring PCI device state during RM teardown..."
for i in $(seq 1 120); do
    DRIVER=$(basename "$(readlink "/sys/bus/pci/devices/$BDF/driver")" 2>/dev/null || echo "none")
    ENABLE=$(cat "/sys/bus/pci/devices/$BDF/enable" 2>/dev/null || echo "?")
    RESOURCE0_SIZE=$(stat -c%s "/sys/bus/pci/devices/$BDF/resource0" 2>/dev/null || echo "?")
    echo "  [$(date +%H:%M:%S)] +${i}s driver=$DRIVER enable=$ENABLE resource0_size=$RESOURCE0_SIZE" | tee -a "$OUTDIR/poll-log.txt"

    # Try a quick resource0 open test (non-blocking)
    timeout 1 dd if="/sys/bus/pci/devices/$BDF/resource0" of=/dev/null bs=4 count=1 2>/dev/null && \
        echo "  [$(date +%H:%M:%S)] resource0 readable!" | tee -a "$OUTDIR/poll-log.txt" && break

    sleep 5
done

# Phase 4: Dump the trace
echo "[4] Stopping trace and saving..."
echo 0 > "$TRACEFS/tracing_on"
cat "$TRACEFS/trace" > "$OUTDIR/ftrace.txt"
echo nop > "$TRACEFS/current_tracer"
echo "" > "$TRACEFS/set_ftrace_filter"

# Phase 5: Also capture device state
echo "[5] Capturing final device state..."
lspci -s "$BDF" -vvv > "$OUTDIR/lspci-final.txt" 2>&1
cat /proc/iomem | grep -A2 "$BDF" > "$OUTDIR/iomem.txt" 2>/dev/null || true

echo ""
echo "=== Profile complete ==="
echo "Artifacts in: $OUTDIR"
ls -la "$OUTDIR/"
