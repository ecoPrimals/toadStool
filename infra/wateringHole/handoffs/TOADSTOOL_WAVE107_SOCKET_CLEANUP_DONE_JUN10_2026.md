# toadStool Wave 107 Response — PRIMAL-SOCKET-CLEANUP

**Date**: 2026-06-10
**From**: toadStool (strandGate)
**Re**: Wave 107 — `PRIMAL-SOCKET-CLEANUP` (P2)
**Status**: **DONE**

---

## Wave 107 Item: `PRIMAL-SOCKET-CLEANUP` — CONFIRMED DONE (S308)

### Violation (as reported)

| Path | Type |
|------|------|
| `/tmp/biomeos/compute-tarpc.sock` | Socket |
| `/tmp/toadstool-jsonrpc-port` | Discovery file |

### Fix

`BIOMEOS_SOCKET_DIR` is now the **highest-priority** directory in every socket and
discovery-file resolution chain. When set by NUCLEUS, toadStool writes **zero files
to `/tmp`**.

**Resolution order** (all paths):
1. `BIOMEOS_SOCKET_DIR` (NUCLEUS socket directory)
2. `XDG_RUNTIME_DIR` (user runtime directory)
3. `temp_dir()` (last resort, with warning log)

### Files Changed (S308)

| File | Change |
|------|--------|
| `crates/server/src/unibin/execution.rs` | `write_tcp_discovery_file`: `BIOMEOS_SOCKET_DIR` > `XDG_RUNTIME_DIR` > `temp_dir` |
| `crates/server/src/unibin/execution.rs` | `write_fleet_file`: same chain |
| `crates/server/src/unibin/format.rs` | `get_socket_path`: `BIOMEOS_SOCKET_DIR` checked after `BIOMEOS_SOCKET_PATH` |
| `crates/core/common/src/primal_sockets/env.rs` | `SocketPathEnv`: added `biomeos_socket_dir` field |
| `crates/core/common/src/primal_sockets/paths.rs` | `resolve_biomeos_dir`: respects `biomeos_socket_dir` |
| `crates/core/common/src/platform_paths/env.rs` | `PathEnv`: added `biomeos_socket_dir` field |
| `crates/core/common/src/platform_paths/paths.rs` | `toadstool_socket_dir`: respects `biomeos_socket_dir` |
| `crates/core/toadstool/src/launcher.rs` | Client discovery paths include `BIOMEOS_SOCKET_DIR` |
| `crates/runtime/display/src/ipc/platform.rs` | Display IPC: `BIOMEOS_SOCKET_DIR` first in chain |

### Verification

- `cargo check --all-targets` — clean
- `cargo clippy --workspace --all-targets` — zero new warnings
- `cargo test -p toadstool-common -p toadstool-server` — all pass (2 pre-existing failures in handler_coverage_expansion_s155_tests unrelated)

### Effect on `ProtectSystem=strict`

With `BIOMEOS_SOCKET_DIR=/run/biomeos` (set by NUCLEUS systemd unit), toadStool will
write all sockets and discovery files to `/run/biomeos/` — fully compatible with
`ProtectSystem=strict` which allows `/run` but blocks `/tmp`.

---

## Ecosystem Status (toadStool perspective)

| Item | Status |
|------|--------|
| Transport (`TRANSPORT_ENDPOINT`) | DONE (S301-S302) |
| `PRIMAL-SOCKET-CLEANUP` | **DONE** (S308) |
| Zero production files >750L | DONE (S307) |
| Zero deprecated sync ctors | DONE (S305) |
| Zero production `#[allow]` | DONE |
| `ProtectSystem=strict` compatible | **YES** (when `BIOMEOS_SOCKET_DIR` set) |
