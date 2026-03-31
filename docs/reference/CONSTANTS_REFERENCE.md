# ToadStool Constants Reference

> **Fossilized (S170).** The November 2025 constants reference has been archived to
> `ecoPrimals/infra/wateringHole/fossilRecord/TOADSTOOL_CONSTANTS_REFERENCE_NOV2025.md`.
>
> Port constants have been replaced by capability-based resolution (S169/S170).

## Port Resolution (S170)

Ports are resolved via `toadstool_config::ports::resolve_capability_port()`:

1. `TOADSTOOL_{CAPABILITY}_PORT` env var
2. `{CAPABILITY}_PORT` env var
3. Compiled fallback from `capability_fallback` constants

| Capability | Env Var | Fallback |
|-----------|---------|----------|
| Daemon API | `TOADSTOOL_DAEMON_API_PORT` | `0` (ephemeral) |
| Coordination (Songbird) | `COORDINATION_PORT` | `8080` |
| Security (BearDog) | `SECURITY_PORT` | `8082` |
| Storage (NestGate) | `STORAGE_PORT` | `8083` |
| Platform (Squirrel) | `PLATFORM_PORT` | `8081` |

## Socket Paths

| Socket | Path |
|--------|------|
| Daemon | `$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock` |
| coralReef discovery | `$XDG_RUNTIME_DIR/biomeos/coralreef*.sock` (scanned) |
| Capability discovery | `$XDG_RUNTIME_DIR/ecoPrimals/{capability}.sock` |

## Configuration Module

```rust
use toadstool_config::ports::{resolve_capability_port, daemon_port, capability_fallback};

let port = resolve_capability_port("COORDINATION", capability_fallback::COORDINATION);
let daemon = daemon_port(); // 0 by default (OS-assigned)
```

See `crates/core/config/src/ports.rs` for the full implementation.
