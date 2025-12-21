#!/bin/bash
# Coverage Collection Script
# Created: November 26, 2025
# Purpose: Collect per-crate coverage to work around llvm-cov workspace hanging

set -e

echo "🍄 ToadStool Coverage Collection Script"
echo "========================================"
echo ""

COVERAGE_DIR="target/coverage-reports"
mkdir -p "$COVERAGE_DIR"

# Core packages (usually work well)
CORE_PACKAGES=(
    "toadstool-common"
    "toadstool-config"
    "toadstool"
)

# Runtime packages
RUNTIME_PACKAGES=(
    "toadstool-runtime-wasm"
    "toadstool-runtime-native"
    "toadstool-runtime-container"
)

# Integration packages
INTEGRATION_PACKAGES=(
    "toadstool-integration-nestgate"
    "toadstool-integration-protocols"
    "toadstool-integration-primals"
)

# Main packages
MAIN_PACKAGES=(
    "toadstool-cli"
    "toadstool-server"
    "toadstool-api"
    "toadstool-distributed"
)

echo "📊 Collecting coverage for core packages..."
for pkg in "${CORE_PACKAGES[@]}"; do
    echo "  → $pkg"
    cargo llvm-cov --package "$pkg" --text > "$COVERAGE_DIR/${pkg}.txt" 2>&1 || echo "    ⚠️  Failed (likely no tests)"
done

echo ""
echo "📊 Collecting coverage for runtime packages..."
for pkg in "${RUNTIME_PACKAGES[@]}"; do
    echo "  → $pkg"
    cargo llvm-cov --package "$pkg" --text > "$COVERAGE_DIR/${pkg}.txt" 2>&1 || echo "    ⚠️  Failed (likely no tests)"
done

echo ""
echo "📊 Collecting coverage for integration packages..."
for pkg in "${INTEGRATION_PACKAGES[@]}"; do
    echo "  → $pkg"
    cargo llvm-cov --package "$pkg" --text > "$COVERAGE_DIR/${pkg}.txt" 2>&1 || echo "    ⚠️  Failed (likely no tests)"
done

echo ""
echo "📊 Collecting coverage for main packages..."
for pkg in "${MAIN_PACKAGES[@]}"; do
    echo "  → $pkg"
    # These may take longer
    timeout 300 cargo llvm-cov --package "$pkg" --text > "$COVERAGE_DIR/${pkg}.txt" 2>&1 || echo "    ⚠️  Failed or timed out"
done

echo ""
echo "✅ Coverage collection complete!"
echo "📁 Reports saved to: $COVERAGE_DIR/"
echo ""
echo "📊 Summary:"
echo "==========="

# Extract TOTAL coverage lines
for file in "$COVERAGE_DIR"/*.txt; do
    pkg=$(basename "$file" .txt)
    total=$(grep "^TOTAL" "$file" 2>/dev/null | awk '{print $NF}')
    if [ -n "$total" ]; then
        printf "%-40s %s\n" "$pkg:" "$total"
    fi
done

echo ""
echo "💡 To view detailed HTML report for a package:"
echo "   cargo llvm-cov --package <package-name> --html --open"
echo ""
echo "⚠️  Note: Some packages may fail due to no tests or hanging performance tests."
echo "   This is expected and documented in LLVM_COV_WORKAROUND_GUIDE.md"

