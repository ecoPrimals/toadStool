# ToadStool Daemon Mode - User Guide

> **Deprecation (S169):** The daemon no longer exposes the HTTP API described in this guide (Axum-based server, `curl` examples, and `/api/v1` routes). Workloads are expected to use the JSON-RPC surface over the Unix socket (see `docs/reference/SERVER_METHODS.md`). This document is retained for historical context and needs a full rewrite to match current behavior.

## 🍄 Overview

ToadStool daemon mode transforms the CLI from a direct execution tool into a network-wide compute service. Like the fungus: **same organism, different forms**.

### Two Modes, One System

| Mode | Purpose | Lifecycle | Integration |
|------|---------|-----------|-------------|
| **CLI Mode** (Fruiting body) | Direct project execution | Exits after completion | Standalone |
| **Daemon Mode** (Mycelium) | Ecosystem compute service | Runs continuously | Full primal integration |

## 🚀 Quick Start

### Starting the Daemon

```bash
# Basic daemon (standalone mode)
toadstool daemon

# With biomeOS registration
toadstool daemon --register

# Custom port
toadstool daemon --port 8085

# Full configuration
toadstool daemon \
  --register \
  --port 8084 \
  --max-workloads 20 \
  --biomeos-socket /custom/path/socket.sock
```

### Submitting Workloads

```bash
# Via HTTP API
curl -X POST http://localhost:8084/api/v1/workload/submit \
  -H "Content-Type: application/json" \
  -d '{
    "biome_yaml": "version: 1.0\nservices:\n  myservice:\n    image: ubuntu:22.04",
    "requester": "beardog",
    "environment": {
      "ENV_VAR": "value"
    },
    "persistent": false
  }'

# Response:
# {
#   "workload_id": "550e8400-e29b-41d4-a716-446655440000",
#   "status": "queued",
#   "message": "Workload queued successfully"
# }
```

### Checking Status

```bash
# Get workload status
curl http://localhost:8084/api/v1/workload/550e8400-e29b-41d4-a716-446655440000

# List all workloads
curl http://localhost:8084/api/v1/workloads

# Health check
curl http://localhost:8084/health

# Prometheus metrics
curl http://localhost:8084/metrics
```

### Cancelling Workloads

```bash
# Cancel a running workload
curl -X DELETE http://localhost:8084/api/v1/workload/550e8400-e29b-41d4-a716-446655440000
```

## 📡 HTTP API Reference

### Base URL

```
http://localhost:8084/api/v1
```

### Endpoints

#### POST /api/v1/workload/submit

Submit a new workload for execution.

**Request Body:**

```json
{
  "biome_yaml": "version: 1.0\n...",
  "requester": "string",
  "environment": {
    "KEY": "value"
  },
  "resources": {
    "cpu_limit": 4.0,
    "memory_limit": 4294967296,
    "gpu_required": false
  },
  "timeout_secs": 3600,
  "persistent": false
}
```

**Response:**

```json
{
  "workload_id": "uuid",
  "status": "queued",
  "message": "Workload queued successfully"
}
```

#### GET /api/v1/workload/:id

Get detailed status of a workload.

**Response:**

```json
{
  "workload_id": "uuid",
  "status": "running",
  "started_at": "2026-01-04T12:00:00Z",
  "completed_at": null,
  "exit_code": null,
  "error": null,
  "resource_usage": {
    "cpu_percent": 15.5,
    "memory_bytes": 536870912,
    "gpu_percent": null,
    "storage_bytes": 104857600
  }
}
```

**Status Values:**
- `queued` - Waiting to start
- `running` - Currently executing
- `completed` - Finished successfully
- `failed` - Execution failed
- `cancelled` - Manually cancelled

#### DELETE /api/v1/workload/:id

Cancel a running or queued workload.

**Response:** `204 No Content`

#### GET /api/v1/workloads

List all workloads.

**Response:**

```json
{
  "total": 5,
  "workloads": [
    {
      "workload_id": "uuid",
      "status": "running",
      "requester": "beardog",
      "started_at": "2026-01-04T12:00:00Z",
      "persistent": false
    }
  ]
}
```

#### GET /health

Health check endpoint.

**Response:**

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 3600,
  "active_workloads": 3,
  "biomeos_connected": true
}
```

#### GET /metrics

Prometheus-compatible metrics endpoint.

**Response:**

```prometheus
# HELP toadstool_daemon_uptime_seconds Daemon uptime in seconds
# TYPE toadstool_daemon_uptime_seconds counter
toadstool_daemon_uptime_seconds 3600

# HELP toadstool_workloads_total Total workloads by status
# TYPE toadstool_workloads_total gauge
toadstool_workloads_total{status="queued"} 1
toadstool_workloads_total{status="running"} 2
toadstool_workloads_total{status="completed"} 10
toadstool_workloads_total{status="failed"} 1

# HELP toadstool_biomeos_connected biomeOS connection status (1=connected, 0=disconnected)
# TYPE toadstool_biomeos_connected gauge
toadstool_biomeos_connected 1
```

## 🏗️ Architecture

### Infant Discovery Flow

ToadStool daemon starts with **zero knowledge** and discovers everything at runtime:

1. **Self-Knowledge**: Load own ports and resource info
2. **biomeOS Discovery**: Connect to capability registry (optional, via `--register`)
3. **Capability Registration**: Report what we provide:
   - Compute (wasm, container, python, native, gpu)
   - Storage (local, distributed, encrypted)
   - Orchestration
4. **Dependency Discovery**: Find BearDog (security) and Songbird (routing) by capability
5. **API Server**: Start HTTP server for workload submission
6. **Heartbeat**: Report resources and health to biomeOS (if registered)

### Components

```
┌─────────────────────────────────────────────┐
│           HTTP API Server (Axum)            │
│  - CORS, Tracing middleware                 │
│  - JSON request/response                    │
│  - Error handling                           │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│          Workload Manager                   │
│  - Queue management                         │
│  - Concurrency control (Semaphore)          │
│  - Lifecycle tracking                       │
│  - Resource usage monitoring                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│          BiomeExecutor                      │
│  - Manifest parsing                         │
│  - Runtime selection (WASM, Container, etc) │
│  - Service orchestration                    │
│  - Security integration                     │
└─────────────────────────────────────────────┘
```

### biomeOS Integration

When `--register` is used:

1. Connect to `/tmp/biomeos-registry-{family}.sock`
2. Register ToadStool's capabilities
3. Discover other primals (BearDog, Songbird, NestGate) by capability
4. Periodic heartbeat with resource status
5. Graceful unregister on shutdown

**Standalone Mode**: If biomeOS is unavailable, daemon falls back to standalone mode with no registration.

## 🔧 Configuration

### Command-Line Options

| Option | Default | Description |
|--------|---------|-------------|
| `--register` | `false` | Register with biomeOS capability registry |
| `--port` | `8084` | HTTP API port |
| `--socket` | None | Unix socket path for IPC (optional) |
| `--config` | None | Configuration file path (optional) |
| `--max-workloads` | `10` | Maximum concurrent workloads |
| `--biomeos-socket` | Auto | biomeOS registry socket path (optional) |

### Configuration File

Create `toadstool-daemon.toml`:

```toml
port = 8084
register_with_biomeos = true
max_concurrent_workloads = 20

# Optional
socket_path = "/tmp/toadstool-daemon.sock"
biomeos_socket = "/tmp/biomeos-registry-main.sock"

# Timeouts
default_workload_timeout = 3600  # seconds
resource_monitor_interval = 30   # seconds
heartbeat_interval = 30           # seconds
health_check_interval = 10        # seconds
```

Then start with:

```bash
toadstool daemon --config toadstool-daemon.toml
```

## 📊 Use Cases

### 1. BearDog Requests ML Inference

```python
# BearDog Python client
import requests

response = requests.post(
    "http://toadstool-daemon:8084/api/v1/workload/submit",
    json={
        "biome_yaml": """
version: 1.0
services:
  ml-inference:
    image: python:3.11
    command: python inference.py
    resources:
      gpu: true
""",
        "requester": "beardog",
        "environment": {
            "MODEL_PATH": "/data/fraud-detection.pt"
        }
    }
)

workload_id = response.json()["workload_id"]
print(f"ML inference submitted: {workload_id}")
```

### 2. Multi-Tower Load Balancing

Tower 2 overloaded → discovers Tower 1 via Songbird:

```bash
# Tower 2 offloads to Tower 1
curl -X POST http://tower1-toadstool:8084/api/v1/workload/submit \
  -H "Content-Type: application/json" \
  -d @heavy-workload.json
```

### 3. Persistent Database Service

```bash
# Submit persistent workload (keeps running)
curl -X POST http://localhost:8084/api/v1/workload/submit \
  -d '{
    "biome_yaml": "version: 1.0\nservices:\n  postgres:\n    image: postgres:15\n    persistent: true",
    "requester": "system",
    "persistent": true
  }'
```

### 4. Remote Compute Cluster

```bash
# Submit compute job from laptop to datacenter
curl -X POST https://datacenter-toadstool:8084/api/v1/workload/submit \
  -H "Authorization: Bearer $TOKEN" \
  -d @compute-job.json

# Track progress
watch -n 5 'curl https://datacenter-toadstool:8084/api/v1/workload/$WORKLOAD_ID | jq .status'
```

## 🐛 Troubleshooting

### Daemon won't start

```bash
# Check port availability
netstat -tuln | grep 8084

# Check biomeOS socket
ls -la /tmp/biomeos-registry-*.sock

# Enable verbose logging
toadstool daemon --register -vvv
```

### Workload stuck in "queued"

```bash
# Check active workloads
curl http://localhost:8084/api/v1/workloads | jq '.total'

# Check max concurrent limit
# If at limit, cancel some workloads or increase --max-workloads
```

### biomeOS connection failed

Daemon will log:
```
⚠️  Failed to connect to biomeOS registry: ...
📍 Running in standalone mode
```

This is normal if biomeOS isn't running. Daemon continues without registration.

### Workload execution failed

```bash
# Get error details
curl http://localhost:8084/api/v1/workload/$WORKLOAD_ID | jq .error

# Common issues:
# - Invalid biome.yaml syntax
# - Missing container images
# - Resource limits exceeded
# - Security policy violations
```

## 🔐 Security

### Authentication

**Phase 3**: No authentication (localhost only)  
**Future**: BearDog JWT tokens required for all API requests

### Network Security

- Bind to `0.0.0.0` for cluster access
- Use firewall rules to restrict access
- Enable TLS for production (future)

### Workload Isolation

- Each workload runs in isolated environment
- Resource limits enforced
- Security policies applied via BearDog

## 📈 Monitoring

### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'toadstool-daemon'
    static_configs:
      - targets: ['localhost:8084']
    metrics_path: '/metrics'
```

### Grafana Dashboard

Key metrics:
- `toadstool_daemon_uptime_seconds` - Daemon uptime
- `toadstool_workloads_total{status="running"}` - Active workloads
- `toadstool_workloads_total{status="failed"}` - Failed workloads
- `toadstool_biomeos_connected` - biomeOS connection status

## 🍄 Philosophy

> "Like the fungus: Same organism, different forms"

**CLI Mode** = **Fruiting body**
- Specialized structure for specific task
- Project-specific execution
- Emerges when needed
- Returns nutrients (results) to developer

**Daemon Mode** = **Mycelium network**
- Persistent underground presence
- Resource sharing across ecosystem
- Network effects through universal adapter
- Enables multi-tower coordination

**Same ToadStool core, adapted to environment!**

## 📚 Next Steps

- [Architecture Deep Dive](../architecture/DAEMON_MODE_EVOLUTION.md)
- [Production Deployment](../reference/PRODUCTION_DEPLOYMENT_GUIDE.md)

## 🎯 Summary

ToadStool daemon mode provides:

✅ **HTTP API** for workload submission  
✅ **biomeOS Integration** for ecosystem coordination  
✅ **Workload Management** with lifecycle tracking  
✅ **Resource Monitoring** for load balancing  
✅ **Infant Discovery** for zero-hardcoded architecture  
✅ **Prometheus Metrics** for observability  
✅ **Graceful Degradation** for reliability  

Transform your ToadStool from a CLI tool to an ecosystem compute service in one command:

```bash
toadstool daemon --register
```

🍄 **Welcome to the mycelium network!**

