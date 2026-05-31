# ToadStool Deep Debt Evolution Pass — Handoff

**Date**: May 31, 2026
**From**: toadStool (biomeGate)
**To**: primalSpring (upstream audit), all downstream consumers
**Session**: S282+ (post-S282 deep debt evolution pass)

---

## Summary

Systematic 6-wave deep debt evolution across toadStool. All waves complete, build clean, zero clippy warnings, ~23,770 tests passing.

---

## Wave 1: Hardcoded Paths → Capability-Based Discovery

### 1A. Data Paths
- Added `data_dir()` and `data_subdir()` to `linux_paths.rs`
- Consolidated 8 hardcoded `/var/lib/toadstool/*` paths across 7 files
- All now respect `TOADSTOOL_DATA_DIR` environment variable

### 1B. Sysfs Paths
- Created `sysfs_paths` module in `toadstool-common` (shared by all crates)
- Updated ~50+ hardcoded `/sys/bus/pci`, `/sys/module`, `/sys/class` paths
- Affected crates: cylinder, server, cli, glowplug, nvpmu, ember
- All now respect `TOADSTOOL_SYSFS_ROOT` for container/test environments
- Added helpers: `sysfs_pci_driver_new_id()`, `sysfs_pci_driver_remove_id()`, `sysfs_module_parameter()`

### 1C. Chip/Driver Discovery
- `capture.rs`: chip name derived from BOOT0 + `detect_chip()` (fallback: `gv100`)
- `config.rs`: DKMS version discovered from installed modules (fallback: `470.256.02`)
- `discover.rs`: PTOP offsets wired through `GenerationProfile` (new fields: `ptop_device_info_base`, `runlist_pbdma_map_base`)

## Wave 2: Large File Refactoring

| File | Before | After |
|------|--------|-------|
| `rm_trigger.rs` | 1,113L monolith | `main.rs` + `rm_ioctl.rs` + `rm_object_tree.rs` |
| `pfifo/mod.rs` | 916L | `init.rs` + `channel.rs` + `runlist.rs` + hub |
| `dispatch/mod.rs` | 903L | `state.rs` + `device.rs` + hub |
| `reagent.rs` | 893L | `catalog.rs` + `vram_capture.rs` + orchestrator |
| `generation.rs` | 862L | 9 per-arch modules (kepler→blackwell) + hub |

## Wave 3: Unsafe Code Tightening

- **3A**: Consolidated mmap/volatile patterns from 6 files → `hw-safe` types (`DeviceMmap`, `VolatileMmio`, `SafeMmapRegion`)
- **3B**: Audited all 18 `unsafe impl Send/Sync` blocks across 10 types — expanded SAFETY documentation

## Wave 4: Orphan Crate Wiring & Cleanup

- `security-sandbox`/`security-policies` wired behind `sandbox` feature on server
- `toadstool-auto-config` wired to CLI behind `zero-config` feature
- `runtime-python`/`burn-inference` archived (workspace-excluded)
- Commented-out Cargo.toml blocks cleaned (server, edge, distributed, root)
- Dead code removed: `exclude_bdf()`, Phase C imports, superseded ELF helpers

## Wave 5: Production Stub Evolution

- `StubRuntimeEngine`: runtime backend probing (WGPU/VFIO/WASM detection)
- `detect_chip()`: now returns `ChipDetection` enum (Nvidia/AmdPresent/NotFound)
- `NoopGspBridge`: enhanced fallback logging with operator guidance

## Wave 6: Docs Refresh

- Session tags updated to S282
- Method count verified at 88
- Test metrics: 23,770+ workspace, 9,182+ lib-only
- `SERVER_METHODS.md` verified against dispatch registry
- FHE integration test archived

---

## For primalSpring Audit

### New public APIs
- `toadstool_common::sysfs_paths::*` — sysfs path helpers (shared by all crates)
- `linux_paths::data_dir()`, `data_subdir()` — data directory helpers
- `GenerationProfile.ptop_device_info_base`, `.runlist_pbdma_map_base`
- `reagent::discover_chip_from_bdf()`, `discover_nvidia_driver_version()`
- `kmod::discover_dkms_version()`
- `sovereign_stages::ChipDetection` enum

### Dependency changes
- `toadstool-hw-safe` added to cylinder Cargo.toml
- `security-sandbox`, `security-policies` optional on server
- `toadstool-auto-config` optional on CLI

### Breaking changes
- `REAGENT_STORE_DIR` constant → `reagent_store_dir()` function
- `CRASH_REPORT_DIR` constant → `crash_report_dir()` function
- `pfifo::init_pfifo_engine` moved to `pfifo::init::init_pfifo_engine` (re-exported)
- `dispatch/mod.rs` split — device methods now `pub(crate)` in `device.rs`

### Gaps for upstream review
- `sysfs_paths` only covers paths used in production; runtime/edge and akida-driver have additional `/sys/class/*` paths using `sysfs_join()` which could benefit from dedicated helpers
- DKMS version discovery scans `/var/lib/dkms/` — may need container-awareness
- AMD `detect_chip()` probes GRBM_STATUS at 0x8010 — needs hardware validation on AMD GPUs
