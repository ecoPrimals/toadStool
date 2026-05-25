# toadStool → primalSpring: Stale Socket Cleanup Response — S264

**Date**: May 18, 2026
**From**: toadStool (compute dispatch primal)
**Session**: S264
**Responding to**: Stale Socket Detection + Cleanup — All Teams

---

## Summary

toadStool already had `unlink()` before `bind()` at all 6 UDS bind sites and
shutdown cleanup in the production server path. Two gaps were found and fixed:
CLI daemon shutdown (no-op stub) and display IPC server (missing Drop impl).

---

## Audit Results

### `unlink()` before `bind()` — ALL 6 BIND SITES ALREADY CLEAN

| Bind Site | File | Pre-existing cleanup |
|-----------|------|---------------------|
| `platform::unix::bind` | `crates/core/toadstool/src/ipc/platform/unix.rs:57` | `tokio::fs::remove_file` if exists |
| `serve_unix` (JSON-RPC) | `crates/server/src/pure_jsonrpc/connection/unix.rs:52` | `tokio::fs::remove_file` if exists |
| `TarpcServer::serve_unix` | `crates/server/src/tarpc_server/mod.rs:152` | `remove_file`; OK if NotFound |
| `try_unix_servers` | `crates/server/src/unibin/execution.rs:228` | Best-effort remove both paths |
| CLI `start_jsonrpc_server` | `crates/cli/src/daemon/jsonrpc_server.rs:155` | `remove_file` if exists |
| Display `try_unix_server` | `crates/runtime/display/src/ipc/server.rs:156` | Best-effort remove |

### Shutdown cleanup — 2 GAPS FIXED

| Component | Before S264 | After S264 |
|-----------|-------------|------------|
| **UniBin server** (production) | Removes both sockets + legacy symlink on SIGINT/SIGTERM | No change needed |
| **`IpcServer::Drop`** | Removes Unix socket files on drop | No change needed |
| **CLI daemon** (`DaemonServer`) | `shutdown()` was no-op stub; only `ctrl_c` | **FIXED**: removes socket on shutdown; handles SIGINT + SIGTERM |
| **Display IPC** (`DisplayServer`) | No Drop impl | **FIXED**: `Drop` removes `display.sock` |

### PID file

Not implemented. Low priority per the audit — primalSpring's consumer-side
connect-probe provides equivalent liveness checking.

---

## Changes Made

1. **`crates/cli/src/daemon/server.rs`**: `DaemonServer::shutdown()` now removes
   the socket file. `wait_for_shutdown()` handles both SIGINT and SIGTERM via
   `tokio::signal::unix::signal(SignalKind::terminate())`. Socket path stored on
   the struct for cleanup access.

2. **`crates/runtime/display/src/ipc/server.rs`**: Added `impl Drop for
   DisplayServer` — removes `display.sock` on drop.

3. **5 pre-existing Rust 1.92 clippy fixes**: `if_not_else` (plx.rs),
   `ignored_unit_patterns` (warm_init.rs), `collapsible_if` (sovereign_init.rs),
   `map_unwrap_or` (pcie_keepalive.rs), `default_trait_access` (sovereign.rs).

---

## Socket Lifecycle (complete picture)

```
Startup:
  unlink(compute.sock)          ← remove stale socket
  unlink(compute-tarpc.sock)    ← remove stale tarpc socket
  bind(compute.sock)            ← JSON-RPC listener
  bind(compute-tarpc.sock)      ← tarpc listener
  symlink(toadstool.sock → ...)  ← legacy compat

Shutdown (SIGINT/SIGTERM):
  abort(server_task)
  remove(compute-tarpc.sock)
  remove(compute.sock)
  remove(toadstool.sock)        ← legacy symlink

Crash recovery (next startup):
  unlink(compute.sock)          ← catches stale from crash
  unlink(compute-tarpc.sock)    ← catches stale from crash
  ... normal bind ...
```

---

## Metrics

| Metric | Value |
|--------|-------|
| JSON-RPC methods (direct) | 85 |
| Lib tests | 9,028 |
| Clippy warnings | 0 |
| `cargo deny` | Clean |

toadStool is **stale-socket-clean**.
