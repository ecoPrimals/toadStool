# ToadStool Production Deployment

> **Fossilized (S170).** The November 2025 deployment guide has been archived to
> `ecoPrimals/infra/wateringHole/fossilRecord/TOADSTOOL_PRODUCTION_DEPLOYMENT_GUIDE_NOV2025.md`.
>
> That guide referenced HTTP APIs, Docker, and metrics that no longer apply.
> ToadStool is now IPC-first (JSON-RPC 2.0 over Unix sockets).

## Current Deployment

ToadStool is a single binary (`toadstool`) following the UniBin standard.
Distributed via **plasmidBin** depot (post-primordial mandate, Wave 49).

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `XDG_RUNTIME_DIR` | Socket directory | `/run/user/$UID` |
| `TOADSTOOL_SOCKET` | Override JSON-RPC socket path | biomeOS convention |
| `TOADSTOOL_DEPLOYMENT_MODEL` | `multi` / `rental` for multi-tenant GPU scheduling | unset (LocalDirect) |
| `TOADSTOOL_STANDALONE` | `1` to skip distributed coordinator | unset (distributed) |
| `TOADSTOOL_FAMILY_ID` | Family ID for BTSP production mode | unset (development) |

### Running

```bash
# Daemon mode (Unix socket)
toadstool daemon start

# Daemon mode with TCP fallback
toadstool daemon start --port 8084

# Hardware inventory
toadstool inventory
```

### Health Check

```bash
echo '{"jsonrpc":"2.0","method":"health.check","id":1}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock
```

### Quality Gates

- `cargo clippy --workspace --all-features -- -D warnings` (0 warnings)
- `cargo test --workspace` (20,000+ workspace / 7,842 lib-only tests, 0 failures)
- `cargo fmt --check` (clean)
