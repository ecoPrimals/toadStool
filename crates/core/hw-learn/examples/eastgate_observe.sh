#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# eastgate GPU observation script
# Run on machines with multiple GPUs to capture init traces for hwLearn.
#
# Prerequisites:
#   - Linux kernel with mmiotrace support
#   - Root access for trace facility
#   - cargo available to build hw-learn
#
# Usage:
#   chmod +x eastgate_observe.sh
#   sudo ./eastgate_observe.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="${HOME}/.local/share/hw-learn/observations"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "=== hwLearn eastgate observation — ${TIMESTAMP} ==="
echo ""

mkdir -p "$OUT_DIR"

# Step 1: Fleet discovery (no root needed)
echo "--- Step 1: Fleet discovery ---"
cd "$CRATE_DIR/../../../"
cargo run -p hw-learn --example fleet_observe -- --json > "${OUT_DIR}/fleet_${TIMESTAMP}.json" 2>&1
echo "Fleet report: ${OUT_DIR}/fleet_${TIMESTAMP}.json"
echo ""

# Step 2: GSP RPC trace from dmesg
echo "--- Step 2: GSP RPC trace ---"
dmesg | grep -i "gsp\|nouveau\|nvidia\|drm" > "${OUT_DIR}/dmesg_gpu_${TIMESTAMP}.log" 2>/dev/null || true
echo "dmesg GPU log: ${OUT_DIR}/dmesg_gpu_${TIMESTAMP}.log"
echo ""

# Step 3: DRM ioctl trace (if strace available)
if command -v strace &> /dev/null; then
    echo "--- Step 3: DRM ioctl trace ---"
    echo "Tracing vulkaninfo for DRM ioctls (5s timeout)..."
    timeout 5 strace -e trace=ioctl -y vulkaninfo > /dev/null 2> "${OUT_DIR}/ioctl_trace_${TIMESTAMP}.log" || true
    echo "ioctl trace: ${OUT_DIR}/ioctl_trace_${TIMESTAMP}.log"
    echo ""
fi

# Step 4: mmiotrace (requires root)
if [ "$(id -u)" -eq 0 ]; then
    echo "--- Step 4: mmiotrace capture ---"
    echo "WARNING: mmiotrace is intrusive and can slow the system."
    echo "Capturing for 10 seconds..."

    echo mmiotrace > /sys/kernel/tracing/current_tracer 2>/dev/null || {
        echo "mmiotrace not available in this kernel config"
        echo "Skip to results"
        echo nop > /sys/kernel/tracing/current_tracer 2>/dev/null || true
    }

    sleep 10
    cat /sys/kernel/tracing/trace > "${OUT_DIR}/mmiotrace_${TIMESTAMP}.log" 2>/dev/null || true
    echo nop > /sys/kernel/tracing/current_tracer

    echo "mmiotrace: ${OUT_DIR}/mmiotrace_${TIMESTAMP}.log"
    echo ""
else
    echo "--- Step 4: mmiotrace (skipped — not root) ---"
    echo "Run as root to capture mmiotrace."
    echo ""
fi

# Summary
echo "=== Observation complete ==="
echo "Output directory: ${OUT_DIR}"
echo ""
echo "Next steps:"
echo "  1. Review fleet report for teacher/student pairs"
echo "  2. If trace files are available, parse them:"
echo "     cargo run -p hw-learn --example fleet_observe -- --trace ${OUT_DIR}/mmiotrace_${TIMESTAMP}.log"
echo "  3. Copy ${OUT_DIR}/ to hotSpring rig for analysis"
echo ""
ls -la "${OUT_DIR}/"*"${TIMESTAMP}"* 2>/dev/null || echo "(no files with current timestamp)"
