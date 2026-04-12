# Dispatch Wire Contract — toadStool S203

**Status**: Active  
**Wire Standard Level**: L3 (Composable)  
**Last Updated**: 2026-04-12  

## Purpose

This document defines the **canonical JSON-RPC result shape** for all
`compute.dispatch.*` and `shader.dispatch` methods exposed by toadStool.

primalSpring's `extract_rpc_result<T>` / `extract_rpc_dispatch<T>` require a
**consistent envelope** across all dispatch variants so typed extractors can
validate composition parity in the Node Atomic chain:

```
coralReef compiles → toadStool dispatches → barraCuda reads back
```

## Standard Envelope

Every dispatch response under `result` follows this shape:

```json
{
  "domain": "compute.dispatch",
  "operation": "<operation>",
  "job_id": "<uuid | null>",
  "status": "<status>",
  "output": <any | null>,
  "error": "<string | null>",
  "metadata": { ... }
}
```

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `domain` | `string` | Always `"compute.dispatch"` for all dispatch variants. |
| `operation` | `string` | The operation verb — see table below. |
| `job_id` | `string \| null` | UUID tracking identifier. Null for stateless operations (capabilities, forward). |
| `status` | `string` | One of the canonical status values below. |
| `output` | `any \| null` | Successful result payload. Null when status is not `completed`. |
| `error` | `string \| null` | Error description. Null when status is not `failed` or `partial_failure`. |
| `metadata` | `object` | Operation-specific context (never required for extraction). |

### Canonical Status Values

| Status | Meaning |
|--------|---------|
| `submitted` | Job accepted, execution deferred (no Coral client or async path). |
| `running` | Execution in progress. |
| `completed` | Execution finished successfully; `output` is populated. |
| `failed` | Execution failed; `error` is populated. |
| `partial_failure` | Pipeline: some stages completed, one failed; both `output` and `error` populated. |

### Operation Values

| Wire Method | `operation` Value |
|-------------|-------------------|
| `compute.dispatch.submit` | `submit` |
| `compute.dispatch.status` | `status` |
| `compute.dispatch.result` | `result` |
| `compute.dispatch.forward` | `forward` |
| `compute.dispatch.capabilities` | `capabilities` |
| `compute.dispatch.pipeline.submit` | `pipeline.submit` |
| `compute.dispatch.pipeline.status` | `pipeline.status` |
| `shader.dispatch` | `shader` |

## Per-Operation Details

### `compute.dispatch.submit` (operation: `submit`)

Dispatches a compiled GPU binary to a target device.

**Success (`completed`)**:
```json
{
  "domain": "compute.dispatch",
  "operation": "submit",
  "job_id": "...",
  "status": "completed",
  "output": { /* Coral execute result */ },
  "error": null,
  "metadata": {
    "bdf": "0000:03:00.0",
    "dispatch_mode": "vfio",
    "binary_size": 4096,
    "thermal_checked": true,
    "workgroup_size": [256, 1, 1]
  }
}
```

**Async (`submitted`)**: Same shape, `output: null`, `error: null`.

**Failure (`failed`)**: Same shape, `output: null`, `error: "<message>"`.

### `shader.dispatch` (operation: `shader`)

Dispatches a compiled shader binary with optional readback.

**Success (`completed`)**:
```json
{
  "domain": "compute.dispatch",
  "operation": "shader",
  "job_id": "...",
  "status": "completed",
  "output": { /* Coral execute result */ },
  "error": null,
  "metadata": {
    "bdf": "0000:03:00.0",
    "dispatch_mode": "drm",
    "binary_size": 2048,
    "arch": "sm89",
    "thermal_checked": true,
    "workgroup_size": [64, 1, 1],
    "readback": true
  }
}
```

### `compute.dispatch.status` (operation: `status`)

Queries the current state of a dispatch job.

```json
{
  "domain": "compute.dispatch",
  "operation": "status",
  "job_id": "...",
  "status": "running",
  "output": null,
  "error": null,
  "metadata": {
    "bdf": "0000:03:00.0",
    "binary_size": 4096,
    "elapsed_ms": 142
  }
}
```

### `compute.dispatch.result` (operation: `result`)

Retrieves the output of a completed dispatch job.

```json
{
  "domain": "compute.dispatch",
  "operation": "result",
  "job_id": "...",
  "status": "completed",
  "output": { /* stored result */ },
  "error": null,
  "metadata": {}
}
```

### `compute.dispatch.forward` (operation: `forward`)

Forwards a dispatch to a remote toadStool node.

```json
{
  "domain": "compute.dispatch",
  "operation": "forward",
  "job_id": null,
  "status": "completed",
  "output": { /* remote result */ },
  "error": null,
  "metadata": {
    "endpoint": "192.168.1.10:9090"
  }
}
```

### `compute.dispatch.capabilities` (operation: `capabilities`)

Reports available dispatch capabilities and hardware.

```json
{
  "domain": "compute.dispatch",
  "operation": "capabilities",
  "job_id": null,
  "status": "completed",
  "output": {
    "sovereign_pipeline": true,
    "shader_compiler_available": true,
    "dispatch_modes": ["vfio", "drm"],
    "methods": ["compute.dispatch.submit", "..."],
    "vfio_gpus": [{ "pci_slot": "...", "vendor": "...", "device_id": "..." }],
    "drm_gpus": [{ "pci_slot": "...", "vendor": "...", "driver": "...", "card_index": 0 }],
    "total_dispatch_count": 42
  },
  "error": null,
  "metadata": {}
}
```

### `compute.dispatch.pipeline.submit` (operation: `pipeline.submit`)

Submits a DAG-structured multi-stage pipeline.

**Success (`completed`)**:
```json
{
  "domain": "compute.dispatch",
  "operation": "pipeline.submit",
  "job_id": "...",
  "status": "completed",
  "output": {
    "stage_results": [
      {
        "stage_id": "tokenize",
        "method": "compute.dispatch.submit",
        "substrate": "gpu_preferred",
        "status": "completed",
        "elapsed_ms": 12,
        "result": { /* inner dispatch result */ },
        "error": null
      }
    ]
  },
  "error": null,
  "metadata": {
    "name": "inference_pipeline",
    "stage_count": 3,
    "stages_completed": 3,
    "total_elapsed_ms": 84
  }
}
```

**Partial failure (`partial_failure`)**:
```json
{
  "domain": "compute.dispatch",
  "operation": "pipeline.submit",
  "job_id": "...",
  "status": "partial_failure",
  "output": { "stage_results": [...] },
  "error": "Device lost during FFN stage",
  "metadata": {
    "name": "inference_pipeline",
    "stage_count": 3,
    "stages_completed": 2,
    "failed_stage": "ffn"
  }
}
```

### `compute.dispatch.pipeline.status` (operation: `pipeline.status`)

Queries pipeline execution state.

```json
{
  "domain": "compute.dispatch",
  "operation": "pipeline.status",
  "job_id": "...",
  "status": "completed",
  "output": { "stage_results": [...] },
  "error": null,
  "metadata": {
    "name": "inference_pipeline",
    "stage_count": 3,
    "stages_completed": 3,
    "elapsed_ms": 200
  }
}
```

## Typed Extraction

primalSpring consumers can extract with:

```rust
#[derive(Deserialize)]
struct DispatchEnvelope {
    domain: String,
    operation: String,
    job_id: Option<String>,
    status: String,
    output: Option<serde_json::Value>,
    error: Option<String>,
    metadata: serde_json::Value,
}

let envelope: DispatchEnvelope = extract_rpc_result(&response)?;
match envelope.status.as_str() {
    "completed" => handle_output(envelope.output),
    "failed" => handle_error(envelope.error),
    "submitted" | "running" => poll_later(envelope.job_id),
    _ => { /* partial_failure etc */ }
}
```

## JSON-RPC Error Objects

Parameter validation failures and missing resources use standard JSON-RPC
error objects (`-32602` invalid params, `-32603` internal error) and do NOT
return the envelope above. These map to `IpcError::ProtocolError` in
primalSpring's `extract_rpc_dispatch` (with `should_retry() == false` for
param errors, potentially retryable for internal errors).

## Compliance

This contract satisfies:
- **CAPABILITY_WIRE_STANDARD.md** Level 3 (Composable) — consistent result shapes
- **PRIMAL_IPC_PROTOCOL.md** — standard JSON-RPC 2.0 envelope
- **SEMANTIC_METHOD_NAMING_STANDARD.md** — `{domain}.{operation}` method names
- **Node Atomic composition** — typed extractors can validate parity across
  coralReef → toadStool → barraCuda chain
