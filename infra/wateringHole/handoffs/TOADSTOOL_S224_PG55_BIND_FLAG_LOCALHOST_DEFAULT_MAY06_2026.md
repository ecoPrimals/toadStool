# ToadStool S224 — PG-55: `--bind` Flag + Localhost Default

**Date**: May 6, 2026
**Session**: S224

---

## Summary

Resolved PG-55 (HIGH) from primalSpring downstream audit: TCP bind address
defaulted to `0.0.0.0` (all interfaces), exposing workload submission on LAN.
Added `--bind host:port` CLI flag and changed default to `127.0.0.1` (loopback).
Follows barraCuda's pattern.

---

## Changes

### 1. `--bind` CLI Flag (Server + Daemon)

Added `--bind` option to both `Commands::Server` and `Commands::Daemon`
subcommands. When provided:
- Parses as `host:port` — overrides both `--port` and `TOADSTOOL_BIND_ADDRESS`
- Host-only values (no `:port` suffix) override only the bind host

Priority chain: `--bind` > `TOADSTOOL_BIND_ADDRESS` env > default `127.0.0.1`.

### 2. Default Bind Address: `0.0.0.0` → `127.0.0.1`

Three locations changed:
- `UnibinExecutionConfig::from_env()` — fallback when `TOADSTOOL_BIND_ADDRESS` unset
- `daemon/jsonrpc_server.rs` — daemon TCP JSON-RPC bind host
- `BIND_ADDRESS_DEFAULT` constant — config-level default

`TOADSTOOL_BIND_ADDRESS=0.0.0.0` can still be set explicitly for cross-host access.

### 3. Wiring

- `run_server_main()` now accepts `bind_override: Option<String>` (2nd param)
- `run_server_daemon()` accepts and forwards `bind_override`
- Legacy `toadstool-server` binary path passes `None` for bind (localhost default)
- `--bind` parsing uses `rsplit_once(':')` to separate host from port

---

## Files Changed

- `crates/cli/src/commands/definitions.rs` — `--bind` field on Server + Daemon
- `crates/cli/src/commands/dispatch/mod.rs` — wire bind through match arms
- `crates/cli/src/commands/dispatch/server.rs` — accept bind_override param
- `crates/cli/src/main.rs` — legacy binary path passes None for bind
- `crates/server/src/unibin/mod.rs` — bind override parsing + forwarding
- `crates/server/src/unibin/execution.rs` — default `BIND_ALL_IPV4` → `LOCALHOST_IPV4`
- `crates/cli/src/daemon/jsonrpc_server.rs` — default `BIND_ALL_IPV4` → `LOCALHOST_IPV4`
- `crates/core/config/src/defaults/network.rs` — `BIND_ADDRESS_DEFAULT` → `127.0.0.1`
- Tests: `unibin/tests.rs`, `unibin_unit_tests.rs`, `dispatch_coverage_tests.rs`,
  `command_variants.rs`, `unibin_deep_coverage_s172_tests.rs`, `defaults_test.rs`

---

## Test Count

| Metric | Before | After |
|--------|--------|-------|
| Workspace tests | 22,833 | 22,838 |
| New tests | — | 5 (2 server config, 3 CLI flag parsing) |
| Failures | 0 | 0 |

---

## Security Impact

- **Before**: `toadstool server --port 9400` binds `0.0.0.0:9400` — any LAN host
  can submit workloads
- **After**: `toadstool server --port 9400` binds `127.0.0.1:9400` — loopback only
- **Cross-host**: `toadstool server --bind 0.0.0.0:9400` or
  `TOADSTOOL_BIND_ADDRESS=0.0.0.0` for intentional LAN exposure
