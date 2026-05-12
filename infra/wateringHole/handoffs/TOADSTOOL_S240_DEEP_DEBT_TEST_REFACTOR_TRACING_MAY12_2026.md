# ToadStool S240 — Deep Debt Sweep: Test Refactor, println→tracing, Constants

**Date**: May 12, 2026
**Commit**: `42fa7c90`
**Tests**: 8,278 lib-only passing (22,843+ workspace), 0 failures, 0 clippy warnings

---

## Changes

### Smart Refactor: `execution/tests.rs` (831→4 submodules)

Refactored the last remaining >800-line production-crate test file into a `tests/`
directory mirroring the production module structure:

- `tests/mod.rs` — shared helpers (`sample_context`, `MockRuntimeEngine`,
  `NativePrimalTemplate`, `FailingNativePrimal`, `TypedRoutePrimal`, `OnlyWasmPrimal`)
- `tests/native.rs` — 7 tests for `execute_native` paths (primal routing, local
  engine fallback, error propagation, process stderr)
- `tests/wasm.rs` — 2 tests for `execute_wasm` (engine absent, engine registered)
- `tests/primal.rs` — 4 tests for `execute_primal` (routing, error statuses,
  handler errors, no-provider listing)
- `tests/biome_os.rs` — 4 tests for `execute_biome_os` (success, statuses,
  route error, no OS provider)

All 17 execution tests pass unchanged.

### neurobench-runner `println!` → `tracing`

`BenchmarkResult::print_summary()` migrated from raw `println!` to
`tracing::info!` with structured fields: `benchmark`, `accuracy_pct`,
`throughput_inf_per_s`, `mean_latency_ms`, `p95_latency_ms`, `p99_latency_ms`,
`mean_power_mw`, `energy_per_inference_uj`, `samples`.

### DiscoveryEngine Named Constant

Inline `Duration::from_secs(5)` in both `with_defaults()` and `new()` extracted
to `DEFAULT_DISCOVERY_TIMEOUT_SECS` constant.

### Handoff Archival

8 pre-S234 handoffs (S224–S233) archived to
`ecoPrimals/infra/wateringHole/fossilRecord/`. 6 current handoffs remain
(S234–S239).

---

## Full Audit Findings

| Category | Result |
|---|---|
| Large files (>800L) | All 8 are test files; all production code <800L |
| Production mocks | Zero — `StubRuntimeEngine` is correct sentinel |
| `println!`/`eprintln!` in libraries | Zero (CLI println is intentional UX) |
| TODO/FIXME/HACK/XXX | Zero in production code |
| Production `unreachable!()` | Zero |
| `unsafe` blocks | 46 across 15 hw-boundary files, all SAFETY-documented |
| `.expect()` in production | Zero (all in `#[cfg(test)]`) |
| Deprecated stubs | `CudaBackend` properly `#[deprecated]` with migration guidance |
| Dependencies | `cargo deny` strict; `paste` RUSTSEC acknowledged; `ring` banned |
| .bak/.tmp/.old/.orig files | Zero |
| Stale scripts | None — all scripts are operational tooling |

---

## Root Docs Updated

- `README.md` → S240
- `CHANGELOG.md` → S240
- `DEBT.md` → S240
- `DOCUMENTATION.md` → S240
- `NEXT_STEPS.md` → S240
- `CONTEXT.md` → S240

---

## Next Steps

Per Compute Trio Continuing Evolution Sprint:
- **Phase C**: cylinder + coral-driver hardware absorption
- **Phase D**: local dispatch (Gate 4)
- **Coverage push**: 83.6% → 90%
