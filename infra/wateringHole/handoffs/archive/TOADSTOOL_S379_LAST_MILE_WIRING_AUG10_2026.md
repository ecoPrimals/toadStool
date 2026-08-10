# ToadStool S379 — Last-Mile Wiring + Archaic Code Excision

**Date**: Aug 10, 2026
**Sprint**: S379
**Wave**: 157g+ ENMESH continuation

## Changes

### Excised (~735 LOC removed)

- `crates/cli/src/executor/display.rs` (~260 LOC) — test-only duplicate of `display_ops.rs`. Deleted.
- `crates/cli/src/ecosystem/discovery.rs` (~475 LOC) — duplicate env/config/mDNS stack. Only `verify_service()` retained (moved to `integrator_impl.rs`).
- `crates/cli/tests/discovery_coverage_tests.rs` — tests for excised discovery. Deleted.
- `component-model = []` feature removed from `runtime/wasm` Cargo.toml. Test stubs collapsed with `#[ignore]`.

### Wired (last-mile gaps closed)

- **WASM workload conversion** — `conversion.rs` now handles `ExecutionSpec::Wasm` and `ExecutionSpec::Container` variants (was: catch-all reject).
- **Runtime hint inference** — `start_primal` and `start_service` call `infer_runtime_type(&workload)` instead of hardcoding `RuntimeType::Native`.

### Evolved

- **`tail_log_file`** — `tokio::fs::File` + async `BufReader` → `std::io::BufReader` + `tokio::time::sleep` polling. No `tokio/fs` feature needed.
- **`toadstool-core` re-exports** — `WorkloadSpec` and `WorkloadType` now re-exported from crate root. Dead `workload_tests` module removed. Missing `tempfile` dev-dependency added.

## Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace --lib` — all pass

## Remaining Last-Mile Gaps

1. **`AkidaNpuDispatch` adapter** — bridge between `toadstool-core::NpuDispatch` and `akida-driver` hardware layer
2. **`ModuleCache` bypass** — WASM `ModuleCache` is functional but not wired into `ModuleExecutor` load path
3. **Server NPU handler** — no `npu.*` JSON-RPC methods registered
4. **Server WASM engine registration** — `runtime_engines` map not populated at startup
5. **Discovery dedup** — `service_discovery` (12 callers) should incrementally route through `CapabilityProvider`
