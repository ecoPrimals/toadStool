# ToadStool S231 — JH-2 Resource Envelope Enforcement at Dispatch

**Date**: May 8, 2026
**Trigger**: primalSpring Phase 60 audit — JH-2 (High): token-carried resource envelope enforcement at `compute.dispatch.submit`
**Depends on**: S229 (JH-0 MethodGate), S230 (error code alignment)

## What Changed

### New Types (`method_gate.rs`)

- **`ResourceEnvelope`** — Serializable struct carrying ionic token resource limits:
  - `mem_mb: Option<u64>` — max memory per dispatch
  - `cpu_cores: Option<u32>` — max CPU cores
  - `method_allowlist: Vec<String>` — methods the token can call (empty = all)
  - `allows_method(&self, method) -> bool` — allowlist check

- **`CallerContext`** — Per-request caller identity and envelope:
  - `identity: Option<String>` — DID from ionic token
  - `envelope: Option<ResourceEnvelope>` — resource bounds
  - `anonymous()` — no-token default (permissive-mode backward compat)

### Enhanced Gate (`method_gate.rs`)

- **`MethodGate::check_with_context(method, ctx)`** — JH-2 upgrade of `check()`:
  - **Permissive mode**: allows all, but still enforces `method_allowlist` if a token is present
  - **Enforcing mode**: anonymous → `UNAUTHORIZED` (-32000); token without method in allowlist → `PERMISSION_DENIED` (-32001)
- `check()` now delegates to `check_with_context()` with anonymous context (backward compat)

### Dispatch Enforcement (`dispatch/submit.rs`)

- **`enforce_envelope(ctx, binary_size, timeout_ms)`** — Pre-dispatch resource check:
  - If caller has `mem_mb` limit: binary size (rounded up to MB) must not exceed it
  - Returns `RESOURCE_EXHAUSTED` (-32004) on violation
  - No-op when no envelope present (anonymous callers pass through)

- **`dispatch_submit_with_context(params, ctx)`** — New context-aware dispatch entry:
  - Calls `enforce_envelope()` after parsing binary, before job creation
  - `dispatch_submit()` delegates to this with anonymous context (backward compat)

### Auth Introspection (`auth.rs`)

- **`auth.peer_info`** now accepts `CallerContext` and returns:
  - `authenticated: bool` — whether caller presented identity
  - `identity: string|null` — caller DID
  - `envelope: object|null` — resource envelope from token

### Handler Wiring (`mod.rs`)

- `handle_method()` creates `CallerContext::anonymous()` and uses `check_with_context()`
- Token extraction point is marked — BearDog JH-1 ships → extract token → populate context

## What's Still Blocked

- **BearDog JH-1**: `identity.create`, `auth.issue_ionic`, `auth.verify_ionic` — required for real token issuance and verification
- **biomeOS JH-2 co-owner**: neuralAPI dispatch must also enforce envelope limits
- Until JH-1 ships, all callers are anonymous and enforcement is a no-op in permissive mode

## Test Coverage

- 17 new tests:
  - `method_gate`: 7 tests (authenticated access, allowlist enforcement, envelope serde, anonymous context)
  - `auth`: 2 tests (anonymous peer_info, identity-bearing peer_info)
  - `dispatch/envelope`: 8 tests (no envelope, mem limit allow/deny/boundary, context-aware dispatch)
- 767 total tests in `toadstool-server` (0 failures)
- Workspace `cargo clippy -- -D warnings`: clean
- Error codes aligned per ecosystem standard (S230)

## Files Changed

| File | Change |
|------|--------|
| `crates/server/src/pure_jsonrpc/handler/method_gate.rs` | `ResourceEnvelope`, `CallerContext`, `check_with_context()`, 7 new tests |
| `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` | `enforce_envelope()`, `dispatch_submit_with_context()` |
| `crates/server/src/pure_jsonrpc/handler/dispatch/tests.rs` | 8 envelope enforcement tests |
| `crates/server/src/pure_jsonrpc/handler/auth.rs` | `auth_peer_info` takes `CallerContext`, 2 updated tests |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | `CallerContext` in `handle_method()`, `check_with_context()` |
| `README.md` | S231 entry |
| `NEXT_STEPS.md` | S231 entry |

## Activation Path (When BearDog JH-1 Ships)

1. Extract ionic token from request params or transport-level metadata
2. Verify token via `auth.verify_ionic` IPC to BearDog
3. Populate `CallerContext { identity, envelope }` from verified token claims
4. Switch `MethodGate` to `GateMode::Enforcing` (or per-composition config)
5. All existing infrastructure activates — no new code needed at dispatch level
