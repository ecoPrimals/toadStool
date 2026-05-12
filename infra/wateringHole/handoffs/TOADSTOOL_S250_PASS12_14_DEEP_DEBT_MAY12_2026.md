# ToadStool S250 — Pass 12-14 Execution + Deep Debt Evolution

**Date**: May 12, 2026
**Session**: S250 (continuation of S249+)
**Scope**: primalSpring Pass 12-14 completion + multi-dimensional deep debt audit + evolution

---

## Pass 12 — Phase C Integration (Batches 5-7)

### Batch 5: VFIO Channel Orchestration (68 + 1 files)
- Copied 68 `.rs` files from `coral-driver/src/vfio/channel/` into `toadstool-cylinder`
- Absorbed `hardware_guard.rs` for `GuardedBar` (needed by kepler_channel + pfifo)
- Updated import paths from `nv::vfio_compute::hardware_guard` to `nv::hardware_guard`
- **499 cylinder tests**, zero clippy

### Batch 6: Sovereign Init + Stages + GspBridge Trait
- Absorbed `sovereign_init.rs` (428 lines) and `sovereign_stages.rs` (541 lines)
- Created `nv/falcon_pio.rs` — standalone IMEM/DMEM PIO upload helpers (no GSP deps)
- Created `nv/gsp_bridge.rs`:
  - `GspBridge` trait with 6 methods (ACR boot, falcon boot, FECS boot, PGOB, GR init)
  - `StubGspBridge` default implementation
  - `FalconBootResult`, `AcrBootResult`, `PgobResult` result types
- Rewrote `sovereign_stages.rs` to route all firmware calls through `GspBridge`
- Added `From<Hbm2TrainingError> for SovereignStagesError`
- **516 cylinder tests**, zero clippy

### Batch 7: BAR0 Absorption + GspBridge Boundary
- Absorbed `nv/bar0.rs` (308 lines) — sovereign BAR0 MMIO access
- Re-exported `ApplyError` and `RegisterAccess` from `vfio::device`
- `probe.rs` and `vfio_compute/` remain in coralReef behind `GspBridge` boundary
- **520 cylinder tests**, zero clippy

## Pass 12 — Phase D: Local Dispatch Cutover

- Added `LocalDeviceFactory` type alias + `local_device_factory` field to `DispatchHandler`
- Implemented `try_local_dispatch()` — attempts local execution via `ComputeDevice` trait
- Wired into `submit.rs`: local dispatch attempted before `coral_client` IPC forward
- Graceful fallback: if local dispatch fails or no factory configured, falls through to coral_client
- Added `toadstool-cylinder` dependency to `toadstool-server`

## Pass 14: `toadstool.validate` JSON-RPC Method

- Implemented `WorkloadHandler::validate()` — Tier 2 Science API pre-flight
- Returns: `valid`, `gpu_available`, `precision_tier`, `estimated_dispatch_time_ms`, `warnings`, `required_capabilities`
- Wired in handler (both explicit `toadstool.validate` and short-form `validate`)
- Added to `DIRECT_JSONRPC_METHODS`, `wire_l3.rs` cost table, identity capabilities
- Semantic alias `runtime.workload.validate` in `mappings_extended.rs`
- Updated `METHODS.md` documentation

## Deep Debt Evolution

### Legacy Env Var Deprecation
- Added `#[deprecated(note = "use ...")]` to 13 legacy primal-named constants in `socket_env.rs`:
  - `LEGACY_BEARDOG_SOCKET_ENV`, `LEGACY_SONGBIRD_SOCKET_ENV`, `LEGACY_NESTGATE_SOCKET_ENV`, `LEGACY_SQUIRREL_SOCKET_ENV`
  - `PRIMAL_SOCKET`, `LEGACY_*_URL`, `LEGACY_*_ENDPOINT`, `TOADSTOOL_SONGBIRD_PORT`, `LEGACY_SONGBIRD_AUTH_TOKEN`
- Added `tracing::warn!` deprecation notices for legacy env vars in crypto.rs and family_seed.rs
- Added `#[allow(deprecated)]` with intent comments at 6 backward-compat call sites

### Sentinel / Hardcoding Evolution
- `NO_HISTORY_SENTINEL_SECS = 999` → `Duration::MAX` in `orchestration/policy.rs`
- `DEFAULT_SCAN_SUBNET` ("192.168.1.0") → `default_scan_subnet()` resolves from `TOADSTOOL_SCAN_SUBNET` env
- `StubRuntimeEngine` version string `"stub"` → `"unregistered"`

### Null-Object Sentinel Documentation
- Clarified `StubRuntimeEngine`, `NoopCloudProvider`, `NoopCryptoProvider` as intentional null-object sentinels (not leaked test mocks)
- Updated rustdoc to explain they are complete implementations of the "no provider registered" state

### Multi-Dimensional Audit Results
- Zero `todo!()` / `unimplemented!()` anywhere in codebase
- All unsafe blocks idiomatic: `# Safety` on declarations, `// SAFETY:` at call sites
- No `#[allow(dead_code)]` in production `src/` code
- `cargo deny check bans` passes clean
- `ring` in lockfile but NOT in active dependency tree (stale entry)
- No cross-primal Rust imports

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo clippy --workspace --lib` | **0 warnings** |
| `cargo test --workspace --lib` | **8,809 passed** |
| `cargo deny check bans` | **PASS** |
| toadstool-cylinder tests | **520** |
| JSON-RPC methods | **66** (direct) |

## Files Changed (Key)

| File | Change |
|------|--------|
| `crates/core/cylinder/src/vfio/channel/` (68 files) | Absorbed from coral-driver |
| `crates/core/cylinder/src/nv/gsp_bridge.rs` | New: GspBridge trait |
| `crates/core/cylinder/src/nv/falcon_pio.rs` | New: PIO upload helpers |
| `crates/core/cylinder/src/nv/bar0.rs` | Absorbed from coral-driver |
| `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` | Phase D local dispatch |
| `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` | try_local_dispatch wiring |
| `crates/server/src/pure_jsonrpc/handler/workload.rs` | toadstool.validate |
| `crates/core/common/src/interned_strings/socket_env.rs` | #[deprecated] on 13 legacy constants |
| `crates/runtime/orchestration/src/policy.rs` | Duration::MAX replacing sentinel |
| `crates/auto_config/src/ecosystem_network.rs` | Env-configurable scan subnet |
