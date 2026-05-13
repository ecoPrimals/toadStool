# Phase C: coral-driver Module Split Plan

**Date**: May 12, 2026 (S244 planning — executed S245–S250)
**Context**: Compute Trio Evolution — toadStool absorbs hardware lifecycle from coralReef
**Status**: EXECUTED — Phase C Batches 1–7 complete. See `TOADSTOOL_S245_*` through `TOADSTOOL_S250_*` handoffs.

---

## Principle

`coral-driver` does **not** depend on the compiler crate (`coral-reef` / `coralreef-core`).
The dependency boundary is clean: `coral-driver` → pure hardware; `coral-reef` → compiler.
`coralreef-core` depends on *both* (service/orchestration layer).

---

## toadStool Absorbs (Hardware Lifecycle)

These are bus-specific device management modules with no compiler coupling:

### VFIO Stack (`coral-driver/src/vfio/`)
- `device/` — VFIO device open, BAR mapping (`mapped_bar.rs`), config space
- `dma.rs` — `DmaBuffer`, page-aligned host memory, VFIO DMA map/unmap
- `pci_discovery/` — sysfs PCI device scanning under VFIO
- `isolation.rs` — fork-isolated BAR0 MMIO with D-state timeout/SIGKILL
- `sysfs_bar0.rs` — read-only BAR0 access helper
- `channel/` — VFIO channel setup (GPFIFO, pushbuf, semaphore)

### DRM Enumeration
- `drm.rs` — `enumerate_render_nodes()`, `/dev/dri/renderD*` scanning
- `linux_paths.rs` — sysfs path helpers

### AMD Hardware (`coral-driver/src/amd/`)
- `gem.rs` — GEM buffer object management
- `pm4.rs` — PM4 command stream encoding
- `ioctl.rs` — DRM ioctl wrappers
- `mod.rs` — `AmdDevice`, fence sync, CS submit

### NVIDIA Hardware (`coral-driver/src/nv/`)
- `bar0.rs` — sysfs BAR0 mmap
- `ioctl/` — DRM ioctl definitions (includes `gem.rs`)
- `pushbuf.rs` — pushbuffer encoding
- `qmd/` — Compute queue descriptor
- `probe.rs` — nouveau device probe
- `vfio_compute/` — VFIO compute dispatch (GR, BAR0, semaphore)

### Device Abstraction
- `hardware.rs` — `ComputeDevice`, `CompletionStyle`, `HardwareProbe`
- `error.rs` — Driver error types

### Multi-GPU Topology
- `coral-gpu/src/pcie.rs` — `probe_pcie_topology()` (sysfs scan, PCIe switch grouping)

---

## coralReef Retains (Compiler Pipeline)

### Compiler Core
- `coral-reef/` — WGSL/SPIR-V/GLSL → vendor binaries (ISA, codegen, IR, legalization)
- `coral-reef-isa/` — ISA tables
- `coral-reef-stubs/` — Mesa replacement stubs
- `coral-reef-bitview/` — Bit field helpers
- `nak-ir-proc/` — Procedural macros for NAK IR

### Service Layer
- `coralreef-core/` — Primal orchestration, health, lifecycle, IPC, JSON-RPC
- **Note**: `coralreef-core/src/discovery.rs` calls `coral_driver::enumerate_render_nodes()`
  for DRM fallback. Post-absorption, this should route through toadStool's
  `compute.dispatch.capabilities` IPC instead

### Compiler+Dispatch Facade
- `coral-gpu/` — Unified `GpuContext` (compile + dispatch). The *compilation* part stays;
  the *dispatch routing* may use toadStool IPC post-Phase D

### Intel Skeleton
- `coral-driver/src/intel/` — DRM syncobj/fence skeleton. Currently compile-only.
  Stays with coralReef until Intel hardware dispatch is real, then moves in a future phase

### GSP Firmware
- `coral-driver/src/gsp/` — NVIDIA GSP firmware interface. Compiler-adjacent (firmware
  loading for codegen targets). Stays with coralReef

---

## Boundary Decisions

| Module | Owner | Rationale |
|--------|-------|-----------|
| `vfio/` | toadStool | Pure hardware lifecycle, no compiler deps |
| `amd/` | toadStool | Device management (GEM/PM4/fence), not codegen |
| `nv/` (hardware) | toadStool | Device management (BAR0/pushbuf/QMD), not codegen |
| `nv/` (codegen helpers) | coralReef | Any modules that encode shader-specific payloads |
| `drm.rs` + `linux_paths.rs` | toadStool | Enumeration infrastructure |
| `hardware.rs` + `error.rs` | toadStool | Device abstraction layer |
| `gsp/` | coralReef | Firmware loading for compiler targets |
| `intel/` | coralReef | Skeleton, not ready for hardware lifecycle |
| `cuda/` (cudarc) | Neither | Deprecated — capability IPC replaces direct binding |

---

## Absorption Pattern

Same as Phase A (ember) and Phase B (glowplug):
1. Copy portable modules into `toadstool-cylinder` crate
2. Adapt to toadStool's `ResourceHandle` trait surface
3. Wire-only principle: no shared Rust crate, JSON-RPC IPC between primals
4. Delete cross-process IPC stubs that are replaced by internal calls
5. All tests must pass in toadStool workspace before coralReef soft-deprecates

---

## Post-Absorption

- coralReef soft-deprecates `coral-driver` hardware modules (same pattern as ember/glowplug)
- `coralreef-core/discovery.rs` DRM fallback → toadStool IPC query
- Phase D: `compute.dispatch.submit` executes locally through absorbed driver layer
