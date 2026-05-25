# ToadStool S248 — Phase C Batch 4: VFIO Foundation Absorption + Deep Debt

**Date**: May 12, 2026
**Session**: S248
**Phase**: C (coral-driver hardware absorption into toadstool-cylinder)

---

## Summary

Absorbed 40 VFIO files from `coral-driver/src/vfio/` into `toadstool-cylinder`,
establishing the sovereign VFIO device layer. Resolved the single `gsp` firmware
dependency by recreating `RegisterAccess`/`ApplyError` locally. Parallel deep
debt sweep extracted ~10 more Duration constants.

## What Was Absorbed

### VFIO Kernel ABI (`vfio/types.rs`)
- `repr(C)` structs for VFIO container, group, device, region, IRQ operations
- iommufd bind/attach/ioas structs
- Complete ioctl opcode constants

### VFIO Ioctl Layer (`vfio/ioctl.rs`)
- Rustix-based VFIO/iommufd ioctl wrappers
- Container/group/device lifecycle operations

### DMA (`vfio/dma.rs`)
- `DmaBuffer` with IOMMU mapping
- `DmaBackend` variants (VFIO container, iommufd)

### PCI Discovery (`vfio/pci_discovery/` — 7 files)
- Sysfs PCI device enumeration
- Config space parsing (vendor/device/class)
- Power management (D0-D3 state transitions)
- `PciDeviceInfo`, `GpuVendor` types
- `force_pci_d0()` for GPU wake-up

### Device Layer (`vfio/device/` — 7 files)
- `VfioDevice` — VFIO device open, group attach, region info
- `MappedBar` — BAR0 mmap with volatile register access
- `DmaBackend` — VFIO container vs iommufd backend selection
- Bus master enable, device handles, runtime state
- **Local `RegisterAccess`/`ApplyError`** (gsp decoupling)

### BAR & Vendor (`vfio/bar_cartography.rs`, `gpu_vendor.rs`, `amd_metal.rs`, `nv_metal/`)
- BAR0 register classification and region scanning
- `GpuMetal` trait for vendor-specific GPU identity
- AMD Vega (GFX906) metal detection
- NVIDIA Volta metal detection and probing

### Memory & Init Types
- `memory/` — topology, regions, PRAMIN/DMA region types
- `sovereign_types.rs` — init pipeline options/results
- `sysfs_bar0.rs` — sysfs resource0 mmap fallback
- `ember_client.rs` / `ember_gate.rs` — fd-passing client/gate

### Support (`cache_ops.rs`, `isolation.rs`, `irq.rs`, `pci_config.rs`)
- x86 cache flush/fence for DMA coherence
- Fork-isolated MMIO (read/write/batch with timeout)
- MSI/MSI-X IRQ → eventfd wiring
- PM capability offset shim

## What Was Deferred

- `vfio/channel/` — Full channel orchestration (PFIFO, devinit, glowplug,
  HBM2 training, diagnostics, page tables). Heavy `nv::vfio_compute` ties
- `vfio/sovereign_init.rs` — Pipeline orchestration, depends on `nv::generation`
  and `vfio_compute`
- `vfio/sovereign_stages.rs` — BAR0 probe, PMC, training stages, falcon boot
- `pcie.rs` from coral-gpu — Imports `coral_reef::GpuTarget`, needs local type

## gsp Boundary Resolution

The only `gsp` dependency was in `device/mapped_bar.rs`:
```rust
use crate::gsp::{ApplyError, RegisterAccess};
```

Resolved by recreating both types locally in `mapped_bar.rs`:
- `RegisterAccess` trait — identical `read_u32`/`write_u32` interface
- `ApplyError` enum — MmioFailed/VerifyFailed/ThermalLimit variants

This cleanly decouples the VFIO device layer from coralReef's GSP firmware
module while maintaining the same API contract for future bridge code.

## Deep Debt Sweep

| File | Constants Extracted |
|------|--------------------|
| `edge/discovery/mod.rs` | 4 (serial/network/BT/mDNS timeouts) |
| `server/background/statistics.rs` | 1 (stats collection interval) |
| `container/types.rs` | 1 (pull timeout) |
| `monitoring/reporting.rs` | 1 (CPU sample window) |
| `orchestration/policy.rs` | 1 (no-history sentinel) |
| `edge/arduino/edge_device.rs` | 1 (serial read timeout) |

## Quality Gates

- **Clippy**: Zero warnings (`-D warnings`), full workspace
- **Tests**: 8,704 lib-only passing (up from 8,583)
- **Cylinder tests**: 415 (up from 294 — 121 new from VFIO)
- **Unsafe**: All blocks SAFETY-documented
- **Production mocks**: All gated behind `test-mocks` feature

## Phase C Progress

| Batch | Session | Modules | Cylinder Tests |
|-------|---------|---------|----------------|
| 1 | S245 | DRM, linux_paths, hardware, error, ComputeDevice | 60 |
| 2 | S246 | MMIO, AMD backend (6 files) | 141 |
| 3 | S247 | NV identity, generation, pushbuf, ioctl, QMD | 294 |
| **4** | **S248** | **VFIO foundation (40 files)** | **415** |

## Next Steps (S249+)

1. **VFIO channel modules** — devinit, glowplug, HBM2 training, diagnostics
   (requires resolving `nv::vfio_compute` boundary)
2. **sovereign init/stages** — pipeline orchestration (heavy nv:: ties)
3. **Phase D prep** — Wire `compute.dispatch.submit` through absorbed driver layer
4. **Coverage push** — MockVfioDevice/MockPciSysfs for hardware-gated test paths
5. **pcie.rs** — Create local `GpuTarget` type to replace coral-reef dependency
