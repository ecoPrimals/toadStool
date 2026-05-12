# S246 Handoff — Phase C Batch 2: MMIO + AMD Backend Absorption

**Date**: May 12, 2026
**From**: toadStool S246
**Status**: Green — all quality gates pass

---

## What Was Done

### Phase C Absorption — Batch 2

Absorbed 8 modules from `coral-driver` into `toadstool-cylinder`:

**MMIO Foundation** (used by all hardware backends):
- `mmio.rs` — `VolatilePtr<T>` safe wrapper for volatile register access
- `mmio_region.rs` — `MmioRegion` RAII wrapper with bounds-checked u32 read/write

**AMD GPU Backend** (complete amdgpu DRM path):
- `amd/mod.rs` — `AmdDevice` implementing `ComputeDevice` trait
- `amd/ioctl.rs` — Pure Rust DRM ioctl definitions (GEM, context, VA, CS, fence)
- `amd/pm4.rs` — PM4 command buffer construction for GFX9-12
- `amd/gem.rs` — GEM buffer object management
- `amd/generation.rs` — Per-generation profiles (GCN5 Vega, RDNA2, RDNA3, RDNA4)
- `amd/shader_binary.rs` — AMDGPU ELF format detection and metadata extraction

### Deep Debt Sweep

- 6 hardcoded `Duration` literals extracted to named constants in 4 files
- Full audit confirmed zero production files >800L
- All 46 unsafe blocks SAFETY-documented
- All production mocks behind `#[cfg(any(test, feature = "test-mocks"))]`
- Zero production `println!`/`eprintln!`

## Quality Metrics

| Metric | Value |
|--------|-------|
| Tests (lib-only) | 8,430 |
| Cylinder tests | 141 |
| New tests (S246) | +81 |
| Clippy warnings | 0 |
| Production panics | 0 |
| Production println | 0 |

## Cylinder Crate Status

| Module | Status | Tests |
|--------|--------|------:|
| `lib.rs` (ComputeDevice, BufferHandle, etc.) | Absorbed S245 | 6 |
| `drm.rs` (DRM ioctl, MappedRegion) | Absorbed S245 | 27 |
| `linux_paths.rs` (sysfs/procfs helpers) | Absorbed S245 | 18 |
| `hardware.rs` (HardwareCapabilities) | Absorbed S245 | 6 |
| `error/` (DriverError, VFIO errors) | Absorbed S245 | 3 |
| `mmio.rs` (VolatilePtr) | **Absorbed S246** | 4 |
| `mmio_region.rs` (MmioRegion) | **Absorbed S246** | 5 |
| `amd/` (complete backend) | **Absorbed S246** | 72 |

## Next Steps (S247+)

1. **Absorb NVIDIA hardware modules** — `nv/bar0.rs`, `nv/pushbuf.rs`, `nv/probe.rs`,
   `nv/generation.rs`, `nv/identity/`, `nv/ioctl/`, `nv/qmd/`
2. **Absorb VFIO foundation** — `vfio/device/`, `vfio/dma.rs`, `vfio/ioctl.rs`,
   `vfio/pci_discovery/`, `vfio/types.rs`
3. **Absorb `pcie.rs`** from `coral-gpu` — `probe_pcie_topology()` sysfs scan
4. **Coverage push** — MockVfioDevice/MockPciSysfs for hardware-gated test paths
5. **Phase D prep** — `compute.dispatch.submit` through absorbed driver layer

## Key Decisions

- MMIO modules are `pub(crate)` — internal implementation detail, not public API
- AMD backend is `pub mod amd` behind `#[cfg(target_os = "linux")]` gate
- `dead_code` allows on MMIO modules with reason annotations (used by future VFIO/NV)
- No modifications to coral-driver source during copy — clean absorption pattern
