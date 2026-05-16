# toadStool S264 — No-FLR Warm Swap Pattern

**Date:** May 15, 2026
**From:** hotSpring (biomeGate)
**For:** toadStool diesel engine, coralReef sovereign dispatch, sibling springs

## Summary

Hardware-validated pattern for preserving GPU initialization state through driver swaps by disabling PCIe Function Level Reset (FLR). Implemented in `SysfsSwapExecutor`.

## Problem

When vfio-pci binds to a PCI device, it triggers an FLR which resets all GPU state:
- PRI Ring (internal register bus) → dead
- Clock trees (PLLs from DEVINIT) → reset
- Memory training (HBM2/GDDR5) → lost
- Falcon firmware (FECS/GPCCS) → HRESET
- PGRAPH engine → clock-gated

This destroys any initialization performed by a vendor driver (nouveau, nvidia, amdgpu).

## Solution

Clear the `reset_method` sysfs attribute before binding vfio-pci:

```
echo "" > /sys/bus/pci/devices/$BDF/reset_method
```

This prevents FLR during the vfio-pci probe. The kernel logs "All device reset methods disabled by user" and proceeds without reset.

### Implementation in Diesel Engine

`SysfsSwapExecutor::execute_swap()` now auto-detects warm-preserving swaps (init-driver → vfio-pci) and disables FLR:

```rust
fn is_warm_preserving_swap(from: &str, to: &str) -> bool {
    let is_init_driver = matches!(from, "nouveau" | "nvidia" | "amdgpu" | "xe" | "i915");
    let is_vfio = to == "vfio-pci";
    is_init_driver && is_vfio
}
```

Combined with `pin_bridge_hierarchy()` and `SwapGuard` burst keepalive for PLX bridges.

## Hardware Validation

**Titan V (GV100)** — direct PCIe (no PLX):
- nouveau init → disable FLR → unbind → bind vfio-pci
- **27/27 registers alive** through swap (zero PRI faults)
- PRI Ring, PGRAPH HUB, GPC registers all preserved
- PMC_ENABLE writable from VFIO (bit 12 GR toggle verified)
- FECS in HRESET (nouveau couldn't boot — missing PMU firmware)

**K80 (GK210B)** — behind PLX PEX 8747:
- nouveau init succeeded (12 GiB GDDR5, 5 GPCs)
- Manual sysfs pinning **insufficient** — PLX D3cold during swap
- Requires `SwapGuard` burst keepalive from diesel engine

## Constraints

1. FLR disabled means the device retains state from the previous driver — security implication for VM passthrough (not relevant for sovereign compute)
2. Parent bridge must be pinned (`power/control=on`, `d3cold_allowed=0`) before swap
3. PLX bridges need continuous CfgRd during swap window (`SwapGuard`)
4. FECS running requires the initializing driver to have completed ACR/SEC2 bootstrap

## Files Changed

- `crates/core/glowplug/src/sysfs_executor.rs`: `disable_flr()`, `restore_flr()`, `is_warm_preserving_swap()`, auto-detection in `execute_swap()`

## Pattern for Other Hardware

This pattern applies to any GPU where sovereign userspace access needs initialized hardware state:
- AMD Vega/RDNA: amdgpu → vfio-pci (preserves PSP state)
- Intel Xe: xe → vfio-pci (preserves GuC state)
- Multi-GPU workstations: selective warm swap per card
