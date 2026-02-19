# ToadStool Server — JSON-RPC Method Reference

All methods follow [JSON-RPC 2.0](https://www.jsonrpc.org/specification) over Unix domain socket
(path: `$XDG_RUNTIME_DIR/toadstool.sock` by default) or optionally over TCP.

---

## Namespace Design

ToadStool exposes **two distinct namespaces** for different client personas:

| Namespace     | Backend              | Client persona         | Abstraction level |
|---------------|----------------------|------------------------|-------------------|
| `toadstool.*` | High-level workload executor (`tarpc_server::StandaloneExecutor`) | General orchestrators, BiomeOS, external primals | `WorkloadSpec` — language/runtime agnostic |
| `compute.*`   | Low-level GPU job queue (`gpu_job_queue::GpuJobQueue`) | GPU pipeline clients, Barracuda integration | `JobType` + priority — direct GPU batch control |

These are **not aliases**. A call to `compute.submit` enqueues a raw GPU job
(WGSL shader, buffers, workgroup size). A call to `toadstool.submit_workload`
submits a `WorkloadSpec` that may be routed to GPU, CPU, WASM, or a remote
primal depending on capability discovery.

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

### `toadstool.health`
Health check endpoint.

**Response:** `{ "status": "ok", "uptime_secs": 42, "version": "0.3.0" }`

---

### `toadstool.version`
Version and build information.

**Response:** `{ "version": "0.3.0", "build": "...", "commit": "..." }`

---

## `compute.*` Methods

Direct GPU job queue access. Use these when you need precise control over
GPU dispatch (e.g. from Barracuda or custom compute pipelines). The job
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

## Error Codes

| Code    | Meaning                        |
|---------|-------------------------------|
| `-32700` | Parse error                   |
| `-32600` | Invalid request               |
| `-32601` | Method not found              |
| `-32602` | Invalid params                |
| `-32603` | Internal error                |

---

## Transport

- **Unix socket** (default): `$XDG_RUNTIME_DIR/toadstool.sock`
- **tarpc** (internal IPC): same methods exposed via tarpc binary protocol
  (used for primal-to-primal communication; not intended for external clients)

The Unix socket path can be overridden via the `TOADSTOOL_SOCKET` environment variable
or `socket_path` in the server config.
