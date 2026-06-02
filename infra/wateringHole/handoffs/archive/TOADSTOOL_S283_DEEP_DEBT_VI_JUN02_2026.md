# toadStool S283 — Deep Debt Evolution VI

**Date**: June 2, 2026
**Session**: S283
**Gate**: biomeGate (Threadripper 3970X, Titan V + K80, 256GB)
**Status**: COMPLETE — All quality gates green

## What Changed

### Large File Refactoring (6 files, >800L)

| File | Before | After |
|------|--------|-------|
| `pmu_investigate.rs` | 1035L | `mod.rs` + `phase_c.rs` |
| `handler/mod.rs` | 984L | extracted `ember.rs` (→688L) |
| `kmod_build.rs` | 898L | `mod.rs` + `build.rs` + `load.rs` |
| `kernel_health.rs` | 868L | 6 submodules (paths/elf/autoconf/probe/reference/repair) |
| `nv_gsp_bridge.rs` | 835L | `mod.rs` + `boot.rs` + `bridge_impl.rs` |
| `sovereign_stages/mod.rs` | 936L | extracted `experiment.rs` |

### Production Mock Isolation
- `server/mocks.rs` → `#[cfg(test)]`
- `tarpc_server/executor/test_doubles.rs` → `#[cfg(test)]`
- `distributed/cloud/test_mocks.rs` → `#[cfg(test)]`
- `integration/primals/mock_primal.rs` → `#[cfg(test)]`

### Capability-Based Evolution
- `bear_dog/` module → `security_client/` (capability naming)
- `CORALREEF_*` env fallbacks: fully removed (6 constants)
- `visualization_client.rs` → capability-based shader discovery
- `ipc_watch.rs` → `discovery_available` (was `songbird_available`)

### Unsafe Evolution
- MMIO in `capture.rs`, `nouveau_oracle.rs` → hw-safe types
- `ffi_loader.rs` → all unsafe centralized with SAFETY docs

### Unwrap Elimination
- 167 bare `.unwrap()` → `.expect()` / `?` across 7 files
- Tests evolved to `-> ToadStoolResult<()>` + `?` pattern

### Env Centralization (~97% → ~98%)
- +15 `socket_env` constants (network, capability ports, hardware)

### Deprecated Dead Code Removal
- `StubGspBridge`, `NestGateResult`, `NestGateMount` type aliases
- `initialize_nestgate_connection`, `with_squirrel`, `initialize_squirrel_connection`
- 4 deprecated items with zero callers removed (S283 cleanup pass)

## Verification
- `cargo check --workspace`: PASS
- `cargo clippy --workspace`: ZERO warnings
- `cargo test --workspace --lib`: ZERO failures
- Environment-dependent test fix: `test_capability_to_biomeos_fallback_crypto` isolated with `temp_env`
- Borrow-after-move fix: `write_toml_fixture`/`write_bytes_fixture` in workload tests

## Gaps for Upstream

| Gap | Owner | Notes |
|-----|-------|-------|
| PBDMA runlist registration | toadStool + primalSpring | Blocks Titan V execution (Jun 1 RCA) |
| FECS golden context reload | toadStool | Requires PBDMA first |
| Test coverage 83%→90% | toadStool | Hardware-dependent paths need mock infra |
| Phase D mixed command streams | toadStool | Blocked on PBDMA |
| `crates/runtime/python/` | Squirrel team | pyo3 removed; fossil candidate |
| `crates/ml/burn-inference/` | barraCuda team | Workspace-excluded; fossil candidate |
