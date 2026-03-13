# Sovereign Compute — Remaining Gaps

**Date**: March 13, 2026 — S152
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
| **Sovereign** | none (VFIO) | `VfioBar0Access` | coralReef coral-driver | `compute.dispatch.submit` ready, MSI-X + huge page DMA |
| **nouveau** | nouveau | sysfs `resource0` | kernel | Functional — BAR0 GR init proven |
| **nvidia** | nvidia UVM | n/a | UAPI | Functional — CTXNOTVALID resolved |

---

## Critical Path (P0) — Blocks sovereign dispatch

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 1 | ~~Dispatch client~~ | `server/handler/dispatch` | ✅ **Resolved S152**: `compute.dispatch.submit` accepts compiled GPU binary, resolves BDF (prefers VFIO), checks thermal, forwards to coralReef. Also: `status`, `result`, `capabilities`. `SOVEREIGN_BINARY_PIPELINE = true`. | — |
| 2 | **VFIO hardware validation** | `nvpmu::vfio` | `VfioBar0Access` is implemented but untested on real VFIO-bound GPU hardware. Need a test rig with `vfio-pci` bound GPU + validation script. | Hardware access |
| 3 | ~~Error recovery / rollback~~ | `nvpmu::init` | ✅ **Resolved S151**: `RegisterSnapshot` captures pre-init state, `apply_with_recovery` rolls back on failure, `NvPmuError::PartialInit` reports rollback status. `init.rs` evolved to `dyn RegisterAccess`. | — |
| 4 | ~~DMA buffer support~~ | `nvpmu::dma` | ✅ **Resolved S151**: `DmaAllocator` + `DmaBuffer` ported from akida-driver. Page-aligned, mlock'd, IOMMU-mapped with automatic cleanup. | — |

## High Priority (P1) — Required for multi-arch and production

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 5 | ~~Multi-arch register classification~~ | `hw-learn` | ✅ **Resolved S152**: `GpuGen` enum (Maxwell→Ampere), `classify_register_for_gen()` with per-generation register tables. GA102 + TU102 ranges from envytools. `classify_events` accepts chip hint. | — |
| 6 | ~~Unified PCI discovery~~ | `toadstool-common` | ✅ **Resolved S151**: `pci_discovery::discover_pci_devices()` with `PciFilter` (vendor, class, device IDs). Vendor constants for NVIDIA, Brainchip, AMD, Intel. Shared scanner for GPU + NPU + any accelerator. | — |
| 7 | **Test coverage → 90%** | Workspace | ~86% line coverage (121K production lines). Remaining ~7.4K lines in hardware-dependent code: V4L2/display (3.8K), neuromorphic/VFIO (2K), test infra (1K). Mock hardware layers or platform-specific harnesses. | D-COV |
| 8 | **OS keyring integration** | `toadstool-common` | File-based credential resolution done (S149). Remaining: D-Bus SecretService (Linux) and macOS Keychain for full OS keyring chain. | D-KEYRING |

## Medium Priority (P2) — Required for fleet / multi-toadStool

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 9 | **Cross-toadStool GPU pooling** | `server/`, `distributed/` | When local GPUs are busy, route dispatch to another toadStool instance via songBird. Needs: GPU availability broadcast, remote dispatch protocol, load-balanced routing. | songBird federation |
| 10 | ~~Thermal safety enforcement~~ | `nvpmu`, `server/hw_learn` | ✅ **Resolved S151**: `check_thermal_for_bdf()` gates `apply` and `auto_init`. `gpu.telemetry` JSON-RPC method returns per-GPU temp/power/safety. `auto_init` captures `RegisterSnapshot` and rolls back on failure. | — |
| 11 | ~~VFIO bind/unbind automation~~ | `nvpmu::vfio_bind` | ✅ **Resolved S151**: `bind_vfio()` / `unbind_vfio()` with safety checks (DRM consumers, IOMMU group). `current_binding()` queries state. `BindResult` tracks previous→current driver. | — |
| 12 | ~~Multi-GPU init orchestration~~ | `nvpmu`, `server/handler/` | ✅ **Resolved S152**: `compute.hardware.auto_init_all` — parallel `spawn_blocking` per GPU, thermal checks, rollback, per-GPU succeeded/failed/skipped reporting. | — |

## Lower Priority (P3) — Polish and coverage

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 13 | **Conv2D/Pool shader evolution** | barraCuda (transferred) | GPU shaders exist but lack full stride/padding/channels/batch support. | D-S46-001, barraCuda |
| 14 | **E2E integration tests** | `testing/` | Chaos framework exists. Need full end-to-end sovereign pipeline test: WGSL → coralReef compile → toadStool dispatch → GPU result. | Gap 2 |
| 15 | **Streaming FASTQ/mzML/MS2** | Future | Bio I/O streaming for wetSpring. Deferred. | — |

---

## Recently Resolved (S152)

| Item | Resolution |
|------|-----------|
| Gap 1: Dispatch client | `compute.dispatch.submit/status/result/capabilities` — accepts compiled binaries, forwards to coralReef, thermal-gated, BDF auto-detect (VFIO-preferred). `SOVEREIGN_BINARY_PIPELINE = true` |
| Gap 5: Multi-arch classifier | `GpuGen` enum + `classify_register_for_gen()` — Volta/Turing/Ampere/Pascal/Maxwell register tables from envytools. `classify_events` accepts chip hint |
| Gap 12: Multi-GPU init | `compute.hardware.auto_init_all` — parallel `spawn_blocking` per GPU, per-GPU thermal check/rollback, succeeded/failed/skipped reporting |
| Huge page DMA | `DmaAllocator::allocate_huge()` with `HugePageSize::Huge2M`/`Huge1G` via `mmap_anonymous` + `MAP_HUGETLB`. `supports_huge_pages()` checks sysfs |
| MSI-X / eventfd | `VfioMsixInterrupt::configure()` — wires eventfd to MSI-X vector via `VFIO_DEVICE_SET_IRQS`. `wait()` and `wait_timeout()` for completion |
| GPU reset / power | `GpuPowerController` — `power_state()`, `reset()` (FLR), `power_on()`/`power_suspend()`, `available_reset_methods()`, `power_limit_uw()` |
| `extern "C"` elimination | `nouveau_drm.rs` FFI ioctl replaced with rustix `DrmIoctl` + `ioctl_nr_to_opcode()`. Zero `extern "C"` in workspace |

## Previously Resolved (S151)

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
