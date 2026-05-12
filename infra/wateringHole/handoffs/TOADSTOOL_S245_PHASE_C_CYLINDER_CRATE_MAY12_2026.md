# toadStool S245 — Phase C Begins: toadstool-cylinder Crate

**Date**: May 12, 2026
**Sprint**: Compute Trio Evolution Sprint 4
**Status**: Phase C foundation absorbed, deep debt sweep complete

---

## Summary

Phase C absorption of `coral-driver` hardware lifecycle modules into toadStool initiated.
Created `toadstool-cylinder` crate as the sovereign hardware driver layer. Foundation layer
(DRM, sysfs, hardware capabilities, error types, `ComputeDevice` trait) absorbed and tested.
Parallel deep debt sweep: last production `println!` migrated, 10 more `Duration` constants
extracted.

---

## Phase C Absorption Progress

### Absorbed (S245)

| Module | Lines | Description |
|--------|------:|-------------|
| `drm.rs` | 783 | DRM ioctl interface, `MappedRegion`, render node enumeration |
| `linux_paths.rs` | 364 | Sysfs/procfs path helpers (`TOADSTOOL_*` env vars) |
| `hardware.rs` | 214 | `Vendor`, `MemoryType`, `WaveSize`, `CompletionStyle`, `HardwareCapabilities` |
| `error/mod.rs` | 412 | `DriverError`, `DriverResult`, trait conversions |
| `error/vfio.rs` | 523 | `PciDiscoveryError`, `ChannelError`, `DevinitError`, `SovereignStagesError` |
| `lib.rs` types | — | `BufferHandle`, `MemoryDomain`, `DispatchDims`, `ShaderInfo`, `ComputeDevice` trait |

### Remaining (Next Sessions)

| Module | Est. Lines | Priority |
|--------|----------:|----------|
| `amd/` (gem, pm4, ioctl, generation, shader_binary) | ~2,800 | P1 |
| `nv/` (hardware subset — bar0, pushbuf, qmd, probe, vfio_compute) | ~4,000+ | P1 |
| `vfio/` (device, dma, pci_discovery, isolation, channel) | ~10,000+ | P1 |
| `mmio` + `mmio_region` (private modules) | ~400 | P1 (needed by amd/nv) |
| `coral-gpu/pcie.rs` (probe_pcie_topology) | 124 | P2 (has coral-reef coupling) |

---

## Deep Debt Sweep

### println!→tracing Migration

- **`testing/src/properties/runner.rs`**: Last production `println!` in library code.
  Migrated to `tracing::warn!` with structured fields (`?input`, `%error`, `test_cases_run`).
- **Status**: Zero production `println!` remaining in library code.

### Duration Constant Extraction (10 new constants)

| File | Constants |
|------|-----------|
| `core/common/src/infant_discovery/config.rs` | `DEFAULT_CACHE_TTL_SECS`, `DEFAULT_DISCOVERY_TIMEOUT_SECS`, `DEFAULT_RETRY_DELAY_SECS` |
| `runtime/gpu/src/config.rs` | `DEFAULT_DISCOVERY_TIMEOUT_SECS`, `DEFAULT_MAX_EXECUTION_TIME_SECS`, `DEFAULT_MONITORING_INTERVAL_SECS`, `DEFAULT_METRICS_RETENTION_SECS`, `DEFAULT_REBALANCE_INTERVAL_SECS`, `DEFAULT_CACHE_TTL_SECS`, `DEFAULT_CHECKPOINT_INTERVAL_SECS` |

---

## Quality Metrics

| Metric | Value |
|--------|-------|
| **Tests (lib-only)** | 8,349 (up from 8,289) |
| **Clippy warnings** | 0 |
| **Production `println!`** | 0 |
| **Production panics** | 0 |
| **Cylinder crate tests** | 60 |

---

## Key Decisions

1. **Environment variable evolution**: `CORALREEF_SYSFS_ROOT` → `TOADSTOOL_SYSFS_ROOT` with
   backward compatibility fallback. Allows gradual migration without breaking existing deployments.

2. **`dead_code` suppression**: `MappedRegion` and `platform_overflow()` are suppressed with
   `#[allow(dead_code, reason = "...")]` because AMD/NV backends that use them are not yet absorbed.

3. **Wire-only principle**: `toadstool-cylinder` has zero dependency on any coral-reef crate.
   All inter-primal communication will be JSON-RPC IPC.

4. **`coral-gpu/pcie.rs` coupling**: Has hard coupling to `coral_reef::GpuTarget`. Will need
   adapter type or sysfs-only extraction when absorbed. Deferred to P2.

---

## Next Steps (S246+)

1. **Absorb `mmio` + `mmio_region`** (private modules needed by amd/nv backends)
2. **Absorb `amd/`** (GEM, PM4, ioctl, fence sync, CS submit)
3. **Absorb `nv/` hardware subset** (BAR0, pushbuf, QMD, probe, vfio_compute)
4. **Absorb `vfio/`** (device open, BAR mapping, DMA, PCI discovery, isolation, channel)
5. **Create hardware mocks** (MockVfioDevice, MockPciSysfs) for coverage push
6. **Coverage push**: 83.6% → 88% by end of Phase C
