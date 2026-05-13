# Diesel Engine Migration — S252 (May 13, 2026)

## Scope

Diesel Engine Migration Batches 1–2 + deep debt sweep + test performance optimization.

## What shipped

### Batch 1: `device.swap` + `device.warm_catch` + socket layout

- **`device.swap`** JSON-RPC handler — swap any GPU to arbitrary target personality via `SwapOrchestrator`. Returns `DeviceSwapResult` with per-step timing.
- **`device.warm_catch`** JSON-RPC handler — detect warm GPU state via PMC_ENABLE sysfs config read. Returns `warm_detected`, `pmc_enable`, `pmc_popcount`, `resource0_exists`.
- **`TOADSTOOL_RUN_DIR`** env var and `run_dir()` helper for `/run/toadstool/` socket tree layout.
- **`read_pci_config_u32()`** — safe sysfs config space reader (no `unsafe`).
- **`DeviceSwapResult`**, **`DeviceSwapStep`** — structured types for swap lifecycle reporting.
- Routes: direct + `dispatch_by_impl_name` + semantic aliases (`device.swap`, `device.warm_catch`).
- Wire L3 cost entries, `capabilities.list` updates, 7 new tests.

### Batch 2: MMIO + Falcon RPCs (6 methods)

- **New module**: `crates/server/src/pure_jsonrpc/handler/mmio.rs`
- **`mmio.read32`** — read 32-bit BAR0 register via `SysfsBar0`.
- **`mmio.write32`** — write 32-bit BAR0 register via new `SysfsBar0Rw`.
- **`mmio.batch`** — batch read/write with per-op results.
- **`mmio.pramin.read32`** — PRAMIN window read (BAR0+0x700000).
- **`mmio.bar0.probe`** — chip identity (boot0, chip_id, vendor, pmc_enable, responsive).
- **`mmio.falcon.status`** — falcon microcontroller registers (cpuctl, mailbox0/1, os, bootvec, hwcfg, halted). Supports pmu/fecs/gpccs/sec2 engines.
- **`SysfsBar0Rw`** — new read-write sysfs BAR0 mmap type in `toadstool-cylinder`.
- Semantic aliases: `ember.mmio.read32`, `ember.mmio.write32`, `ember.mmio.batch`, `ember.mmio.pramin.read32`, `ember.bar0.probe`, `ember.falcon.status`.
- Wire L3 costs, `capabilities.list` mmio group, 12 new tests.

### Deep Debt Sweep

- **7 `#[allow(deprecated)]` → `#[expect(deprecated, reason)]`**: `fallback.rs`, `env.rs`, `primal_discovery_complete/mod.rs`, `discovery.rs`, `format.rs`, `execution.rs`, `scheduler.rs`.
- **Audit confirmed**: 0 files >800L, all unsafe SAFETY-documented, all mocks #[cfg(test)]-gated or documented null-objects, hardcoded values centralized in config modules.

### Test Performance

- **5s timeout test** (`test_with_default_timeout_failure`) → 10ms (`with_timeout_duration`).
- **`OnceLock` capability cache** in `query_local_capabilities()` — GPU enumeration via wgpu runs once per process instead of per-call.
- **Testing crate**: 5.01s → 0.31s.

## Quality gates

| Gate | Result |
|------|--------|
| `cargo clippy --workspace` | 0 warnings |
| `cargo test --workspace --lib` | 8,827 passed, 0 failed |
| `cargo deny check bans` | clean |
| JSON-RPC methods (direct) | 74 |

## Files changed

### New files
- `crates/server/src/pure_jsonrpc/handler/mmio.rs`

### Modified files
- `crates/server/src/glowplug_client.rs` — `swap()`, `warm_detect()`, `run_dir()`, `read_pci_config_u32()`, `DeviceSwapResult`, `DeviceSwapStep`
- `crates/server/src/pure_jsonrpc/handler/mod.rs` — mmio module, device.swap/warm_catch routing
- `crates/server/src/pure_jsonrpc/handler/core/mod.rs` — 8 new DIRECT_JSONRPC_METHODS entries
- `crates/server/src/pure_jsonrpc/handler/core/identity.rs` — mmio capability group
- `crates/server/src/pure_jsonrpc/handler/core/wire_l3.rs` — 8 new cost entries
- `crates/core/toadstool/src/semantic_methods/mappings_extended.rs` — 8 new semantic aliases
- `crates/core/cylinder/src/vfio/sysfs_bar0.rs` — `SysfsBar0Rw` (read-write BAR0 mmap)
- `crates/core/common/src/interned_strings/socket_env.rs` — `TOADSTOOL_RUN_DIR`
- `crates/server/src/unibin/capabilities.rs` — `OnceLock` cache
- `crates/testing/src/helpers/timeout.rs` — 5s → 10ms timeout test
- 7 files: `#[allow(deprecated)]` → `#[expect(deprecated, reason)]`
- Root docs: `README.md`, `CONTEXT.md`

## Remaining batches (Diesel Engine Migration)

- **Batch 3**: Real `OwnedFd` in `VfioResourceHandle`
- **Batch 4**: `ember.vfio_fds` fd-passing endpoint (SCM_RIGHTS)
- **Batch 5**: Warm-FECS pipeline end-to-end
- **Batch 6**: CLI parity + fleet artifact + docs
