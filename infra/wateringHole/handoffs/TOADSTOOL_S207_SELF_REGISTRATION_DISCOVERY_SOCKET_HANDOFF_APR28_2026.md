# ToadStool S207 — Self-Registration via DISCOVERY_SOCKET

**Date**: April 28, 2026
**Session**: S207
**Scope**: Primal self-registration with Songbird at startup (primalSpring Phase 55b P5)

---

## Context

primalSpring v0.9.21 requested ToadStool self-register at startup so
primals can join a running NUCLEUS without a restart/re-launch cycle.
Previously, the composition launcher registered all 12 primals with
Songbird on behalf.

## Changes

### 1. `register_with_discovery()` — new primary registration function

**File**: `crates/core/toadstool/src/ipc_helpers/connection.rs`

- Uses `resolve_capability_socket_fallback("discovery", &SocketPathEnv::from_env())`
  to honor `DISCOVERY_SOCKET` (highest precedence, set by `composition_nucleus.sh`)
- Method: `ipc.register` (was `capability.register`)
- Payload: `{ "primal_id": "toadstool", "capabilities": ["compute.dispatch", "compute.capabilities"], "endpoint": "unix:///…/compute.sock" }`
- Endpoint uses `resolve_toadstool_socket()` — matches actual listen path
- Fire-and-forget at call site: if absent or fails, primal continues standalone

### 2. `find_by_capability()` — evolved to use discovery path

- Now uses `resolve_capability_socket_fallback("discovery", ...)` instead of
  hardcoded `BIOMEOS_COORDINATION_SOCKET` / `COORDINATION_SOCKET`
- Method evolved from `capability.find` to `ipc.find_capability`
- Honors `DISCOVERY_SOCKET` with full fallback chain

### 3. `register_with_coordination()` — deprecated

- Now delegates to `register_with_discovery()`
- Annotated `#[deprecated(note = "use register_with_discovery")]`
- Re-exports in `ipc/mod.rs` and `ipc_helpers/mod.rs` use `#[expect(deprecated)]`

### 4. DaemonServer startup — self-registration wired

**File**: `crates/cli/src/daemon/server.rs`

Previously only `run_server_main` (UniBin path) called registration.
Now `DaemonServer::start()` also calls `register_with_discovery()` —
both startup paths are covered.

### 5. Stale `resolve_coordination_socket()` removed

Internal helper was dead code after the evolution; removed.

---

## Files Changed (5)

| File | Change |
|------|--------|
| `crates/core/toadstool/src/ipc_helpers/connection.rs` | New `register_with_discovery()`, evolved `find_by_capability()`, deprecated old fn, removed dead helper |
| `crates/core/toadstool/src/ipc_helpers/mod.rs` | Export `register_with_discovery`, `#[expect(deprecated)]` on old re-export |
| `crates/core/toadstool/src/ipc/mod.rs` | Same re-export evolution |
| `crates/server/src/unibin/mod.rs` | Call `register_with_discovery()` instead of `register_with_coordination()` |
| `crates/cli/src/daemon/server.rs` | Wire `register_with_discovery()` into `DaemonServer::start()` |

## Tests

- **7,842 lib-only** (+1 from `test_register_with_discovery_sends_ipc_register_method`)
- All registration mocks updated to use `DISCOVERY_SOCKET`
- New wire-format capture test verifies `ipc.register` method + `primal_id` + capabilities + `unix://` endpoint
- 0 failures, clippy clean, fmt clean

## For primalSpring / guideStone

- ToadStool now self-registers at startup when `DISCOVERY_SOCKET` is set
- Method: `ipc.register` with `primal_id: "toadstool"`, capabilities: `["compute.dispatch", "compute.capabilities"]`
- Endpoint format: `unix:///run/user/1000/biomeos/compute.sock` (actual resolved path)
- If `DISCOVERY_SOCKET` is absent, falls back to `BIOMEOS_COORDINATION_SOCKET` / legacy / default — then continues standalone if unreachable
- Both `run_server_main` and `DaemonServer` paths now self-register

## Next Evolution

- Pipeline encryption (extend dispatch encryption to pipeline submit)
- Coverage push 83.6% → 90% (hardware mocks for V4L2/VFIO)
- `display.composite` for multi-layer blending
