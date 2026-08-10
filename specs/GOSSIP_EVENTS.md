# ToadStool — swarmVine Gossip Injection Points

**Wave**: 157e | **Date**: Aug 10, 2026 | **Status**: SPEC

## Overview

ToadStool announces hardware and workload lifecycle events to the swarmVine
gossip mesh. These events enable cross-gate capability discovery, load
balancing, and fault detection. Events are injected via `swarmVine`'s
`gossip.publish` JSON-RPC method.

## Event Taxonomy

Events follow the dotted-capability naming convention used across ecoPrimals.

### Hardware Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `hardware.gpu.added` | GPU becomes available (hot-plug, driver load) | `{ device_id, vendor, model, vram_mb }` |
| `hardware.gpu.removed` | GPU goes offline (hot-unplug, driver error) | `{ device_id, reason }` |
| `hardware.gpu.error` | GPU reports uncorrectable error (ECC, thermal) | `{ device_id, error_class, detail }` |
| `hardware.npu.added` | NPU/accelerator becomes available | `{ device_id, vendor, model }` |
| `hardware.npu.removed` | NPU goes offline | `{ device_id, reason }` |
| `hardware.thermal.warning` | Thermal throttling detected | `{ device_id, temp_celsius, threshold }` |

### Silicon Capability Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `silicon.capabilities.updated` | Silicon registry re-scanned (startup, device change) | `{ gate, units_count, summary }` |
| `silicon.shader_info.updated` | coralReef IPC returned new shader compile capabilities | `{ gate, compiler_version }` |

### Workload Lifecycle Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `workload.submitted` | New workload accepted for dispatch | `{ workload_id, workload_type }` |
| `workload.completed` | Workload execution finished successfully | `{ workload_id, runtime_ms, runtime_type }` |
| `workload.failed` | Workload execution failed | `{ workload_id, error_class }` |
| `workload.queued` | Workload queued (all engines busy) | `{ workload_id, queue_depth }` |

### Runtime Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `runtime.engine.registered` | New runtime engine registered (native, WASM, GPU, etc.) | `{ engine_type, capabilities }` |
| `runtime.engine.removed` | Runtime engine deregistered | `{ engine_type, reason }` |
| `runtime.capacity.changed` | Available compute capacity changed significantly | `{ gate, cpu_free_pct, gpu_free_pct }` |

### Node Atomic Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `node.startup` | toadStool daemon started on this gate | `{ gate, version, capabilities_count }` |
| `node.shutdown` | toadStool daemon shutting down | `{ gate, reason }` |
| `node.health.degraded` | Health check detects degradation | `{ gate, component, detail }` |

## Integration Pattern

Events are published via `swarmVine`'s JSON-RPC interface:

```json
{
  "jsonrpc": "2.0",
  "method": "gossip.publish",
  "params": {
    "topic": "hardware.gpu.added",
    "payload": {
      "device_id": "0000:01:00.0",
      "vendor": "nvidia",
      "model": "RTX 4090",
      "vram_mb": 24576
    },
    "source_gate": "strandGate",
    "source_primal": "toadstool"
  }
}
```

## Manifest Declaration

Gossip events are declared in `biome.yaml` per primal:

```yaml
primals:
  toadstool:
    gossip_events:
      - hardware.gpu.added
      - hardware.gpu.removed
      - silicon.capabilities.updated
      - workload.completed
      - workload.failed
```

## Implementation Status

- **SPEC**: Event taxonomy defined (this document)
- **PENDING**: Wire `gossip.publish` calls into hardware discovery, workload lifecycle, and runtime registration code paths
- **BLOCKED ON**: swarmVine JSON-RPC socket discovery fix (157e subwave)
