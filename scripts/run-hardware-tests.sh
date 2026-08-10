#!/bin/bash
# ToadStool Hardware Test Runner — strandgate fleet
# Runs #[ignore] tests on real hardware: AMD RX 6950 XT, NVIDIA RTX 3090, Akida AKD1000
#
# Usage:
#   ./scripts/run-hardware-tests.sh          # Run all hardware tests
#   ./scripts/run-hardware-tests.sh gpu      # GPU tests only
#   ./scripts/run-hardware-tests.sh npu      # NPU tests only
#   ./scripts/run-hardware-tests.sh display  # V4L2/display tests only
#   ./scripts/run-hardware-tests.sh coverage # Full hardware coverage with llvm-cov

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

SUITE="${1:-all}"

# strandgate hardware map
AMD_SLOT="0000:25:00.0"
NVIDIA_SLOT="0000:41:00.0"
AKIDA_SLOT="0000:e2:00.0"

detect_hardware() {
    echo -e "${CYAN}Hardware Detection${NC}"
    echo "======================================"

    HAS_AMD=false
    HAS_NVIDIA=false
    HAS_AKIDA=false
    HAS_V4L2=false

    if [ -d "/sys/class/drm/card0/device" ]; then
        local vendor
        vendor=$(cat /sys/class/drm/card0/device/vendor 2>/dev/null || echo "")
        if [ "$vendor" = "0x1002" ]; then
            HAS_AMD=true
            echo -e "  ${GREEN}AMD GPU${NC}: card0 (renderD128) — RX 6950 XT at $AMD_SLOT"
        fi
    fi

    if [ -d "/sys/class/drm/card1/device" ]; then
        local vendor
        vendor=$(cat /sys/class/drm/card1/device/vendor 2>/dev/null || echo "")
        if [ "$vendor" = "0x10de" ]; then
            HAS_NVIDIA=true
            echo -e "  ${GREEN}NVIDIA GPU${NC}: card1 (renderD129) — RTX 3090 at $NVIDIA_SLOT"
        fi
    fi

    if lspci 2>/dev/null | grep -qi "brainchip\|akida"; then
        HAS_AKIDA=true
        local akida_dev="/dev/akida0"
        if [ -e "$akida_dev" ]; then
            echo -e "  ${GREEN}Akida NPU${NC}: $akida_dev — AKD1000 at $AKIDA_SLOT"
        else
            echo -e "  ${YELLOW}Akida NPU${NC}: PCIe present at $AKIDA_SLOT (driver not loaded)"
        fi
    fi

    if ls /dev/video* &>/dev/null; then
        HAS_V4L2=true
        echo -e "  ${GREEN}V4L2 Capture${NC}: $(ls /dev/video* 2>/dev/null | head -1)"
    fi

    echo ""
    echo -e "  CPU: $(nproc) cores — $(cat /proc/cpuinfo | grep 'model name' | head -1 | cut -d: -f2 | xargs)"
    echo "======================================"
    echo ""
}

run_gpu_tests() {
    echo -e "${GREEN}GPU Hardware Tests${NC}"
    echo "--------------------------------------"

    if ! $HAS_AMD && ! $HAS_NVIDIA; then
        echo -e "${YELLOW}No GPU hardware detected — skipping GPU tests${NC}"
        return 0
    fi

    local failed=0

    if $HAS_AMD; then
        echo -e "${CYAN}Testing AMD adapter (card0, renderD128)...${NC}"
        TOADSTOOL_GPU_ADAPTER="0" \
            cargo test --workspace -- --ignored \
            --test-threads=1 2>&1 \
            | grep -E "test result" || failed=$((failed + 1))
        echo ""
    fi

    if $HAS_NVIDIA; then
        echo -e "${CYAN}Testing NVIDIA adapter (card1, renderD129)...${NC}"
        TOADSTOOL_GPU_ADAPTER="1" \
            cargo test --workspace -- --ignored \
            --test-threads=1 2>&1 \
            | grep -E "test result" || failed=$((failed + 1))
        echo ""
    fi

    echo -e "${CYAN}Testing auto adapter selection...${NC}"
    TOADSTOOL_GPU_ADAPTER="auto" \
        cargo test -p toadstool-runtime-universal -- --ignored \
        --test-threads=1 2>&1 \
        | grep -E "test result" || failed=$((failed + 1))
    echo ""

    echo -e "${CYAN}Testing GPU runtime crates...${NC}"
    for crate in toadstool-runtime-gpu toadstool-runtime-universal; do
        echo -e "  ${YELLOW}$crate${NC}"
        cargo test -p "$crate" -- --ignored --test-threads=1 2>&1 \
            | grep -E "test result" || failed=$((failed + 1))
    done

    if [ $failed -gt 0 ]; then
        echo -e "${RED}GPU tests had $failed failures${NC}"
        return 1
    fi
    echo -e "${GREEN}GPU tests passed${NC}"
}

run_npu_tests() {
    echo -e "${GREEN}NPU Hardware Tests (Akida AKD1000)${NC}"
    echo "--------------------------------------"

    if ! $HAS_AKIDA; then
        echo -e "${YELLOW}No Akida NPU detected — skipping NPU tests${NC}"
        return 0
    fi

    if [ ! -e "/dev/akida0" ]; then
        echo -e "${YELLOW}Akida PCIe present but driver not loaded — skipping NPU tests${NC}"
        echo -e "  Load driver: sudo modprobe akida  OR  ./scripts/install-akida-driver.sh"
        return 0
    fi

    local failed=0

    # Neuromorphic crates require hardware (Akida NPU) or strandGate-local config.
    echo -e "${CYAN}Testing akida-driver backends...${NC}"
    cargo test -p akida-driver -- --ignored --test-threads=1 2>&1 \
        | grep -E "test result" || failed=$((failed + 1))

    echo -e "${CYAN}Testing cross-substrate validation...${NC}"
    cargo test -p cross-substrate-validation -- --ignored --test-threads=1 2>&1 \
        | grep -E "test result" || failed=$((failed + 1))

    if [ $failed -gt 0 ]; then
        echo -e "${RED}NPU tests had $failed failures${NC}"
        return 1
    fi
    echo -e "${GREEN}NPU tests passed${NC}"
}

run_display_tests() {
    echo -e "${GREEN}Display / V4L2 Hardware Tests${NC}"
    echo "--------------------------------------"

    if ! $HAS_V4L2; then
        echo -e "${YELLOW}No V4L2 capture device detected — skipping display tests${NC}"
        return 0
    fi

    local failed=0

    echo -e "${CYAN}Testing display transport...${NC}"
    cargo test -p toadstool-display -- --ignored --test-threads=1 2>&1 \
        | grep -E "test result" || failed=$((failed + 1))

    if [ $failed -gt 0 ]; then
        echo -e "${RED}Display tests had $failed failures${NC}"
        return 1
    fi
    echo -e "${GREEN}Display tests passed${NC}"
}

run_coverage() {
    echo -e "${GREEN}Hardware Coverage (llvm-cov with --ignored)${NC}"
    echo "--------------------------------------"

    cargo llvm-cov clean

    echo -e "${CYAN}Running standard tests with coverage...${NC}"
    cargo llvm-cov --workspace --no-report -- --skip performance_bench --skip slow 2>&1 \
        | grep -E "test result" || true

    if $HAS_AMD || $HAS_NVIDIA; then
        echo -e "${CYAN}Running GPU --ignored tests with coverage...${NC}"
        TOADSTOOL_GPU_ADAPTER="auto" \
            cargo llvm-cov -p toadstool-runtime-gpu -p toadstool-runtime-universal \
            --no-report -- --ignored --test-threads=1 2>&1 \
            | grep -E "test result" || true
    fi

    if $HAS_AKIDA && [ -e "/dev/akida0" ]; then
        # Neuromorphic crates require Akida hardware for --ignored tests.
        echo -e "${CYAN}Running NPU --ignored tests with coverage...${NC}"
        cargo llvm-cov -p akida-driver -p cross-substrate-validation \
            --no-report -- --ignored --test-threads=1 2>&1 \
            | grep -E "test result" || true
    fi

    echo ""
    echo -e "${CYAN}Generating combined report...${NC}"
    cargo llvm-cov report --html
    COVERAGE=$(cargo llvm-cov report 2>/dev/null | grep "TOTAL" | awk '{print $10}')
    echo ""
    echo -e "${GREEN}Coverage with hardware: $COVERAGE${NC}"
    echo -e "  HTML report: target/llvm-cov/html/index.html"
}

# Main
detect_hardware

case "$SUITE" in
    gpu)
        run_gpu_tests
        ;;
    npu)
        run_npu_tests
        ;;
    display)
        run_display_tests
        ;;
    coverage)
        run_coverage
        ;;
    all)
        run_gpu_tests
        echo ""
        run_npu_tests
        echo ""
        run_display_tests
        echo ""
        echo "======================================"
        echo -e "${GREEN}All hardware tests complete${NC}"
        echo "  For coverage: $0 coverage"
        ;;
    *)
        echo "Usage: $0 [gpu|npu|display|coverage|all]"
        exit 1
        ;;
esac
