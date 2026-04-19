#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Capture MMIO trace during NVIDIA GPU compute initialization.
#
# Prerequisites:
#   - NVIDIA proprietary driver loaded (nvidia, nvidia-uvm)
#   - Root access (mmiotrace requires debugfs)
#   - CUDA toolkit installed (for cuda_memcpy test binary)
#
# Usage:
#   sudo ./capture_mmiotrace.sh [output_dir]
#
# Output:
#   - {output_dir}/mmiotrace_baseline.txt  (driver load, no compute)
#   - {output_dir}/mmiotrace_compute.txt   (driver load + compute init)
#   - {output_dir}/ioctl_trace.txt         (strace of CUDA workload)
#
# The hw-learn distiller can then diff these to extract the PMU init recipe.

set -euo pipefail

OUTPUT_DIR="${1:-/tmp/hw-learn-captures}"
TRACE_DIR="/sys/kernel/tracing"
[ -d "$TRACE_DIR" ] || TRACE_DIR="/sys/kernel/debug/tracing"
TRACE_FILE="$TRACE_DIR/trace"
TRACER_FILE="$TRACE_DIR/current_tracer"

mkdir -p "$OUTPUT_DIR"

echo "=== hw-learn MMIO Capture ==="
echo "Output directory: $OUTPUT_DIR"

# Verify we're root
if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: Must run as root (mmiotrace requires debugfs access)"
    exit 1
fi

# Verify NVIDIA driver is loaded
if ! lsmod | grep -q nvidia; then
    echo "ERROR: NVIDIA proprietary driver not loaded"
    echo "Load with: modprobe nvidia nvidia-uvm"
    exit 1
fi

# Verify debugfs is mounted
if [ ! -f "$TRACER_FILE" ]; then
    echo "Mounting debugfs..."
    mount -t debugfs none /sys/kernel/debug 2>/dev/null || true
fi

echo ""
echo "--- Phase 1a: Baseline capture (no compute) ---"
echo "Enabling mmiotrace..."
echo mmiotrace > "$TRACER_FILE"
sleep 2

echo "Capturing baseline (5 seconds)..."
sleep 5
cat "$TRACE_FILE" > "$OUTPUT_DIR/mmiotrace_baseline.txt"

echo "Disabling mmiotrace..."
echo nop > "$TRACER_FILE"
echo "" > "$TRACE_FILE"
sleep 1

BASELINE_LINES=$(wc -l < "$OUTPUT_DIR/mmiotrace_baseline.txt")
echo "Baseline captured: $BASELINE_LINES lines"

echo ""
echo "--- Phase 1b: Compute capture ---"
echo "Enabling mmiotrace..."
echo mmiotrace > "$TRACER_FILE"
sleep 2

echo "Triggering compute initialization..."
# Simple CUDA program that forces compute engine init
if command -v nvidia-smi &>/dev/null; then
    nvidia-smi -q > /dev/null 2>&1 || true
fi

# If CUDA is available, run a minimal compute workload
if command -v cuda-memcheck &>/dev/null || [ -x /usr/local/cuda/bin/cuda-memcheck ]; then
    echo "Running CUDA compute workload..."
    # Create a minimal CUDA program inline
    CUDA_PROG=$(mktemp /tmp/cuda_init_XXXXXX.cu)
    cat > "$CUDA_PROG" << 'CUDA_EOF'
#include <cuda_runtime.h>
int main() {
    float *d_a;
    cudaMalloc(&d_a, 1024);
    cudaMemset(d_a, 0, 1024);
    cudaFree(d_a);
    cudaDeviceSynchronize();
    return 0;
}
CUDA_EOF
    CUDA_BIN="${CUDA_PROG%.cu}"
    if nvcc -o "$CUDA_BIN" "$CUDA_PROG" 2>/dev/null; then
        "$CUDA_BIN"
        rm -f "$CUDA_BIN" "$CUDA_PROG"
    else
        echo "WARNING: nvcc not available, using nvidia-smi only"
        rm -f "$CUDA_PROG"
    fi
else
    echo "WARNING: CUDA toolkit not found, using nvidia-smi only"
fi

sleep 2
cat "$TRACE_FILE" > "$OUTPUT_DIR/mmiotrace_compute.txt"

echo "Disabling mmiotrace..."
echo nop > "$TRACER_FILE"

COMPUTE_LINES=$(wc -l < "$OUTPUT_DIR/mmiotrace_compute.txt")
echo "Compute captured: $COMPUTE_LINES lines"

echo ""
echo "--- Phase 1c: ioctl trace ---"
if command -v strace &>/dev/null && command -v nvidia-smi &>/dev/null; then
    strace -e trace=ioctl -o "$OUTPUT_DIR/ioctl_trace.txt" nvidia-smi -q 2>/dev/null || true
    IOCTL_LINES=$(wc -l < "$OUTPUT_DIR/ioctl_trace.txt")
    echo "ioctl trace captured: $IOCTL_LINES lines"
else
    echo "WARNING: strace or nvidia-smi not available, skipping ioctl trace"
fi

echo ""
echo "=== Capture complete ==="
echo "Files:"
ls -lh "$OUTPUT_DIR"/mmiotrace_*.txt "$OUTPUT_DIR"/ioctl_trace.txt 2>/dev/null
echo ""
echo "Next step: use toadstool JSON-RPC to extract PMU init recipe"
echo "  1. toadstool server  # start daemon"
echo "  2. echo '{\"jsonrpc\":\"2.0\",\"method\":\"compute.hardware.observe\",\"params\":{\"trace_file\":\"$OUTPUT_DIR/mmiotrace_compute.txt\"},\"id\":1}' | socat - UNIX-CONNECT:\${XDG_RUNTIME_DIR}/biomeos/compute.sock"
echo "  3. echo '{\"jsonrpc\":\"2.0\",\"method\":\"compute.hardware.distill\",\"params\":{\"baseline_file\":\"$OUTPUT_DIR/mmiotrace_baseline.txt\",\"compute_file\":\"$OUTPUT_DIR/mmiotrace_compute.txt\",\"chip\":\"gv100\"},\"id\":2}' | socat - UNIX-CONNECT:\${XDG_RUNTIME_DIR}/biomeos/compute.sock"
