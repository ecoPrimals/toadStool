# ToadStool S277 — Wave 54 Response: Early Health Responder

**Date:** 2026-05-27
**Session:** S277
**Upstream:** primalSpring Wave 54 — Mountain Upstream Blurbs
**Gate:** southGate NUCLEUS (8/13 health → 9/13 with this fix)

---

## Issue

primalSpring Wave 54 flagged ToadStool as "SOCKET" on southGate — the
UDS socket file exists and a process is running behind it, but
`health.check` / `health.liveness` JSON-RPC probes return no response.

## Root Cause

**Startup timing gap** between socket pre-bind and handler readiness.

Wave 49 (S275) added `prebind_unix_listener()` to bind the socket before
`create_executor()` runs. This made the socket FILE appear immediately,
so `wait_for_socket` in the launcher succeeds. But the accept loop
(`serve_unix_prebound`) only started after `create_executor` + handler
construction (~4-8s). During that window:

1. Launcher sees socket file → `wait_for_socket` passes
2. Launcher sends `health.liveness` via socat
3. Kernel accept backlog queues the connection (socket is bound)
4. Nobody is calling `accept()` yet → socat times out after 3s
5. Launcher reports status "SOCKET" (connection succeeded, no response)

BTSP was NOT the blocker. The `handle_btsp_connection` auto-detects
plaintext JSON-RPC (first byte >= 0x09) and degrades gracefully to
NDJSON handling. This is confirmed by neuralSpring achieving ALIVE
status with the same binary — their deployment simply waited longer.

## Fix

Added **early health responder** that runs on the pre-bound listener
immediately, before `create_executor()`:

- `spawn_early_health_responder()` accepts connections and responds to:
  - `health.liveness` → `{"status":"alive"}`
  - `health.check` / `toadstool.health` / `compute.health` → `{"status":"starting"}`
  - `health.readiness` → `{"status":"starting"}`
  - All other methods → `-32002 "Server initializing"`
- Uses `tokio::sync::watch` channel for shutdown signaling
- Listener is `Arc<UnixListener>` shared between early responder and
  eventual full handler
- Early responder is cancelled and yields before `start_servers_with_fallback`

Timeline with fix:
```
t=0ms   prebind_unix_listener() → socket bound
t=1ms   spawn_early_health_responder() → accepting connections
t=5ms   Launcher: wait_for_socket → PASS
t=100ms Launcher: health.liveness → {"status":"alive"} ← NEW
t=4-8s  create_executor() completes
t=8s    early_stop_tx.send(true) → early responder stops
t=8s    serve_unix_prebound() → full handler takes over
```

## BTSP Documentation (for launcher)

BTSP is NOT required for health probes. ToadStool auto-detects the wire
protocol per connection:

- **Plaintext JSON-RPC** (socat/curl): First byte >= 0x09 → NDJSON handling
  No handshake needed. Send `{"jsonrpc":"2.0","method":"health.liveness","id":1}\n`
- **BTSP binary framing**: First byte < 0x09 → full BTSP handshake
  Required only for encrypted/authenticated sessions

## Socket Naming

The launcher passes `TOADSTOOL_SOCKET=toadstool-${FAMILY_ID}.sock`, which
ToadStool respects (env var takes priority). Without `TOADSTOOL_SOCKET`,
ToadStool uses capability-domain naming: `compute-${FAMILY_ID}.sock`.

Both paths work. The launcher's `[compute]="toadstool-${FAMILY_ID}.sock"`
mapping matches what `TOADSTOOL_SOCKET` overrides to.

## Test Verification

- 3 new tests: `early_health_liveness_responds_alive`,
  `early_health_check_responds_starting`, `early_health_unknown_method_returns_error`
- Full workspace: 9,161+ lib tests, 0 failures, 0 clippy warnings

## Files Changed

- `crates/server/src/pure_jsonrpc/connection/unix.rs` — `spawn_early_health_responder()` + `handle_early_health()`, `serve_unix_prebound` now takes `Arc<UnixListener>`
- `crates/server/src/pure_jsonrpc/connection/mod.rs` — re-export
- `crates/server/src/pure_jsonrpc/mod.rs` — re-export
- `crates/server/src/unibin/mod.rs` — wire early responder with watch channel
- `crates/server/src/unibin/execution.rs` — `Option<Arc<UnixListener>>` signature
- `crates/server/src/pure_jsonrpc/connection/tests.rs` — 3 new tests
