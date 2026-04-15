# ToadStool Server — JSON-RPC Method Reference

All methods follow [JSON-RPC 2.0](https://www.jsonrpc.org/specification) over Unix domain sockets
or optionally over TCP.

## Transport

ToadStool binds **two Unix sockets** plus an optional TCP listener:

| Transport | Path | Protocol | Use case |
|-----------|------|----------|----------|
| JSON-RPC (primary) | `$XDG_RUNTIME_DIR/biomeos/compute.sock` | JSON-RPC 2.0 (newline-delimited) | External clients, `socat`, springs, biomeOS Neural API |
| tarpc (hot-path) | `$XDG_RUNTIME_DIR/biomeos/compute-tarpc.sock` | tarpc binary (Tokio codec) | High-perf primal-to-primal IPC (Rust-to-Rust) |
| TCP (optional) | `0.0.0.0:<port>` | JSON-RPC 2.0 | Cross-host access (`--port`) |

The two protocols use **separate sockets** to avoid bind collision and allow independent
lifecycle management. JSON-RPC is the universal entry point; tarpc is the optional
performance channel for Rust peers.

Socket paths can be overridden:

- `TOADSTOOL_SOCKET` env var — overrides JSON-RPC socket path
- `TOADSTOOL_TARPC_SOCKET` env var — overrides tarpc socket path
- `--socket <PATH>` CLI flag
- `--family-id <ID>` — creates `compute-{ID}.sock` / `compute-{ID}-tarpc.sock`

biomeOS routes to `compute.sock` for capability-based dispatch. Clients requesting
tarpc performance should resolve via `get_toadstool_tarpc_socket_path()` or the
`TOADSTOOL_TARPC_SOCKET` env var.

### Starting the Server

```bash
# Recommended (UniBin standard naming)
toadstool server

# With options
toadstool server --port 9090 --register --family-id lab01

# Backward-compatible alias
toadstool daemon
```

Stopping: send `SIGINT` or `SIGTERM` to the process.

---

## Namespace Design

ToadStool exposes **two distinct namespaces** for different client personas:

| Namespace     | Backend              | Client persona         | Abstraction level |
|---------------|----------------------|------------------------|-------------------|
| `toadstool.*` | High-level workload executor (`tarpc_server::StandaloneExecutor`) | General orchestrators, biomeOS, external primals | `WorkloadSpec` — language/runtime agnostic |
| `compute.*`   | Low-level GPU job queue (`gpu_job_queue::GpuJobQueue`) | GPU pipeline clients, compute service integration | `JobType` + priority — direct GPU batch control |

These are **not aliases**. A call to `compute.submit` enqueues a raw GPU job
(WGSL shader, buffers, workgroup size). A call to `toadstool.submit_workload`
submits a `WorkloadSpec` that may be routed to GPU, CPU, WASM, or a remote
primal depending on capability discovery.

---

## Discovery & Health Methods

These are the entry points biomeOS and primalSpring use for live validation.

| Method | Aliases | Description |
|--------|---------|-------------|
| `capabilities.list` | `capability.list`, `primal.capabilities`, `compute.capabilities` | Returns compute capabilities for this instance |
| `compute.discover_capabilities` | — | Returns full method list + semantic mappings |
| `health.liveness` | `health.readiness`, `health.check`, `toadstool.health`, `compute.health` | Health check with uptime, version, error count |
| `identity.get` | — | Returns primal identity + registered semantic methods |
| `toadstool.version` | `compute.version` | Version and build info |

---

## `toadstool.*` Methods

### `toadstool.submit_workload`
Submit a high-level workload for execution.

**Request params:**
```json
{
  "workload_id": "uuid",
  "workload_type": "Native|Wasm|Container|Gpu",
  "payload": { ... },
  "priority": "Normal|High|Critical",
  "requirements": { "min_memory_gb": 2, "requires_gpu": false }
}
```

**Response:**
```json
{ "execution_id": "uuid", "status": "Queued|Running|Success|Failed" }
```

---

### `toadstool.query_status`
Query the execution status of a previously submitted workload.

**Request params:** `"<execution_id>"`

**Response:** `ExecutionResponse` object (includes stdout, stderr, exit_code, metrics)

---

### `toadstool.list_workloads`
List all workloads with optional filtering.

**Request params:** `{ "status_filter": "Running" }` (optional)

**Response:** Array of `ExecutionResponse`

---

### `toadstool.cancel_workload`
Cancel a running or queued workload.

**Request params:** `"<execution_id>"`

**Response:** `{ "cancelled": true }`

---

### `toadstool.query_capabilities`
Query what execution capabilities this ToadStool instance has.

**Response:** `CapabilityResponse` object

---

### `toadstool.resources.*`

| Method | Description |
|--------|-------------|
| `toadstool.resources.estimate` / `resources.estimate` | Estimate resource requirements for a workload |
| `toadstool.resources.validate_availability` / `resources.validate_availability` | Validate resource availability |
| `toadstool.resources.suggest_optimizations` / `resources.suggest_optimizations` | Suggest resource optimizations |
| `toadstool.ai.local_inference` / `ai.local_inference` | Alias for `resources.estimate` |
| `toadstool.ai.local_execute` / `ai.local_execute` | Alias for `resources.validate_availability` |

---

## `compute.*` Methods

Direct GPU job queue access. Use these when you need precise control over
GPU dispatch (e.g. from the compute service or custom pipelines). The job
queue runs independently of the workload executor.

### `compute.submit`
Enqueue a raw GPU job.

**Request params:**
```json
{
  "job_type": { "Wgsl": { "shader": "...", "entry_point": "main", "buffers": [...] } },
  "priority": 0
}
```

**Response:** `{ "job_id": "uuid" }`

---

### `compute.status`
Get the status of a GPU job.

**Request params:** `{ "job_id": "uuid" }`

**Response:** `GpuJob` object (includes `state: Queued|Running|Complete|Failed`)

---

### `compute.result`
Retrieve the result of a completed GPU job.

**Request params:** `{ "job_id": "uuid" }`

**Response:** Serialized result buffer

---

### `compute.cancel`
Cancel a pending GPU job.

**Request params:** `{ "job_id": "uuid" }`

**Response:** `{ "cancelled": true }`

---

### `compute.list`
List GPU jobs, optionally filtered by state.

**Request params:** `{ "state": "Running" }` (optional)

**Response:** `{ "jobs": [...], "counts": { "queued": 3, "running": 1, "complete": 42 } }`

---

### `compute.dispatch.*`

Low-level shader dispatch (VFIO/DRM passthrough).

| Method | Description |
|--------|-------------|
| `compute.dispatch.submit` | Submit a dispatch job |
| `compute.dispatch.status` | Query dispatch job status |
| `compute.dispatch.result` | Retrieve dispatch result |
| `compute.dispatch.forward` | Forward dispatch to another gate |
| `compute.dispatch.capabilities` | Query dispatch capabilities |

---

### `compute.hardware.*`

Hardware learning and auto-initialization (biomeOS v2.30 compute.hardware capabilities).

| Method | Description |
|--------|-------------|
| `compute.hardware.observe` | Observe hardware behavior for learning |
| `compute.hardware.distill` | Distill observations into recipes |
| `compute.hardware.apply` | Apply a hardware recipe |
| `compute.hardware.share_recipe` | Share recipe with peer gates |
| `compute.hardware.auto_init` | Auto-initialize a device from learned recipes |
| `compute.hardware.auto_init_all` | Auto-initialize all devices |
| `compute.hardware.status` | Hardware learning subsystem status |
| `compute.hardware.vfio_devices` | List VFIO-bound devices |

---

### `compute.performance_surface.*`

Silicon performance surface reporting and routing.

| Method | Description |
|--------|-------------|
| `compute.performance_surface.report` | Report performance surface data |
| `compute.performance_surface.query` | Query surface for a workload profile |
| `compute.performance_surface.list` | List available performance surfaces |
| `compute.route.multi_unit` | Route across multiple functional units |

---

## `gpu.*` Methods

| Method | Description |
|--------|-------------|
| `gpu.query_info` / `gpu.info` | GPU adapter info (driver, f64 support, workgroups) |
| `gpu.query_memory` / `gpu.memory` | GPU memory info (total, available, used) |
| `gpu.query_telemetry` / `gpu.telemetry` | GPU telemetry (temperature, utilization) |

---

## `gate.*` Methods

Distributed cross-gate routing.

| Method | Description |
|--------|-------------|
| `gate.update` | Register or update a remote gate |
| `gate.remove` | Remove a gate from the routing table |
| `gate.list` | List known gates |
| `gate.route` | Route a job to a specific gate |

---

## `transport.*` Methods

Hardware transport discovery and routing (DRM, V4L2, serial).

| Method | Description |
|--------|-------------|
| `transport.discover` | Discover available hardware transports |
| `transport.list` | List active transports |
| `transport.route` | Route data through a transport |
| `transport.open` | Open a transport channel |
| `transport.stream` | Stream data through a transport |
| `transport.status` | Query transport status |

---

## `ember.*` Methods

glowPlug/ember device lifecycle management.

| Method | Description |
|--------|-------------|
| `ember.list` | List managed GPU devices |
| `ember.status` | Device manager status |

---

## `shader.dispatch`

Sovereign shader dispatch: send a compiled shader binary to GPU hardware
via VFIO or DRM passthrough. Input formats: base64, byte array, or
`compile_result` object from the visualization service.

---

## `provenance.*` Methods

| Method | Aliases | Description |
|--------|---------|-------------|
| `provenance.query` | `provenance.get`, `toadstool.provenance` | Cross-spring evolution provenance matrix |

---

## Error Codes

| Code    | Meaning                        |
|---------|-------------------------------|
| `-32700` | Parse error                   |
| `-32600` | Invalid request               |
| `-32601` | Method not found              |
| `-32602` | Invalid params                |
| `-32603` | Internal error                |

---

## Semantic Method Resolution

Methods can also be invoked using semantic names (`{domain}.{operation}`)
via the `SemanticMethodRegistry`. The handler resolves semantic names to
implementation names before dispatch:

1. Direct literal match (backward-compatible `toadstool.*` and `compute.*` names)
2. Semantic registry lookup: `{domain}.{operation}` → implementation name → handler

Use `compute.discover_capabilities` to retrieve the full list of registered
semantic method mappings.

---

## CLI Flags (Server Mode)

```
toadstool server [OPTIONS]

Options:
  --register            Register with biomeOS capability registry
  --port <PORT>         JSON-RPC TCP port (0 = OS-assigned; default from config)
  --socket <PATH>       Unix socket path override
  --config <PATH>       Configuration file
  --max-workloads <N>   Maximum concurrent workloads (default: 10)
  --biomeos-socket <PATH>  biomeOS registry socket path
  --family-id <ID>      Family ID for multi-family socket support
```

`toadstool daemon` accepts the same flags (backward-compatible alias).
