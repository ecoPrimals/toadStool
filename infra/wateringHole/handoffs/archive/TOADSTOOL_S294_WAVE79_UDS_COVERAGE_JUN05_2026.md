# toadStool S294 — Wave 79: UDS Compliance + Coverage Push

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Wave 79 P2 items delivered.

## Changes

### P2 — Binary UDS compliance (FIXED)

**Root cause**: `--socket` CLI arg was parsed by clap, logged, and discarded. `run_server_daemon` never forwarded it to `run_server_main` → `format::get_socket_path()`.

**Fix**: Full parameter threading:

| File | Change |
|------|--------|
| `format.rs` | `get_socket_path()` accepts `cli_override` (precedence #0) and `biomeos_socket_override` |
| `unibin/mod.rs` | `run_server_main()` accepts `socket_override` + `biomeos_socket_override`, passes to path resolution |
| `dispatch/server.rs` | `run_server_daemon()` forwards both socket overrides |
| `dispatch/mod.rs` | Passes `socket.clone()` + `biomeos_socket.clone()` from Server/Daemon commands |
| `main.rs` | Legacy binary updated for new signature |
| `identity.rs` | `primal.announce` reports actual bound path, not re-resolved default |

**Precedence chain**: CLI `--socket` > `TOADSTOOL_SOCKET` env > `PRIMAL_SOCKET` env > CLI `--biomeos-socket` > `BIOMEOS_SOCKET_PATH` env > XDG runtime dir > `/tmp` fallback.

### P2 — Coverage push (+57 tests → 8,952)

| Module | Tests added |
|--------|------------|
| `extract_caller_context` + `resolve_local_gate_id` | 6 — Anonymous, BTSP, MutualAuth, LocalTransport, gate_id resolution |
| `workload.rs` | 8 — submit/cancel/validate param validation, executor forwarding |
| `resources.rs` | 6 — estimate/validate/optimize error paths |
| `dispatch/queries.rs` | 8 — status/result missing params, unknown jobs, happy paths |
| `dispatch/state.rs` | 8 — handler construction, gate_id resolution, state transitions |
| `core/compute.rs` | 5 — version_info, gpu_info, gpu_memory smoke tests |
| `runtime_engine_dispatch.rs` | 8 — supports_workload, get_capabilities, get_metrics, shutdown |
| `ConnectionTrustHints` mutual-auth | 8 — mutual-auth wiring + extraction |

Also wired `mutually_authenticated` field on `ConnectionTrustHints` and `UNIX_MUTUAL_BTSP` constant.

## Metrics

- **8,952** lib tests passed (up from 8,895), 0 failed
- Full workspace clippy `-D warnings` clean
- 27 files changed, 813 insertions, 59 deletions

## Remaining Debt

1. **Coverage**: ~84.5% estimated → target 90%. Next wave: `ember.rs`, `dispatch/submit.rs`, background services, CLI executor gaps
2. **CallerContext full threading**: `identity` + `envelope` still not populated from ionic tokens
3. **LEGACY env removal**: 23 reads now have deprecation tracing; next step is staged removal
4. **tarpc in server defaults**: Still in `default = ["gpu-discovery", "tarpc", "btsp"]`
5. **Files 750L+**: `bar_cartography.rs` (782L), `pmu.rs` (771L) in cylinder
