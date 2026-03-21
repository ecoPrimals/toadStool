# Sovereign Compute — Remaining Gaps

**Date**: March 21, 2026 — S162
**Purpose**: Single checklist of work remaining before toadStool's sovereign compute pipeline is complete.
**Scope**: toadStool-owned gaps only. barraCuda and coralReef track their own.

---

## Architecture Reference

```
barraCuda (WGSL math)
  → [JSON-RPC] coralReef (WGSL → native SASS/GFX binary)
    → [JSON-RPC] toadStool (dispatch binary → VFIO/DRM → GPU)
      → GPU hardware (any PCIe GPU — NVIDIA, AMD, Intel)
```

**IPC-first** (barraCuda v0.35): All inter-primal communication via JSON-RPC 2.0 at runtime. barraCuda has zero compile-time deps on coralReef or toadStool. VFIO detection is exclusively toadStool's responsibility via `compute.hardware.vfio_devices`.

**coralReef Iteration 43**: PFIFO hardware channel creation via BAR0 MMIO (instance block, V2 MMU page tables, runlist, PCCSR bind/enable). RAMUSERD offsets corrected (GP_GET=0x88, GP_PUT=0x8C). Hardware validation on Titan V: 6/7 VFIO tests pass. Dispatch blocked on USERD_TARGET encoding in runlist entry (DW0 bits [3:2] = 0 VRAM, should be 2 SYS_MEM_COH). One-register fix in coralReef `channel.rs`.

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
| 2 | **VFIO hardware validation** | `nvpmu::vfio` | `VfioBar0Access` implemented. hotSpring validated 6/7 coralReef VFIO tests on Titan V (GV100). BAR0, DMA, upload/readback all pass. Dispatch blocked on coralReef USERD_TARGET encoding fix (one register). toadStool infra is ready. | coralReef fix |
| 3 | ~~Error recovery / rollback~~ | `nvpmu::init` | ✅ **Resolved S151**: `RegisterSnapshot` captures pre-init state, `apply_with_recovery` rolls back on failure, `NvPmuError::PartialInit` reports rollback status. `init.rs` evolved to `dyn RegisterAccess`. | — |
| 4 | ~~DMA buffer support~~ | `nvpmu::dma` | ✅ **Resolved S151**: `DmaAllocator` + `DmaBuffer` ported from akida-driver. Page-aligned, mlock'd, IOMMU-mapped with automatic cleanup. | — |

## High Priority (P1) — Required for multi-arch and production

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 5 | ~~Multi-arch register classification~~ | `hw-learn` | ✅ **Resolved S152**: `GpuGen` enum (Maxwell→Ampere), `classify_register_for_gen()` with per-generation register tables. GA102 + TU102 ranges from envytools. `classify_events` accepts chip hint. | — |
| 6 | ~~Unified PCI discovery~~ | `toadstool-common` | ✅ **Resolved S151**: `pci_discovery::discover_pci_devices()` with `PciFilter` (vendor, class, device IDs). Vendor constants for NVIDIA, Brainchip, AMD, Intel. Shared scanner for GPU + NPU + any accelerator. | — |
| 7 | ~~Test coverage → 90%~~ | Workspace, `testing/mocks/` | ✅ **Resolved S152**: `MockV4l2Device` (synthetic frames, error injection, format switching) + `MockVfioDevice` (BAR0 register sim, access logging, `RegisterAccess` trait impl). Headless CI can now test V4L2 capture and VFIO paths. | — |
| 8 | ~~OS keyring integration~~ | `toadstool-common` | ✅ **Resolved S152**: `os_keyring` module — `secret-tool` (D-Bus SecretService) on Linux, `security` (Keychain) on macOS. Step 2.5 in credential chain. `query`, `store`, `delete`, `available`. | — |

## Medium Priority (P2) — Required for fleet / multi-toadStool

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 9 | ~~Cross-toadStool GPU pooling~~ | `server/`, `cross_gate` | ✅ **Resolved S152**: `RemoteDispatcher` forwards jobs via Unix socket or TCP. `GateGpuInfo.endpoint` for remote gates. `compute_submit` auto-forwards when router selects remote gate, falls back to local on failure. `compute.dispatch.forward` for explicit relay. | — |
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
| Gap 8: OS keyring | `os_keyring` module — Linux D-Bus SecretService (`secret-tool`), macOS Keychain (`security`). Wired as step 2.5 in `resolve_credential` chain |
| Gap 9: Cross-gate GPU pooling | `RemoteDispatcher` (Unix + TCP relay), `GateGpuInfo.endpoint`, auto-forward in `compute_submit`, `compute.dispatch.forward` JSON-RPC method |
| Unsafe evolution (display) | `MmapBuffer::as_slice()` centralizes V4L2 mmap unsafe; `read_frame` no longer has inline unsafe |
| Gap 7: Mock hardware layers | `MockV4l2Device` (4 frame patterns, error injection, capture lifecycle) + `MockVfioDevice` (register sim, access log, `RegisterAccess` impl, error injection) in `crates/testing/src/mocks/` |
| VfioGpuInfo bridge | `compute.hardware.vfio_devices` — discovers VFIO-bound GPUs, exposes descriptors over IPC. toadStool is sole VFIO detection source (barraCuda v0.35 IPC-first removed local detection) |
| ecoprimals-mode CLI | `toadstool mode science/gaming/status` — single-GPU mode switching between display driver and `vfio-pci` (S153) |

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
