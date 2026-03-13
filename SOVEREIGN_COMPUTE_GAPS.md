# Sovereign Compute — Remaining Gaps

**Date**: March 13, 2026 — S151
**Purpose**: Single checklist of work remaining before toadStool's sovereign compute pipeline is complete.
**Scope**: toadStool-owned gaps only. barraCuda and coralReef track their own.

---

## Architecture Reference

```
barraCuda (WHAT) → coralReef (COMPILE+DISPATCH) → toadStool (WHERE+ORCHESTRATE)
                     │ VFIO transport              │ VFIO interface
                     │ (coral-driver: channels,    │ (device mgmt, BAR0 init,
                     │  QMD, pushbuf, DMA)         │  permissions, pooling,
                     │                             │  thermal safety, routing)
```

Three dispatch paths:

| Path | Driver | BAR0 | Channels | toadStool Status |
|------|--------|------|----------|------------------|
| **Sovereign** | none (VFIO) | `VfioBar0Access` | coralReef coral-driver | Interface ready, transport WIP (coralReef) |
| **nouveau** | nouveau | sysfs `resource0` | kernel | Functional — BAR0 GR init proven |
| **nvidia** | nvidia UVM | n/a | UAPI | Functional — CTXNOTVALID resolved |

---

## Critical Path (P0) — Blocks sovereign dispatch

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 1 | **Dispatch client** | `server/handler/` | toadStool JSON-RPC method that accepts a compiled binary from coralReef and triggers dispatch on the target GPU. Today `compute.hardware.apply` handles BAR0 init recipes; need `compute.dispatch.submit` for actual compute workloads via coralReef's `dispatch_binary()`. | coralReef dispatch API stable |
| 2 | **VFIO hardware validation** | `nvpmu::vfio` | `VfioBar0Access` is implemented but untested on real VFIO-bound GPU hardware. Need a test rig with `vfio-pci` bound GPU + validation script. | Hardware access |
| 3 | ~~Error recovery / rollback~~ | `nvpmu::init` | ✅ **Resolved S151**: `RegisterSnapshot` captures pre-init state, `apply_with_recovery` rolls back on failure, `NvPmuError::PartialInit` reports rollback status. `init.rs` evolved to `dyn RegisterAccess`. | — |
| 4 | ~~DMA buffer support~~ | `nvpmu::dma` | ✅ **Resolved S151**: `DmaAllocator` + `DmaBuffer` ported from akida-driver. Page-aligned, mlock'd, IOMMU-mapped with automatic cleanup. | — |

## High Priority (P1) — Required for multi-arch and production

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 5 | **Multi-arch register classification** | `hw-learn` | Currently NVIDIA-only. AMD (AMDGPU MMIO) and Intel (Xe MMIO) need register classification, recipe format, and applicator support. | Register documentation |
| 6 | ~~Unified PCI discovery~~ | `toadstool-common` | ✅ **Resolved S151**: `pci_discovery::discover_pci_devices()` with `PciFilter` (vendor, class, device IDs). Vendor constants for NVIDIA, Brainchip, AMD, Intel. Shared scanner for GPU + NPU + any accelerator. | — |
| 7 | **Test coverage → 90%** | Workspace | ~86% line coverage (121K production lines). Remaining ~7.4K lines in hardware-dependent code: V4L2/display (3.8K), neuromorphic/VFIO (2K), test infra (1K). Mock hardware layers or platform-specific harnesses. | D-COV |
| 8 | **OS keyring integration** | `toadstool-common` | File-based credential resolution done (S149). Remaining: D-Bus SecretService (Linux) and macOS Keychain for full OS keyring chain. | D-KEYRING |

## Medium Priority (P2) — Required for fleet / multi-toadStool

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 9 | **Cross-toadStool GPU pooling** | `server/`, `distributed/` | When local GPUs are busy, route dispatch to another toadStool instance via songBird. Needs: GPU availability broadcast, remote dispatch protocol, load-balanced routing. | songBird federation |
| 10 | ~~Thermal safety enforcement~~ | `nvpmu`, `server/hw_learn` | ✅ **Resolved S151**: `check_thermal_for_bdf()` gates `apply` and `auto_init`. `gpu.telemetry` JSON-RPC method returns per-GPU temp/power/safety. `auto_init` captures `RegisterSnapshot` and rolls back on failure. | — |
| 11 | ~~VFIO bind/unbind automation~~ | `nvpmu::vfio_bind` | ✅ **Resolved S151**: `bind_vfio()` / `unbind_vfio()` with safety checks (DRM consumers, IOMMU group). `current_binding()` queries state. `BindResult` tracks previous→current driver. | — |
| 12 | **Multi-GPU init orchestration** | `nvpmu`, `server/handler/` | `compute.hardware.auto_init` handles one GPU. For multi-GPU arrays (4x RTX 3050 on PCIe switch), need parallel init with topology awareness (existing `PcieTopologyGraph`). | Gap 6 (unified discovery) |

## Lower Priority (P3) — Polish and coverage

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 13 | **Conv2D/Pool shader evolution** | barraCuda (transferred) | GPU shaders exist but lack full stride/padding/channels/batch support. | D-S46-001, barraCuda |
| 14 | **E2E integration tests** | `testing/` | Chaos framework exists. Need full end-to-end sovereign pipeline test: WGSL → coralReef compile → toadStool dispatch → GPU result. | Gaps 1, 2 |
| 15 | **Streaming FASTQ/mzML/MS2** | Future | Bio I/O streaming for wetSpring. Deferred. | — |

---

## Recently Resolved (S151)

| Item | Resolution |
|------|-----------|
| Gap 3: Error recovery | `RegisterSnapshot` + `apply_with_recovery` + `NvPmuError::PartialInit` |
| Gap 4: DMA buffers | `nvpmu::dma::DmaAllocator` + `DmaBuffer` (page-aligned, mlock, IOMMU-mapped) |
| Gap 6: Unified PCI discovery | `toadstool_common::pci_discovery` with `PciFilter` and vendor constants |
| Gap 10: Thermal enforcement | `check_thermal_for_bdf()` gates apply/auto_init; `gpu.telemetry` JSON-RPC method |
| Gap 11: VFIO bind/unbind | `nvpmu::vfio_bind` — `bind_vfio()` / `unbind_vfio()` with safety checks |
| `init.rs` → RegisterAccess | All init functions accept `dyn RegisterAccess` (works with Bar0 + VFIO) |
| V4L2 unsafe reduction | 6 `MaybeUninit::zeroed().assume_init()` → `Default::default()` |
| NVK zero-guard extraction | Extracted to `backends/nvk_zero_guard.rs` (smart refactor, not just split) |
| Hardcoded primal knowledge | Removed vendor fallback ports, primal-specific port comments, `"songbird"` in tests |
| mDNS schema constants | `CAPABILITY_PREFIX` / `CAPABILITY_FEATURES_SUFFIX` replace magic strings |
| sysmon clippy debt | Fixed `doc_markdown` and `if_not_else` lint violations |
| F64 throttle magic number | Replaced `8.0` with `DEFAULT_F64_THROTTLE_RATIO` constant |

## Previously Resolved (S150)

| Item | Resolution |
|------|-----------|
| BAR0 requires root | `nvpmu::permissions` udev rules + `setup-gpu-sovereign.sh` |
| VFIO limited to NPU | `nvpmu::vfio::VfioBar0Access` — full VFIO lifecycle for NVIDIA GPUs |
| nvpmu apply_recipe duplication | Delegates to `hw_learn::RecipeApplicator` via `RegisterAccess` |
| hw_learn_apply dry-run only | `compute.hardware.apply` supports `"live": true` |
| Gap 5: knowledge → init | `compute.hardware.auto_init` — auto-detect → best recipe → BAR0 apply |

---

## How to Use This Document

1. Pick the lowest-numbered unresolved gap you can act on.
2. Implement, test, update this doc.
3. Write a wateringHole handoff when crossing primal boundaries.
4. Mark resolved with date and one-line description.

*This is the work list. When it's empty, toadStool's sovereign compute pipeline is complete.*
