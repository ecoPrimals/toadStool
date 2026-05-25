# ToadStool S247 — Phase C Batch 3: NVIDIA Backend Absorption + Deep Debt

**Date**: May 12, 2026
**Session**: S247
**Phase**: C (coral-driver hardware absorption into toadstool-cylinder)

---

## Summary

Absorbed the complete NVIDIA hardware module suite from `coral-driver` into
`toadstool-cylinder`. Parallel deep debt sweep extracted ~30 hardcoded
Duration literals to named constants across 15 files.

## What Was Absorbed

### NV Identity (`nv/identity/` — 6 files)
- `gpu_identity.rs` — PCI vendor/device ID tables, SM architecture mapping
- `chip_map.rs` — boot0-to-SM translation, chip name/variant lookup
- `constants.rs` — PCI vendor ID constants
- `firmware.rs` — `/lib/firmware/nvidia/` inventory, nouveau firmware checks
- `sysfs.rs` — sysfs-based GPU identity probing
- `tests.rs` — identity module test suite

### NV Generation (`nv/generation.rs`)
- Per-GPU-generation profiles: QmdVersion, LaunchMethod, CompletionStrategy,
  BootStrategy, NctaidSource, PageTableFormat
- SM50 through SM120+ (Volta through Blackwell)

### NV Pushbuf (`nv/pushbuf.rs`)
- PushBuf command stream builder
- Compute class IDs (Volta/Turing/Ampere+)
- Method constants (SET_OBJECT, PCAS/PCAS2, memory windows)

### NV Ioctl (`nv/ioctl/` — 4 files)
- `mod.rs` — Nouveau DRM ioctl numbers, NVIF routes/classes
- `gem.rs` — GEM create/mmap, pushbuf submit
- `new_uapi.rs` — VM init/bind/unmap, exec submit with syncobj
- `diag.rs` — Diagnostic re-exports (identity, firmware)

### NV QMD (`nv/qmd/` — 10 files)
- `mod.rs` — `build_qmd_for_sm`, standard CBUF encoding, driver constants
- `types.rs` — CbufBinding, QmdParams, DRIVER_CBUF_INDEX contract
- `field.rs` — Bitfield writers (qmd_set_field, qmd_set_field_dyn)
- `build.rs`, `sm_config.rs` — Per-SM-version QMD construction
- `v21_v22.rs`, `v23.rs`, `v30.rs`, `v50.rs` — Version-specific builders
- `tests.rs` — QMD test suite

### VA Constants (`nv/mod.rs`)
- `NV_KERNEL_MANAGED_ADDR` (0x80_0000_0000) — VM_INIT kernel region
- `NV_USER_VA_START` (0x1_0000_0000) — Userspace VA heap base

## What Was Deferred

- `nv/bar0.rs` — Depends on `gsp::RegisterAccess` (firmware, stays coralReef)
- `nv/probe.rs` — Depends on `gsp::GrFirmwareBlobs` (firmware, stays coralReef)
- `nv/kepler_falcon.rs` — Falcon microcontroller (firmware-adjacent)
- `nv/fecs_init.rs` — FECS GR initialization (gsp-dependent)
- `nv/vfio_compute/` — Full VFIO NV device (Phase C later batch with VFIO)
- `nv/mod.rs::NvDevice` — Full device orchestration (depends on probe/bar0)

## Deep Debt Sweep

~30 hardcoded Duration literals extracted to named constants:

| File | Constants Extracted |
|------|--------------------|
| `discovery_defaults.rs` | 8 (default/prod/dev/test timeouts + cache TTLs) |
| `primal_discovery.rs` | 2 (cache TTL, health check interval) |
| `capability_discovery/types.rs` | 1 (discovery timeout) |
| `runtime_discovery/service.rs` | 1 (cache TTL) |
| `primal_discovery_mdns.rs` | 2 (prod/test timeouts) |
| `backends.rs` | 4 (mDNS probe + TCP connect timeouts) |
| `modern_utils.rs` | 2 (backoff initial + max) |
| `glowplug/swap.rs` | 1 (quiescence timeout) |
| `launcher.rs` | 1 (startup poll interval) |
| `wasm.rs` + `native.rs` | 2 (execution timeout) |
| `runtime_bridge.rs` | 5 (legacy timing + poll interval) |
| `wasm/config.rs` | 1 (module cache TTL) |
| `python/lib.rs` | 1 (execution timeout) |
| `monitoring/types.rs` | 1 (metrics retention) |
| `client/core.rs` | 2 (wait timeout + poll interval) |
| `ecosystem_network.rs` | 2 (TCP probe timeouts) |
| `config_builder.rs` | 1 (discovery timeout) |
| `crypto validators` | 1 (max proof age) |

## Quality Gates

- **Clippy**: Zero warnings (`-D warnings`), full workspace
- **Tests**: 8,583 lib-only passing (up from 8,430)
- **Cylinder tests**: 294 (up from 141 — 153 new from NV modules)
- **Unsafe**: All blocks SAFETY-documented
- **Production mocks**: All gated behind `test-mocks` feature

## Key Decisions

1. **bar0/probe deferred**: These modules depend on `gsp` firmware structures.
   The split plan says "Do NOT absorb: gsp/". They'll come when/if we decide
   to absorb gsp or create a trait boundary for firmware access.
2. **QMD contract**: QMD encoding absorbs into toadStool, but `DRIVER_CBUF_INDEX`
   and 16-byte stride must stay synchronized with coralReef's `func_builtins.rs`.
   This is a documented dual-maintenance contract, not a Rust crate dependency.
3. **NvDevice orchestration**: Not absorbed yet because it wires together
   probe + bar0 + GEM + pushbuf + QMD into a full `ComputeDevice` impl.
   Once bar0/probe boundary is resolved, NvDevice can follow.

## Next Steps (S248+)

1. **Absorb VFIO modules** — device open, BAR mapping, DMA, PCI discovery,
   isolation, channel setup from `coral-driver/src/vfio/`
2. **Absorb `pcie.rs`** from `coral-gpu` — `probe_pcie_topology()`, sysfs scan
3. **Coverage push** — MockVfioDevice/MockPciSysfs for hardware-gated test paths
4. **Phase D prep** — Wire `compute.dispatch.submit` through absorbed driver layer
