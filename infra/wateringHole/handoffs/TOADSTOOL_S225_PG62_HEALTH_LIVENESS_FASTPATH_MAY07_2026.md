# ToadStool S225 — PG-62: Health Liveness Fast-Path + Startup Reorder

**Date**: May 7, 2026
**Author**: toadStool team (automated session S225)
**Priority**: P2 (hotSpring V0.6.32 handoff — GAP-HS-040 / PG-62)
**Status**: RESOLVED

---

## Problem

hotSpring reported intermittent timeouts when calling `health.liveness` during composition startup with their default 2-second probe timeout. The root causes were:

1. **Discovery registration blocked listener startup** — `register_with_discovery()` ran before the JSON-RPC socket listener was spawned, meaning the socket didn't exist yet during slow discovery attempts
2. **No readiness semantics** — `health.liveness` and `health.readiness` returned static `"alive"`/`"ready"` regardless of actual initialization state, providing no signal to callers about whether the server was truly operational

## Solution

### 1. Health Liveness Fast-Path (`Arc<AtomicBool>` readiness flag)

Added a shared `Arc<AtomicBool>` readiness flag to `JsonRpcHandler`:

- `health.liveness` returns `{"status":"starting"}` while `ready == false`, `{"status":"alive"}` once `ready == true`
- `health.readiness` returns `{"status":"starting","version":"..."}` → `{"status":"ready","version":"..."}`
- `health.check` (full envelope) always returns the full payload (unchanged — it includes uptime/error_count which are meaningful even during init)

### 2. Startup Reorder

Moved discovery registration and biomeOS socket scan **after** the listener spawn:

**Before (S224):**
1. Create executor
2. `register_with_discovery()` ← blocks socket creation
3. biomeOS scan
4. Create `JsonRpcHandler`
5. `tokio::spawn(start_servers_with_fallback)` ← socket created here

**After (S225):**
1. Create executor
2. Create `JsonRpcHandler` with `ready = false`
3. `tokio::spawn(start_servers_with_fallback)` ← socket created immediately
4. `register_with_discovery()` ← runs while socket is already accepting
5. biomeOS scan
6. `ready.store(true, Release)` ← health.liveness transitions to "alive"

### 3. Recommended Timeout

Documented in README: **≥3 seconds** for health probes during startup. If BTSP handshake is required, add its budget (5s default, overridable via `BTSP_HANDSHAKE_TIMEOUT_SECS`).

## Files Changed

### Production
- `crates/server/src/pure_jsonrpc/handler/mod.rs` — Added `ready: Arc<AtomicBool>` to `JsonRpcHandler`, pass to `health_liveness`/`health_readiness`
- `crates/server/src/pure_jsonrpc/handler/core/health.rs` — `health_liveness(ready: bool)` and `health_readiness(version, ready: bool)` with starting/alive semantics
- `crates/server/src/unibin/mod.rs` — Readiness flag, startup reorder (discovery after listener spawn)

### Tests (new: +5)
- `crates/server/src/pure_jsonrpc/handler/core/mod.rs` — `health_liveness_starting_before_ready`, `health_readiness_starting_before_ready`
- `crates/server/src/pure_jsonrpc/handler/mod_tests.rs` — `test_health_liveness_returns_starting_before_ready` (full lifecycle: starting → set ready → alive)

### Tests (updated: ~30 call sites)
All `JsonRpcHandler::new()` calls updated to pass the new `ready` parameter (`Arc::new(AtomicBool::new(true))` for existing tests).

### Documentation
- `README.md` — S225 entry, test count 22,843, health probe timeout table
- `NEXT_STEPS.md` — S225 entry, BearDog `crypto.sign_contract` (PG-60+) tracked
- `DOCUMENTATION.md` — S225 state, recommended caller timeout
- `CONTEXT.md` — Test count updated

## Cross-Primal Notes

### For hotSpring
- `health.liveness` now responds immediately during init with `{"status":"starting"}`
- Recommended probe timeout: ≥3s (≥5s if BTSP handshake is in the path)
- Callers can distinguish "starting" from "alive" to implement retry/backoff logic

### For primalSpring
- PG-62 status: **RESOLVED**
- PG-46 (older slow socket response gap): also addressed by this startup reorder

### BearDog — crypto.sign_contract (PG-60+, P3 Deferred)
- Cross-family ionic bond contract signing tracked in `NEXT_STEPS.md` for Phase 60+
- Not implemented in this session (no urgency per primalSpring guidance)
- When ready: expose `crypto.sign_contract` as JSON-RPC method accepting `{proposer, acceptor, capabilities, duration}`

## Test Results

22,843 tests, 0 failures, 0 clippy warnings, 0 fmt diffs.
