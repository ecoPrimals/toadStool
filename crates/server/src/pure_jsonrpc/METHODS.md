# JSON-RPC Method Reference

ToadStool exposes two distinct namespaces over the same JSON-RPC 2.0 endpoint.
Both are routed through `JsonRpcHandler::handle_method`. Semantic aliases (e.g.
`runtime.workload.submit`) are resolved by `SemanticMethodRegistry` before
dispatch and ultimately forward to one of the implementations below.

---

## `toadstool.*` — Workload Executor

These methods interact with the **ToadStool workload executor** (`WorkloadExecutor`
trait), which manages the full lifecycle of compute workloads: submission,
queuing, status polling, and cancellation.

| Method | Params | Returns | Notes |
|---|---|---|---|
| `toadstool.submit_workload` | `JsonWorkloadSubmission` | `WorkloadResult` | Submit a compute workload |
| `compute.execute` | `JsonWorkloadSubmission` | `WorkloadResult` | Alias for `toadstool.submit_workload` |
| `toadstool.query_status` | `"<workload_id>"` | `Job` | Poll workload status by UUID string |
| `toadstool.cancel_workload` | `"<workload_id>"` | `{"success": true}` | Cancel by UUID string |
| `toadstool.list_workloads` | none | `{"jobs": [...], "counts": {...}}` | List all workloads |
| `toadstool.query_capabilities` | none | `Capabilities` | Self-knowledge: what this node can do |
| `toadstool.health` | none | `HealthStatus` | Liveness + uptime check |
| `toadstool.version` | none | `{"version": "...", ...}` | Protocol version info |

---

## `compute.*` — GPU Job Queue

These methods interact with the **GPU job queue** (`GpuJobQueue`), a dedicated
priority queue for GPU-accelerated jobs (FHE, matrix ops, shader workloads).
This is a **distinct subsystem** from the workload executor.

| Method | Params | Returns | Notes |
|---|---|---|---|
| `compute.submit` | `JobType` + optional `priority: u32` | `{"job_id": "<uuid>"}` | Submit to GPU queue |
| `compute.status` | `{"job_id": "<uuid>"}` | `Job` | Poll GPU job status |
| `compute.result` | `{"job_id": "<uuid>"}` | `Value` | Retrieve GPU job output |
| `compute.cancel` | `{"job_id": "<uuid>"}` | `{"cancelled": true}` | Cancel GPU job |
| `compute.list` | optional `{"state": "<JobState>"}` | `{"jobs": [...], "counts": {...}}` | List GPU jobs |

---

## Semantic Aliases

`SemanticMethodRegistry` maps additional names to the `toadstool.*`
implementation names. This enables callers using the wateringHole
`{domain}.{operation}` convention.

| Alias | Resolves to |
|---|---|
| `runtime.workload.submit` | `toadstool.submit_workload` |
| `runtime.workload.status` | `toadstool.query_status` |
| `runtime.workload.cancel` | `toadstool.cancel_workload` |
| `runtime.workload.list` | `toadstool.list_workloads` |
| `runtime.capabilities` | `toadstool.query_capabilities` |

---

## Choosing the Right Namespace

| Use case | Namespace |
|---|---|
| Submitting a general compute workload (CPU, GPU, WASM, container) | `toadstool.*` |
| Submitting a raw GPU shader or FHE operation to the job queue | `compute.*` |
| Health checks, version info | `toadstool.*` |
| Capability discovery | `toadstool.*` |

> **Note**: `toadstool.query_status` and `compute.status` return different
> response shapes — the former is a `WorkloadResult`, the latter is a `GpuJob`.
> Use the namespace that matches how the job was submitted.

---

## IPC Contract: Pre-Resolved Values

All JSON-RPC methods expect **pre-resolved** parameter values. The server does
**not** perform `${VAR}` / `$VAR` environment variable expansion on any string
fields — paths, identifiers, metadata values, or BDF addresses must be fully
resolved by the caller before submission.

Environment variable expansion is a **CLI-only** convenience provided by
`load_workload_file` for locally-authored workload TOML/JSON specs. This
separation is intentional: in cross-primal composition, the server's process
environment differs from the caller's, and implicit expansion would create
ambiguity about whose environment applies.

| Path | Env expansion? |
|------|----------------|
| CLI `toadstool execute <file.toml>` | **Yes** — expands `${VAR}` in file text before parse |
| JSON-RPC `compute.execute` / `toadstool.submit_workload` | **No** — pre-resolved only |
| JSON-RPC `compute.dispatch.submit` | **No** — pre-resolved only |
| JSON-RPC `compute.dispatch.pipeline.submit` | **No** — pre-resolved only |

Graph specs and composition callers should use absolute paths or pre-expand
variables on the client side before sending structured JSON-RPC requests.

---

## `compute.dispatch.submit` — Trio-Standard IPC Contract (S235)

Accepts both legacy and trio-standard parameter formats (backward-compatible).

### Request params

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `binary_b64` | string (base64) | One of `binary_b64` or `binary` | **Preferred** — base64-encoded compiled binary |
| `binary` | array (u8) | One of `binary_b64` or `binary` | Legacy — JSON u8 array |
| `bdf` | string | No | PCI BDF address (e.g. `"0000:03:00.0"`) |
| `dispatch_dims` | array [x,y,z] | No | **Preferred** — dispatch dimensions |
| `workgroup_size` | array [x,y,z] | No | Legacy — fallback if `dispatch_dims` absent |
| `shader_info` | object | No | `{ gprs, shared_memory, barriers, workgroup, wave_size }` |
| `buffers` | array | No | `[{ binding, data_b64?, data?, size, usage }]` — `data_b64` auto-decoded |
| `dispatch_mode` | string | No | `"vfio"`, `"drm"`, or `"passthrough"` |
| `timeout_ms` | u64 | No | Default: 30s |

When both `binary_b64` and `binary` are present, `binary_b64` takes precedence.
When both `dispatch_dims` and `workgroup_size` are present, `dispatch_dims` takes precedence.
Buffer entries with `data_b64` are auto-decoded; the `data_b64` field is replaced with `data`.

### Response

```json
{
  "domain": "compute.dispatch",
  "operation": "submit",
  "job_id": "<uuid>",
  "status": "submitted|completed|failed",
  "output": ...,
  "error": null,
  "timing": { "dispatch_ms": 42, "readback_ms": 5 },
  "metadata": { "bdf": "...", "binary_size": 1024, "workgroup_size": [256,1,1], "shader_info": ... }
}
```

### `compute.dispatch.capabilities` — Gate 2 Hardware Info

Returns hardware inventory including architecture detection.

```json
{
  "output": {
    "gpu_count": 2,
    "architectures": ["sm75", "sm80"],
    "vfio_status": { "available": true, "device_count": 1 },
    "vfio_gpus": [{ "pci_slot": "...", "vendor": "Nvidia", "device_id": "0x1e04", "architecture": "sm75" }],
    "drm_gpus": [{ "pci_slot": "...", "vendor": "Amd", "driver": "amdgpu", "card_index": 1, "architecture": "rdna3" }],
    ...
  }
}
```
