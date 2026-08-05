# ToadStool Production Deployment Guide

**Updated**: Jul 29, 2026 — S345 (Wave 155i)
**Binary**: `toadstool` (UniBin — single binary, multiple modes)
**Protocol**: JSON-RPC 2.0 over Unix domain sockets (primary) + tarpc (hot-path)
**License**: AGPL-3.0-or-later

---

## Quick Start

```bash
# Standalone compute node (no Tower required)
TOADSTOOL_STANDALONE=1 toadstool server

# Full Node (Tower running, GPU available)
toadstool server --family-id nat0

# VPS / headless (no GPU)
toadstool server --headless
```

---

## Deployment Models

### Standalone

ToadStool runs independently. No crypto, no coordination, no peer discovery.
Suitable for development, testing, or isolated compute nodes.

```bash
export TOADSTOOL_STANDALONE=1
toadstool server
```

### Node Atomic (Tower + ToadStool)

Tower services (security + coordination) must be running first. ToadStool
discovers them via capability sockets under `$XDG_RUNTIME_DIR/biomeos/`.

```bash
# Prerequisite: Tower services running, providing:
#   $XDG_RUNTIME_DIR/biomeos/crypto.sock        (bearDog / security)
#   $XDG_RUNTIME_DIR/biomeos/coordination.sock   (songBird / discovery)

toadstool server --family-id nat0
```

### NUCLEUS Composition

Full composition: security + coordination + compute + storage. Orchestrated
by biomeOS signal graphs. ToadStool is launched as part of the composition;
the Neural API discovers it by capability.

```bash
# biomeOS launches toadStool with transport injection:
export TRANSPORT_ENDPOINT='{"transport":"uds","path":"/run/biomeos/compute.sock"}'
toadstool server --family-id $BIOMEOS_FAMILY_ID
```

---

## Server Invocation

The server starts with `toadstool server` (recommended) or `toadstool daemon`
(backward-compatible alias — identical code path).

**There is no `start` subcommand.** The server runs in the foreground and
shuts down on SIGINT/SIGTERM.

### CLI Flags

| Flag | Purpose | Default |
|------|---------|---------|
| `--family-id <id>` | Multi-instance socket naming + BTSP production mode | `default` |
| `--port <port>` | TCP JSON-RPC listener (0 = OS-assigned) | 0 (TCP off unless set) |
| `--bind <host:port>` | Full TCP bind override | — |
| `--socket <path>` | JSON-RPC Unix socket override | auto-resolved |
| `--biomeos-socket <path>` | biomeOS socket override | — |
| `--headless` | Skip GPU/NPU hardware probes | false |
| `--config <path>` | Config file path | — |
| `--max-workloads <n>` | Max concurrent workloads | 10 |
| `--register` | Explicit registration (also happens automatically) | — |

### Legacy Binary Names

For backward compatibility, the binary auto-detects its invocation name:

| Invocation | Behavior |
|-----------|----------|
| `toadstool server` | Normal server startup |
| `toadstool-server` (symlink) | Auto-runs server mode |
| `toadstool-byob-server` (symlink) | Auto-runs BYOB HTTP server |

---

## Startup Sequence

```
 1. Version banner + BTSP guard check
 2. Resolve family_id (CLI → TOADSTOOL_FAMILY_ID → BIOMEOS_FAMILY_ID → "default")
 3. Resolve node_id (TOADSTOOL_NODE_ID → "default")
 4. Resolve JSON-RPC socket path
 5. [Linux] Pre-bind socket + spawn early health responder
 6. Load UnibinExecutionConfig from env
 7. Create executor (distributed or standalone)
 8. Build JSON-RPC handler + tarpc server (ready=false)
 9. [Linux] Recover VFIO anchors from systemd fd store
10. Stop early health responder → start full IPC servers
11. [Linux] Start background: PCIe keepalive, catalyst watchdog
12. Register with discovery service (best-effort)
13. Scan for peer primals
14. ready=true → sd_notify(READY=1)
15. Announce capabilities to biomeOS Neural API
16. Block on SIGINT/SIGTERM
17. [Linux] Store VFIO fds in systemd FDSTORE
18. Cleanup sockets
```

---

## Sockets

### Bound by ToadStool

| Socket | Default Path | Protocol |
|--------|-------------|----------|
| JSON-RPC (primary) | `$XDG_RUNTIME_DIR/biomeos/compute.sock` | JSON-RPC 2.0 NDJSON |
| tarpc (hot-path) | `$XDG_RUNTIME_DIR/biomeos/compute-tarpc.sock` | tarpc binary codec |
| TCP (optional) | `127.0.0.1:<port>` | JSON-RPC 2.0 (when `--port` set) |

With `--family-id nat0`:
- `compute-nat0.sock`
- `compute-nat0-tarpc.sock`

### Socket Path Resolution Order

1. CLI `--socket`
2. `TOADSTOOL_SOCKET`
3. `BIOMEOS_SOCKET_PATH`
4. `BIOMEOS_SOCKET_DIR/compute.sock`
5. `$XDG_RUNTIME_DIR/biomeos/compute.sock`
6. `/run/user/<uid>/biomeos/compute.sock`
7. `/tmp/biomeos/compute.sock` (dev fallback only)

### Peer Capability Sockets (discovered, not bound)

| Capability | Socket | Env Override |
|------------|--------|-------------|
| crypto / security | `crypto.sock` | `BEARDOG_SOCKET`, `BIOMEOS_CRYPTO_SOCKET` |
| coordination / discovery | `coordination.sock` | `DISCOVERY_SOCKET`, `SONGBIRD_SOCKET` |
| storage | `storage.sock` | `NESTGATE_SOCKET` |
| routing / ai | `routing.sock` | `SQUIRREL_SOCKET` |

---

## Health Probes

ToadStool responds to health probes **immediately** after socket bind (early
health responder). Full readiness is signaled after init completes.

| Method | During boot | After ready |
|--------|-------------|-------------|
| `health.liveness` | `{"status":"alive"}` | `{"status":"alive"}` |
| `health.readiness` | `{"status":"starting"}` | `{"status":"ready","version":"..."}` |
| `health.check` | Starting envelope | Full health envelope |
| `health` | `{status, primal, version}` | `{status, primal, version}` |

### Probe Commands

```bash
# Liveness (always returns alive if socket reachable)
echo '{"jsonrpc":"2.0","method":"health.liveness","id":1}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock

# Readiness (starting → ready)
echo '{"jsonrpc":"2.0","method":"health.readiness","id":1}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock

# Capabilities
echo '{"jsonrpc":"2.0","method":"capabilities.list","id":1}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock
```

### Probe Timeouts

Use **>=3 seconds** (5s recommended during composition startup). If BTSP is
enabled (`--family-id` set to non-default), add handshake budget
(`BTSP_HANDSHAKE_TIMEOUT_SECS`, default 5s).

---

## Environment Variables

### Identity

| Variable | Purpose | Default |
|----------|---------|---------|
| `TOADSTOOL_FAMILY_ID` | Family ID for BTSP + multi-instance sockets | `default` |
| `TOADSTOOL_NODE_ID` | Node identity | `default` |
| `TOADSTOOL_GATE_ID` | Gate identity for cross-gate trust | hostname |
| `TOADSTOOL_HARDWARE_OWNER_GATE_ID` | Owner gate for yield-to-owner mesh | — |

### Server

| Variable | Purpose | Default |
|----------|---------|---------|
| `TOADSTOOL_SOCKET` | Override JSON-RPC socket path | auto |
| `BIOMEOS_SOCKET_DIR` | Socket directory (for `ProtectSystem=strict`) | auto |
| `TOADSTOOL_BIND_ADDRESS` | TCP bind host | `127.0.0.1` |
| `TOADSTOOL_STANDALONE` | `1` to skip distributed coordinator | unset |
| `TOADSTOOL_HEADLESS` | `1` to skip hardware probes | unset |
| `TOADSTOOL_SOCKET_MODE` | Unix socket permissions (e.g. `0660`) | default |
| `TOADSTOOL_DEPLOYMENT_MODEL` | `multi` / `rental` / unset (LocalDirect) | unset |
| `TRANSPORT_ENDPOINT` | Launcher-injected transport (UDS/TCP JSON) | — |

### Execution

| Variable | Purpose | Default |
|----------|---------|---------|
| `TOADSTOOL_MAX_CONCURRENT_EXECUTIONS` | Executor concurrency | 100 |
| `TOADSTOOL_EXECUTION_TIMEOUT` | Workload timeout (seconds) | 300 |
| `TOADSTOOL_TCP_IDLE_TIMEOUT_SECS` | TCP idle timeout | 300 |

### Security

| Variable | Purpose | Default |
|----------|---------|---------|
| `COORDINATION_AUTH_TOKEN` | Coordination plane auth token | — |
| `BIOMEOS_INSECURE` | `1` for dev mode (conflicts with non-default FAMILY_ID) | unset |
| `TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED` | `1` to suppress startup security warning | unset |
| `BTSP_HANDSHAKE_TIMEOUT_SECS` | BTSP handshake timeout | 5 |

### GPU

| Variable | Purpose | Default |
|----------|---------|---------|
| `TOADSTOOL_GPU_ADAPTER` | GPU selection: index, name substring, or `auto` | `auto` |
| `TOADSTOOL_VFIO_DEVICE` | VFIO device path for sovereign dispatch | — |

Full env reference: `.env.example` and `crates/core/common/src/interned_strings/socket_env.rs`

---

## systemd Integration

### Recommended Unit File

```ini
[Unit]
Description=ToadStool Sovereign Compute
After=network.target beardog.service songbird.service
Wants=beardog.service songbird.service

[Service]
Type=notify
NotifyAccess=main
User=toadstool
Group=biomeos

Environment=BIOMEOS_SOCKET_DIR=/run/biomeos
Environment=TOADSTOOL_FAMILY_ID=nat0

RuntimeDirectory=biomeos
RuntimeDirectoryMode=0750

ExecStart=/usr/local/bin/toadstool server

Restart=always
RestartSec=5

ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
NoNewPrivileges=yes

FileDescriptorStoreMax=16

[Install]
WantedBy=multi-user.target
```

### systemd Features Used

| Feature | Purpose |
|---------|---------|
| `Type=notify` | `sd_notify(READY=1)` after full init |
| `FileDescriptorStoreMax` | VFIO fd store — preserve GPU bindings across restarts |
| `RuntimeDirectory` | Creates `/run/biomeos` with correct permissions |
| `ProtectSystem=strict` | Requires `BIOMEOS_SOCKET_DIR=/run/biomeos` (no `/tmp` fallback) |

### GPU Sovereign Dispatch (VFIO)

For sovereign VFIO dispatch (bypassing kernel GPU driver), additional setup:

```bash
# Run the sovereign GPU setup script (requires root)
sudo scripts/setup-gpu-sovereign.sh
```

The systemd unit will automatically recover VFIO fds from the fd store on restart.

### Headless / VPS

```ini
Environment=TOADSTOOL_HEADLESS=1
Environment=TOADSTOOL_STANDALONE=1
```

---

## Tower Dependency

ToadStool **does not hard-require Tower**. All integrations gracefully degrade:

| Integration | If Tower absent |
|-------------|-----------------|
| Crypto (encrypt dispatch payloads) | Proceeds unencrypted with warning |
| Discovery/coordination registration | Logs warning, runs standalone |
| Distributed coordinator | Set `TOADSTOOL_STANDALONE=1` for explicit standalone |
| biomeOS Neural API announce | Skipped with info log |

For production **Node Atomic** deployment, start Tower services first so
`crypto.sock` and `coordination.sock` exist under `$BIOMEOS_SOCKET_DIR/`.

---

## Deployment Checklist

### Standalone

- [ ] Binary installed (`toadstool` in `$PATH`)
- [ ] `TOADSTOOL_STANDALONE=1` set
- [ ] `toadstool server` starts and `health.liveness` responds
- [ ] GPU detected (or `--headless` for VPS)

### Node Atomic (Tower + Compute)

- [ ] Tower services running (`crypto.sock` + `coordination.sock` exist)
- [ ] `TOADSTOOL_FAMILY_ID` set (enables BTSP production mode)
- [ ] `BIOMEOS_SOCKET_DIR` set (for `ProtectSystem=strict`)
- [ ] `TOADSTOOL_SOCKET_MODE=0660` set (group-shared sockets)
- [ ] `toadstool server --family-id <id>` starts
- [ ] `health.readiness` returns `"ready"`
- [ ] `capabilities.list` returns GPU/NPU capabilities
- [ ] Registration with coordination service confirmed in logs

### NUCLEUS Composition

- [ ] All Node Atomic checks pass
- [ ] `TRANSPORT_ENDPOINT` set by composition launcher
- [ ] biomeOS Neural API discovers toadStool capabilities
- [ ] Signal graph dispatch reaches `compute.dispatch.submit`

---

## Stopping

Send `SIGINT` (Ctrl+C) or `SIGTERM`:

```bash
kill -SIGTERM $(pgrep -f "toadstool server")

# Or with systemd:
systemctl stop toadstool
```

On shutdown, ToadStool:
1. Stores VFIO fds in systemd FDSTORE (if applicable)
2. Cleans up Unix sockets
3. Aborts server tasks
4. Exits cleanly

---

## Diagnostics

```bash
# Check GPU detection
toadstool doctor

# List detected hardware capabilities
toadstool capabilities

# Device status
toadstool device list
toadstool device status
```

---

## BYOB Server (Bring Your Own Biome)

Separate long-running mode for container-based team biome workloads.

```bash
toadstool byob-server --port 9090
```

BYOB uses HTTP (axum), not the primary JSON-RPC IPC path. It provides
deploy/list/stop/resource endpoints for team biome management. This is a
distinct deployment from `toadstool server`.

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [Server Methods](SERVER_METHODS.md) | All 112 JSON-RPC methods |
| [Config Patterns](CONFIG_PATTERNS_GUIDE.md) | Env centralization patterns |
| [Daemon Mode Guide](../daemon/DAEMON_MODE_USER_GUIDE.md) | IPC daemon reference |
| [Capability Discovery (ADR-004)](../architecture/adrs/ADR-004-capability-based-service-discovery.md) | Discovery architecture |
| [Multitenant Architecture](../../specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) | Deployment models |
