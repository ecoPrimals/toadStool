# Sovereign Compute — Remaining Gaps

**Date**: March 12, 2026 — S150
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
| 3 | **Error recovery / rollback** | `nvpmu::init`, `hw-learn` | If a BAR0 init recipe partially applies and the GPU enters a bad state, no rollback mechanism exists. Need: snapshot pre-init register state, rollback on error, exponential backoff on retry. | — |
| 4 | **DMA buffer support** | `nvpmu` | `VfioBar0Access` provides register I/O but not DMA. For sovereign dispatch, toadStool must provide IOMMU-mapped DMA buffer handles that coralReef's transport layer can use for push buffers and data. Pattern exists in `akida-driver::vfio::DmaBuffer`. | — |

## High Priority (P1) — Required for multi-arch and production

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 5 | **Multi-arch register classification** | `hw-learn` | Currently NVIDIA-only. AMD (AMDGPU MMIO) and Intel (Xe MMIO) need register classification, recipe format, and applicator support. | Register documentation |
| 6 | **Unified PCI discovery** | `nvpmu::pci`, `akida-driver` | Two separate PCI scanners: `nvpmu::pci::discover_gpus()` for NVIDIA, `akida_driver` for NPU. Unify into a single scanner that returns typed device descriptors (GPU, NPU, other accelerators). | — |
| 7 | **Test coverage → 90%** | Workspace | ~86% line coverage (121K production lines). Remaining ~7.4K lines in hardware-dependent code: V4L2/display (3.8K), neuromorphic/VFIO (2K), test infra (1K). Mock hardware layers or platform-specific harnesses. | D-COV |
| 8 | **OS keyring integration** | `toadstool-common` | File-based credential resolution done (S149). Remaining: D-Bus SecretService (Linux) and macOS Keychain for full OS keyring chain. | D-KEYRING |

## Medium Priority (P2) — Required for fleet / multi-toadStool

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 9 | **Cross-toadStool GPU pooling** | `server/`, `distributed/` | When local GPUs are busy, route dispatch to another toadStool instance via songBird. Needs: GPU availability broadcast, remote dispatch protocol, load-balanced routing. | songBird federation |
| 10 | **Thermal safety enforcement** | `nvpmu`, `sysmon` | BAR0 init and ongoing compute should respect thermal limits. `toadstool-sysmon::gpu` provides `GpuTelemetry` (temp, power); wire it as a gate before and during dispatch. Throttle or migrate workloads on overheat. | — |
| 11 | **VFIO bind/unbind automation** | `nvpmu::vfio`, `scripts/` | `setup-gpu-sovereign.sh` provides guidance but doesn't auto-bind GPUs to `vfio-pci`. Add safe bind/unbind: check no consumers, unbind current driver, bind `vfio-pci`, verify. Reverse on shutdown. | — |
| 12 | **Multi-GPU init orchestration** | `nvpmu`, `server/handler/` | `compute.hardware.auto_init` handles one GPU. For multi-GPU arrays (4x RTX 3050 on PCIe switch), need parallel init with topology awareness (existing `PcieTopologyGraph`). | Gap 6 (unified discovery) |

## Lower Priority (P3) — Polish and coverage

| # | Gap | Module | Description | Depends On |
|:-:|-----|--------|-------------|------------|
| 13 | **Conv2D/Pool shader evolution** | barraCuda (transferred) | GPU shaders exist but lack full stride/padding/channels/batch support. | D-S46-001, barraCuda |
| 14 | **E2E integration tests** | `testing/` | Chaos framework exists. Need full end-to-end sovereign pipeline test: WGSL → coralReef compile → toadStool dispatch → GPU result. | Gaps 1, 2 |
| 15 | **Streaming FASTQ/mzML/MS2** | Future | Bio I/O streaming for wetSpring. Deferred. | — |

---

## Recently Resolved (S150)

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
