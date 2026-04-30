# ToadStool S212–S213 — Coverage Push + Deep Debt (Lint Reason + Capability Names + Orchestrator Resilience)

**Date**: April 30, 2026
**Sessions**: S212, S213
**Commits**: `224ced84b` (S212), `8c7049556` (S213)
**Tests**: 22,000+ workspace, 7,842+ lib-only, 0 failures, clippy clean, fmt clean

---

## S212: Coverage Push — primalSpring Phase 56c Audit Response

Targeted the 83.6% → 90% coverage gap identified by primalSpring Phase 56c
audit. Added ~100 new inline `#[cfg(test)]` module tests across 10 previously-
untested production files:

- **server identity/capability/discovery handlers** (8 tests)
- **server job handler** error paths, gate routing, list/cancel (16 tests)
- **CLI metrics collectors** + dispatch enum (15 tests)
- **platform monitoring** Linux proc parsers + live-process metrics (8 tests)
- **auto_config detection** Linux/macOS/Windows/unknown + HW scaling (12 tests)
- **auto_config generation** small/large HW, security, history cap (8 tests)
- **auto_config NL templates** all 5 templates + fallback chains (8 tests)
- **auto_config config builder** chaining, defaults, full build (4 tests)
- **distributed security_provider dispatch** via mock lifecycle (7 tests)
- **distributed crypto_dispatch** provider identity + capabilities (2 tests)

**1,004 new test lines** across 10 source files + 2 doc files.

---

## S213: Deep Debt — Lint Reason Sweep + Capability Names + Orchestrator Resilience

### Phase 1: Lint Reason Completion

All remaining bare `#[allow]`/`#[expect]` attributes given `reason = "..."`:
- `core/config/types/network.rs` (2 test `#[expect(deprecated)]`)
- `core/common/infant_discovery/sources/service_mesh.rs` (4 test `#[expect(deprecated)]`)
- `core/common/infant_discovery/sources/mdns.rs` (1 `#[expect(path_statements)]`)
- `cli/ecosystem/discovery.rs` (1 test `#[expect(deprecated)]`)
- `cli/ecosystem/types/mod.rs` (1 test `#[expect(deprecated)]`)
- `core/common/interned_strings/mod.rs` (1 test `#[expect(deprecated)]`)
- `client/tarpc_client.rs` (1 test `#[expect(deprecated)]`)
- `management/monitoring/collection.rs` (1 `#[expect(unused_imports)]`)
- `core/toadstool/workload_migration/validation.rs` (1 `#[allow(deprecated)]`)
- `server/config/mod.rs` (1 `#[allow(clippy::float_cmp)]`)

Workspace now **fully lint-reason compliant**.

### Phase 2: Capability-Based Primal References

GPU backend stubs evolved from hardcoded primal names to capability URIs:
- `runtime/gpu/backends/cuda_impl/mod.rs`: Error messages, deprecation notes,
  and doc comments now reference `discover_capability("gpu.dispatch.cuda")`
  instead of naming `barraCuda`/`coralReef`.
- `runtime/gpu/backends/mod.rs`: Module docs updated to reference
  `gpu.dispatch.cuda` capability providers.

### Phase 3: Orchestrator Lock Resilience

Evolved `WorkloadOrchestrator` from `expect("lock poisoned")` panics to
proper error returns:
- Added `OrchestrationError::LockPoisoned(String)` variant
- `register_substrate()` → `Result<(), OrchestrationError>`
- `num_substrates()` → `Result<usize, OrchestrationError>`
- `stats()` → `Result<OrchestratorStats, OrchestrationError>`
- All internal `select_substrate`, `rank_substrates`, `execute` lock
  acquisitions use `.map_err(|e| OrchestrationError::LockPoisoned(...))?`
- **Zero production `expect("lock poisoned")` remaining**

---

## For primalSpring / guideStone

No wire-protocol changes. No new JSON-RPC methods. Coverage push (S212)
adds test confidence; no behavioral changes. Orchestrator API change (S213)
is internal — callers that used `num_substrates()` or `stats()` now receive
`Result` types.

---

## Remaining Debt (Post-S213)

- **D-COV**: ~83.6% → 90% target. Remaining gap: hardware-dependent paths
  (VFIO, DRM, V4L2, akida userspace), neuromorphic drivers, specialty runtimes.
- **D-BTSP-PHASE3**: Encrypted post-handshake channel deferred (ecosystem-wide).
- `safe_mmap.rs` `as_volatile()` / `as_ptr()` still use `expect()` for
  defense-in-depth null checks (changing would break public API).
- `burn-inference` crate excluded from workspace — cannot use workspace deps.
- Legacy serde aliases in config types still embed primal names for backward
  compat deserialization.
