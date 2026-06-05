# toadStool S293 — Deep Debt X: tarpc Gating + Unwrap Purge + Cylinder Split + LEGACY Tracing

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Deep debt pass complete. Software-only.

## Changes

### tarpc dependency cleanup

- **runtime/display**: Removed completely unused `tarpc` dependency. All IPC was already JSON-RPC over Unix/TCP sockets. README updated.
- **integration/protocols**: `tarpc` made `optional = true`. Gated behind `tarpc-transport` feature. `tarpc_service` module only compiled when feature enabled.
- **server/Cargo.toml**: Adds `tarpc-transport` feature to protocols dependency.
- **client/Cargo.toml**: Client's `tarpc` feature now includes `protocols/tarpc-transport`.

### Production unwrap/expect purge (zero remaining)

| File | Fix |
|------|-----|
| `runtime/gpu/cpu_resource.rs` | `expect("zero-thread pool")` → non-panicking `build_last_resort_degraded_pool()` cascade |
| `cylinder/bin/rm_trigger/rm_ioctl.rs` | `try_into().unwrap()` → infallible `[buf[8], buf[9], ...]` array construction |
| `neuromorphic/akida-driver/mmio.rs` | Removed deprecated panicking `read32`/`write32`/`read64`/`write64` (only `try_*` remain) |
| `neuromorphic/akida-models/bin/model_zoo.rs` | `set_global_default().expect()` → `let _ = set_global_default()` |
| `neuromorphic/neurobench-runner/bin/neurobench.rs` | Same tracing fix |
| `neuromorphic/akida-reservoir-research/bin/test_state_extraction.rs` | `unwrap()` → `let Some(...) else { return Ok(()) }` |

### Cylinder module split: mmu_oracle/capture.rs (795L → 4 modules)

- `capture/mod.rs` (175L) — public API, orchestration
- `capture/bar0.rs` (200L) — BAR0 MMIO accessor, PRAMIN window
- `capture/types.rs` (215L) — serializable capture types + tests
- `capture/walk.rs` (195L) — PCCSR channel scan, PD3→PT walk

### LEGACY env deprecation tracing

23 `LEGACY_*` env reads across 5 files now emit `tracing::warn!` when the legacy variable is used as fallback. No behavioral change — just visibility for operators migrating to capability-based discovery.

## Metrics

- **8,895** lib tests passed (default), 0 failed
- Full workspace clippy `-D warnings` clean
- 19 files changed, net -525 lines

## Remaining Debt

1. **CallerContext full threading** — P0 security; ionic token → identity/envelope
2. **LEGACY env removal** — 23 reads now have deprecation tracing; next step is staged removal
3. **Files 750L+** — `bar_cartography.rs` (782L), `pmu.rs` (771L), `registers.rs` (766L) in cylinder
4. **tarpc in server defaults** — still in `default = ["gpu-discovery", "tarpc", "btsp"]`
5. **wgpu always-on in runtime/adaptive** — should be optional
6. **Coverage push** — 84% → 90% target
