#!/usr/bin/env bash
# Akida Detection Demo
# Deep Debt: No sudo required - uses best available backend

set -euo pipefail

echo "=== Akida NPU Detection Demo ==="
echo ""
echo "Testing dual-backend driver architecture..."
echo ""

# Auto-select best backend
echo "[1/4] Auto-detection (tries kernel, falls back to userspace)..."
cargo run --release --example detect_akida || {
    echo "   No hardware detected (expected in most environments)"
}
echo ""

# Try userspace explicitly
echo "[2/4] Userspace backend (no kernel module needed)..."
cargo run --release --example detect_akida -- --backend=userspace || {
    echo "   No hardware detected (expected in most environments)"
}
echo ""

# Enumerate boards
echo "[3/4] Enumerating all Akida boards..."
cargo run --release --example enumerate_boards || {
    echo "   No hardware detected (expected in most environments)"
}
echo ""

# Query capabilities
echo "[4/4] Querying NPU capabilities..."
cargo run --release --example query_capabilities || {
    echo "   No hardware detected (expected in most environments)"
}
echo ""

echo "=== Demo Complete ==="
echo ""
echo "Driver Status:"
if lsmod | grep -q akida 2>/dev/null; then
    echo "  ✓ Kernel driver loaded (high performance)"
elif ls /sys/bus/pci/devices/*/resource0 2>/dev/null | grep -q .; then
    echo "  ✓ Userspace driver available (no kernel module needed)"
else
    echo "  ℹ No Akida hardware detected"
fi
echo ""
echo "For production deployment:"
echo "  sudo ../../scripts/install-akida-driver.sh"
echo ""
echo "See: docs/guides/AKIDA_DRIVER_DEPLOYMENT.md"
