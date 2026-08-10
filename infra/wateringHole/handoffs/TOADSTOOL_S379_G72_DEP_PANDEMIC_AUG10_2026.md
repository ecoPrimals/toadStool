# ToadStool S379 — G72 Dependency Pandemic Tier 1 + Last-Mile Wiring

**Date**: Aug 10, 2026
**Sprint**: S379
**Wave**: 157g STADIAL SHIFT — G72 Dependency Pandemic

## G72 Compliance — toadStool as Exemplar

### Tier 1 Completed

- **tokio `["full"]` trimmed** — examples only (was pulling 15+ features)
- **tokio `signal` scoped** — workspace 7→6 features. Only CLI+server need signal.
- **tokio::fs fully eliminated** — 28 files migrated to `std::fs`. Zero `tokio::fs` workspace-wide.
- **7 dead deps removed** — `http-body-util` (×2), `criterion`, `uuid`, `env_logger`, `test-log`, `tempfile`
- **6 deps promoted to workspace** — `bytemuck` (aligned 1→1.14), `zeroize`, `wasmi`, `blake3`, `anyhow`, `mdns-sd`
- **Version alignment** — `bytes` aligned in `toadstool-core`, `bytemuck` standardized at 1.14

### Tier 2 Candidates (for next sprint)

- **`tokio-serde`** in server — should use `{ workspace = true, features = ["json"] }`
- **Container `axum`** — regression from S378 ethos. Migrate BYOB to UDS JSON-RPC.
- **`tracing-subscriber`** in core library — heavy for WASM type builds, gate behind `logging` feature
- **`uuid` workspace promotion** — 16 crates pin inline, could standardize

## Last-Mile Wiring

- WASM workload conversion wired (`conversion.rs`: Wasm + Container variants)
- Runtime hint inference (`start_primal` / `start_service`)
- `component-model` dead feature excised

## Excised (~1,750 LOC)

- `executor/display.rs` (~260) — test-only duplicate
- `ecosystem/discovery.rs` (~475) — duplicate discovery stack
- `discovery_coverage_tests.rs`, `ecosystem_discovery_tests.rs` (324), `executor_modules_unit_tests.rs` (531) — stale test debris

## Doc Sync

- Test count: 9,008→8,447 (actual after feature gating)
- Unsafe blocks: 138→160 (actual)
- Forbid crates: 41→39 (actual)
- `ffi_loader` removed from containment lists (deleted S378)
- Stale metrics fixed in README, CONTEXT, DOCUMENTATION, NEXT_STEPS, sporeprint

## WASM Status

38/48 (79%). Remaining 9 are architecturally native (CLI binary, server daemon, container/display/native runtimes, sandbox, testing infra). Dep pandemic confirms: no new WASM targets unlockable via dependency trimming alone.

## Verification

- `cargo check --workspace` — 0 errors
- `cargo test --workspace --lib` — 8,447 passed, 0 failed
- Zero `tokio::fs` in workspace
- Zero `tokio = ["full"]` in workspace
