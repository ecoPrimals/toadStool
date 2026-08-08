#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# toadStool Cross-Architecture Verification
#
# Validates that the entire workspace type-checks against all supported
# architecture targets. This is a cargo check (no linking), so cross-linkers
# are NOT required — only the Rust std library for each target.
#
# Install all targets:
#   rustup target add $(grep -oP '"\K[^"]+' <<< "$TARGETS")
#
# Usage:
#   ./scripts/cross-arch-check.sh          # run full sweep
#   ./scripts/cross-arch-check.sh quick    # tier 1 only (fast)

set -euo pipefail

TIER1_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

TIER2_TARGETS=(
    "x86_64-pc-windows-msvc"
    "aarch64-pc-windows-gnullvm"
    "aarch64-linux-android"
    "armv7-unknown-linux-gnueabihf"
    "riscv64gc-unknown-linux-gnu"
    "powerpc64le-unknown-linux-gnu"
    "s390x-unknown-linux-gnu"
    "loongarch64-unknown-linux-gnu"
)

if [[ "${1:-full}" == "quick" ]]; then
    TARGETS=("${TIER1_TARGETS[@]}")
    echo "=== toadStool Cross-Arch Check (Tier 1 — ${#TARGETS[@]} targets) ==="
else
    TARGETS=("${TIER1_TARGETS[@]}" "${TIER2_TARGETS[@]}")
    echo "=== toadStool Cross-Arch Check (Full — ${#TARGETS[@]} targets) ==="
fi

echo ""
PASS=0
FAIL=0
FAILED_LIST=()

for t in "${TARGETS[@]}"; do
    printf "  %-42s " "$t"
    if cargo check --workspace --target "$t" 2>/dev/null; then
        echo "✓"
        PASS=$((PASS + 1))
    else
        echo "✗ FAIL"
        FAIL=$((FAIL + 1))
        FAILED_LIST+=("$t")
    fi
done

echo ""
echo "=== Results: ${PASS}/${#TARGETS[@]} pass ==="

if [[ ${FAIL} -gt 0 ]]; then
    echo ""
    echo "FAILED targets:"
    for t in "${FAILED_LIST[@]}"; do
        echo "  - $t"
    done
    exit 1
fi
