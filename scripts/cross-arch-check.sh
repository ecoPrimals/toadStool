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
#   ./scripts/cross-arch-check.sh          # run full sweep (tier 1+2)
#   ./scripts/cross-arch-check.sh quick    # tier 1 only (fast)
#   ./scripts/cross-arch-check.sh wasm     # tier 3 WASM compute subset

set -euo pipefail

TIER1_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "aarch64-apple-ios"
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

TIER3_TARGETS=(
    "wasm32-unknown-unknown"
    "wasm32-wasip1"
)

WASM_CRATES=(
    "toadstool"
    "toadstool-common"
    "toadstool-config"
    "toadstool-core"
    "toadstool-hw-safe"
    "toadstool-sysmon"
    "toadstool-integration-primals"
    "toadstool-integration-storage"
    "toadstool-management-resources"
    "toadstool-management-performance"
    "toadstool-management-analytics"
    "toadstool-runtime-secure-enclave"
    "toadstool-runtime-universal"
    "toadstool-runtime-orchestration"
    "toadstool-runtime-adaptive"
    "toadstool-runtime-specialty"
    "toadstool-security-monitoring"
    "toadstool-security-policies"
    "toadstool-integration-security"
    "toadstool-ember"
    "toadstool-cylinder"
    "toadstool-glowplug"
    "cross-substrate-validation"
    "hw-learn"
    "nvpmu"
    "akida-chip"
    "akida-models"
    "akida-driver"
    "akida-setup"
    "akida-reservoir-research"
    "neurobench-runner"
    "toadstool-auto-config"
    "toadstool-client"
    "toadstool-integration-protocols"
    "toadstool-management-monitoring"
    "toadstool-distributed"
    "toadstool-runtime-wasm"
    "toadstool-runtime-gpu"
)

if [[ "${1:-full}" == "quick" ]]; then
    TARGETS=("${TIER1_TARGETS[@]}")
    echo "=== toadStool Cross-Arch Check (Tier 1 — ${#TARGETS[@]} targets) ==="
elif [[ "${1:-full}" == "wasm" ]]; then
    echo "=== toadStool Cross-Arch Check (Tier 3 WASM — ${#TIER3_TARGETS[@]} targets × ${#WASM_CRATES[@]} crates) ==="
    echo ""
    PASS=0
    FAIL=0
    FAILED_LIST=()
    for t in "${TIER3_TARGETS[@]}"; do
        echo "  Target: $t"
        for c in "${WASM_CRATES[@]}"; do
            printf "    %-42s " "$c"
            if cargo check -p "$c" --no-default-features --target "$t" 2>/dev/null; then
                echo "✓"
                PASS=$((PASS + 1))
            else
                echo "✗ FAIL"
                FAIL=$((FAIL + 1))
                FAILED_LIST+=("$c@$t")
            fi
        done
        echo ""
    done
    TOTAL=$((${#TIER3_TARGETS[@]} * ${#WASM_CRATES[@]}))
    echo "=== Results: ${PASS}/${TOTAL} pass ==="
    if [[ ${FAIL} -gt 0 ]]; then
        echo ""
        echo "FAILED:"
        for f in "${FAILED_LIST[@]}"; do
            echo "  - $f"
        done
        exit 1
    fi
    exit 0
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
