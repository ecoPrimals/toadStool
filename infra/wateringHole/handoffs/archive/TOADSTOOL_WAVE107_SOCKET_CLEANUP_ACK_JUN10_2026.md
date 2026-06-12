# toadStool Wave 107 ACK — PRIMAL-SOCKET-CLEANUP DONE

**Date**: 2026-06-10
**From**: toadStool (strandGate)
**Re**: Wave 107 — `TOADSTOOL-SOCKET-CLEANUP` (P2)
**Status**: **DONE — shipped before this cascade landed**

---

## TOADSTOOL-SOCKET-CLEANUP — CONFIRMED DONE (S308, commit 92213e419)

The blurb lists toadStool as "last primal with `/tmp` hardcoding." This was resolved
**before** the Wave 107 cascade was generated:

- **S308** (`92213e419`, pushed to `origin/main`): `BIOMEOS_SOCKET_DIR` wired into all 8
  socket/discovery-file resolution chains
- **S308b** (`2ffe2a20e`, pushed to `origin/main`): Root docs synchronized, handoffs archived,
  debris cleaned

### What S308 Did

| Component | Change |
|-----------|--------|
| `write_tcp_discovery_file()` | `BIOMEOS_SOCKET_DIR` > `XDG_RUNTIME_DIR` > `temp_dir` (with warning) |
| `write_fleet_file()` | Same 3-tier chain |
| `get_socket_path()` | `BIOMEOS_SOCKET_DIR` checked after explicit overrides |
| `resolve_biomeos_dir()` | Respects `biomeos_socket_dir` field |
| `toadstool_socket_dir()` | Respects `biomeos_socket_dir` field |
| `PathEnv` + `SocketPathEnv` | Added `biomeos_socket_dir` field from env |
| Launcher discovery paths | `BIOMEOS_SOCKET_DIR` first in search order |
| Display IPC | `BIOMEOS_SOCKET_DIR` first in discovery chain |

### Verification

```
$ grep -r '"/tmp"' crates/**/src/**/*.rs
(zero matches in production code)
```

Zero `/tmp` string literals in production Rust. `temp_dir()` used only as last-resort
fallback with warning log. When `BIOMEOS_SOCKET_DIR` is set (by NUCLEUS), toadStool
writes **zero files to `/tmp`**.

### Effect

- `ProtectSystem=strict` compatible
- Zero stale socket debris
- Consistent with 4/5 other primals that shipped this wave

---

## toadStool Status

| Item | Status |
|------|--------|
| `TOADSTOOL-SOCKET-CLEANUP` | **DONE** (S308) |
| Transport (`TRANSPORT_ENDPOINT`) | DONE (S301–S302) |
| Zero production files >750L | DONE (S307) |
| Zero deprecated sync ctors | DONE (S305) |
| Zero production `#[allow]` | DONE (S291) |
| Root docs synchronized | DONE (S308b) |
| Tests | 23,000+ (9,069+ lib), 0 failures |
| P1 | **ZERO** |
| P2 | **ZERO** |

**toadStool is 13/13 CLEAN. No remaining debt items.**

---

## Request to Upstream

Please update the Wave 107 ecosystem snapshot:
- `TOADSTOOL-SOCKET-CLEANUP`: DONE → move to "Shipped This Wave"
- `PRIMAL-SOCKET-CLEANUP`: 5/5 (not 4/5)
- toadStool debt column: `Clean` (not `/tmp` pending)
- P2 upstream (code): **1** (biomeOS auto-register only)
