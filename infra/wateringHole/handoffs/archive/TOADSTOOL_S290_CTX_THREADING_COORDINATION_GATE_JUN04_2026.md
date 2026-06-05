# toadStool S290 — CallerContext Threading + Coordination Feature Gate + Panic Hygiene

**Date**: June 4, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: All targets complete. Software-only.

## Changes

### P0: CallerContext wired through fan_out

- `compute.fan_out` now uses `CallerContext` end-to-end (was `_ctx` — ignored)
- Pre-dispatch resource check via orchestrator when `gpu_required`
- Envelope enforcement: `cpu_cores × 4` max concurrent unit cap
- Telemetry emission via `emit_dispatch_completion_telemetry`
- Response includes `caller.gate_id` and `caller.trust_level` for audit
- Removed stale `#[expect(clippy::unused_async)]` — handler now awaits resource check
- +3 tests (envelope rejects, no-envelope allows, caller context in response)

### P1: distributed::coordination feature-gated

- `pub mod coordination` gated behind `#[cfg(feature = "legacy-coordination")]`
- Deprecated re-exports also gated
- `coordination_integration` remains always-compiled (production path)
- 2 integration test files require `legacy-coordination` feature
- ~6.3k LOC no longer compiled in default builds (1,289 tests behind feature)

### P1: sovereign_acr_boot + diagnostic unwraps hardened

- `sovereign_acr_boot.rs`: guarded unwraps → single `if let` tuple pattern
- 7 diagnostic files: ~45 `write!().unwrap()` / `writeln!().expect()` → `let _ = write!()`
  - `bar_cartography.rs`, `report.rs`, `topology.rs`, `device_info.rs`
  - `probe.rs`, `pmu.rs`, `vram.rs`

## Metrics

- **8,895** lib tests passed (default), **+1,289** with `legacy-coordination` (10,184 total)
- Full workspace clippy `-D warnings` clean
- 13 files changed, 284 insertions, 231 deletions

## Remaining Debt

1. **`networking` feature** — `fallback_response()` intentional degraded mode; P2 documentation debt
2. **`LEGACY_*` env fallbacks** — ~30 deprecated constants; staged removal pending NUCLEUS standardization
3. **`serialport`** always-on in `runtime/edge` — no pure-Rust replacement for USB-UART
4. **Sovereign dispatch CallerContext** — `fan_out` done; status/result/capabilities/sovereign paths still need ctx
5. **`DispatchTelemetryRecord`** emitted but not persisted to disk
6. **Test file proliferation** — ~77 `*_coverage_tests.rs` files could consolidate
7. **Large file prevention** — `device.rs` (781L) and `handler/mod.rs` (732L) approaching 800L threshold
