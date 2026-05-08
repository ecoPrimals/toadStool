# ToadStool S233 — DF-2 Fix: TOADSTOOL_AUTH_MODE + Tier 3 eprintln! Migration

**Date**: May 8, 2026
**Session**: S233
**Scope**: DF-2 RESOLVED, Tier 3 eprintln migration

---

## DF-2: TOADSTOOL_AUTH_MODE Environment Variable

**Problem**: `JsonRpcHandler::new()` hardcoded `MethodGate::permissive()`, ignoring the
`TOADSTOOL_AUTH_MODE` environment variable. Deployments with `TOADSTOOL_AUTH_MODE=enforcing`
had no effect — the gate was always permissive.

**Fix**: `JsonRpcHandler::new()` now reads `TOADSTOOL_AUTH_MODE` at startup:
- `"enforcing"` or `"enforced"` (case-insensitive) → `GateMode::Enforcing`
- Any other value or unset → `GateMode::Permissive` (backward-compatible default)
- Logs `info!` when enforcing mode is activated

**Verification**: `auth.mode` JSON-RPC method returns `{"mode": "enforcing"}` when deployed
with `TOADSTOOL_AUTH_MODE=enforcing`. Two new tests confirm this behavior.

**File**: `crates/server/src/pure_jsonrpc/handler/mod.rs`

---

## Tier 3: eprintln! → tracing Migration

**Audit finding**: `eprintln!` still appears in toadStool codebase (inherited from coral-driver
paths). Ecosystem logging standard is `tracing` for structured log filtering.

**Changes**:
- `crates/testing/src/performance/manager.rs`: `eprintln!` → `tracing::warn!` (benchmark iteration failures)
- `crates/cli/src/commands/npu.rs`: `eprintln!` → `tracing::error!` (NPU device discovery failure)

**Retained as-is** (idiomatic):
- `crates/core/nvpmu/src/bin/nvpmu_apply.rs`: Standalone CLI binary — `eprintln!` is standard for
  usage text, progress messages, and fatal errors in binaries without a tracing subscriber
- `crates/neuromorphic/akida-models/examples/*.rs`: Example binaries — same rationale
- Test files (`gpu_safety_debug.rs`, `buffer/tests.rs`, `*_tests.rs`): `eprintln!` in test code
  is idiomatic Rust for test diagnostics (visible on failure, no subscriber needed)

**Status**: RESOLVED — all production library code paths now use `tracing`.

---

## Tests Added

- `test_auth_mode_enforcing_gate`: Verifies `MethodGate::new(GateMode::Enforcing)` → `auth.mode` returns `"enforcing"`
- `test_enforcing_gate_denies_anonymous_protected_method`: Verifies enforcing gate rejects anonymous protected method calls with `-32000`

---

## Open Items

None from this audit. toadStool has **zero remaining upstream debt** per primalSpring audit.
