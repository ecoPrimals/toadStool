# ToadStool S229 — JH-0: MethodGate Adoption (May 8, 2026)

**Date**: May 8, 2026
**Author**: toadStool team (automated session S229)
**Priority**: P2 (ecosystem-wide JH-0 standard adoption)
**Status**: RESOLVED (JH-0 complete; JH-2 tracked, blocked on BearDog JH-1)

---

## Problem

primalSpring v0.9.25 shipped the ecosystem-standard pre-dispatch capability gate (JH-0). All primals must adopt the `MethodGate` pattern: classify methods as Public/Protected, gate before dispatch, start permissive.

ToadStool additionally co-owns JH-2: resource envelope enforcement from ionic tokens at `compute.dispatch.submit` time.

## Solution

### JH-0: MethodGate Pattern (COMPLETE)

**New module: `crates/server/src/pure_jsonrpc/handler/method_gate.rs`**

- `MethodVisibility` enum: `Public` | `Protected`
- `GateMode` enum: `Permissive` | `Enforcing`
- `MethodGate` struct with `check(method)` → `Result<(), JsonRpcError>`
- `classify_method(method)` — maps all ~65 methods into visibility tiers
- Ships in `Permissive` mode (all calls allowed)

**Method classification:**

| Visibility | Methods |
|------------|---------|
| Public | `health.*`, `identity.get`, `capabilities.*`, `toadstool.version`, `compute.version`, `compute.health`, `compute.capabilities`, `compute.discover_capabilities`, `toadstool.query_capabilities`, `toadstool.health`, `provenance.*`, `auth.*` |
| Protected | `compute.dispatch.*`, `shader.dispatch`, `compute.execute`, `compute.submit`, `toadstool.submit_workload`, `compute.hardware.*`, `gate.*`, `transport.*`, `ember.*`, `compute.performance_surface.*`, all resource/AI methods |

**New module: `crates/server/src/pure_jsonrpc/handler/auth.rs`**

Three public JSON-RPC methods for gate introspection:
- `auth.check` — test whether a method would be allowed (`{"method": "..."}` → `{"allowed": bool, "visibility": "...", "mode": "..."}`)
- `auth.mode` — return current gate mode (`{"mode": "permissive"}`)
- `auth.peer_info` — return caller info (minimal until JH-2: `{"transport": "unknown", "authenticated": false}`)

**Wiring:**
- `MethodGate` added as field on `JsonRpcHandler`
- Gate check inserted at top of `handle_method()` — runs before any routing
- Auth methods registered in semantic method registry (`mappings_core.rs`)
- Auth methods accessible via both direct name and semantic dispatch

**Error codes:**
- `-32005` `AUTH_REQUIRED` — caller has no identity/token (existing, now documented)
- `-32006` `PERMISSION_DENIED` — caller lacks access to method (new)

### JH-2: Resource Envelope Enforcement (TRACKED)

**Status**: Blocked on BearDog JH-1 (ionic token infrastructure).

When BearDog ships `identity.create`, `auth.issue_ionic`, and `auth.verify_ionic`:
1. Thread `CallerContext` (token + peer info) through `handle_request`
2. `MethodGate::check()` will verify token validity and resource envelope
3. `dispatch_submit` will check `mem`/`cpu` limits from token envelope
4. Switch `GateMode` to `Enforcing`

The enforcement point is the existing `job_id` session pattern in `dispatch/submit.rs`.

## Verification

- `cargo clippy --workspace -- -D warnings`: **zero warnings**
- `cargo test --workspace`: all tests pass, exit code 0
- 21 new tests: 6 in `method_gate`, 6 in `auth`, 3 handler integration tests, 6 classification coverage tests

## Files Changed

- `crates/server/src/pure_jsonrpc/handler/method_gate.rs` (NEW)
- `crates/server/src/pure_jsonrpc/handler/auth.rs` (NEW)
- `crates/server/src/pure_jsonrpc/handler/mod.rs` — gate field, gate check, auth routes
- `crates/server/src/pure_jsonrpc/handler/mod_tests.rs` — 3 integration tests
- `crates/server/src/pure_jsonrpc/types.rs` — `unauthorized()`, `permission_denied()` constructors
- `crates/core/common/src/constants/jsonrpc.rs` — `PERMISSION_DENIED` code (-32006)
- `crates/core/toadstool/src/semantic_methods/mappings_core.rs` — auth domain mappings

## Cross-Primal Notes

- toadStool now emits structured `tracing::trace` on every gate check — skunkBat (JH-5) can consume these
- `auth.check` / `auth.mode` / `auth.peer_info` are callable by any primal for composition health checks
- The `MethodGate` pattern matches primalSpring's `METHOD_GATE_STANDARD.md` specification
