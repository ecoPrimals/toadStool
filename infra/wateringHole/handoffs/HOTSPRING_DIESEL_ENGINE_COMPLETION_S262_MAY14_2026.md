# hotSpring Diesel Engine Completion — toadStool Response (S262)

**Date**: May 14, 2026
**Session**: S262
**From**: toadStool team
**To**: hotSpring team (diesel engine completion sprint)

---

## Critical Path Item 1: `device.gr.init` IPC Surface — SHIPPED

**What was requested**: Expose `init_gr_context()` over JSON-RPC so hotSpring can feed GR
register sequences from experiments 184-190 to initialize warm-caught Titan V / K80.

**What shipped**:

### `device.gr.init` / `compute.context.init` JSON-RPC endpoint

Accepts GR context initialization method entries and submits them as a pushbuffer
to the warm-caught GPU.

**Params:**
```json
{
  "bdf": "0000:01:00.0",
  "method_entries": [
    [2304, 4660],
    [2308, 22136]
  ]
}
```

- `method_entries` is an array of `[register_address, value]` pairs (decimal or hex via JSON)
- Minimum 1 entry required
- Returns `{ "status": "completed", "entries_submitted": N, "timing": { "init_ms": ... } }`
- On failure returns `{ "status": "failed", "error": "..." }`

**Semantic aliases**: `ember.gr.init`, `sovereign.gr.init`, `device.gr.init`, `compute.context.init`

### Optional `gr_init_entries` on `device.vfio.roundtrip`

For single-call GR init + dispatch:

```json
{
  "bdf": "0000:01:00.0",
  "gr_init_entries": [[2304, 4660], [2308, 22136]],
  "binary_b64": "...",
  "shader_info": { "gprs": 32, "shared_memory": 16384 },
  "dispatch_dims": [256, 1, 1]
}
```

The GR init runs before dispatch in the same VFIO session.

### Architecture change

`init_gr_context()` promoted from inherent method on `NvVfioComputeDevice` to `ComputeDevice`
trait method with default `Unsupported` return. NVIDIA impl submits via GPFIFO pushbuffer.
AMD and other devices cleanly return `Unsupported`.

---

## Critical Path Item 2: Shader Metadata Aliases — SHIPPED

**What was requested**: Wire coralReef `CompilationInfoResponse` metadata fields into QMD
so the compile→dispatch pipeline completes without field name translation.

**What shipped**:

`resolve_shader_info()` helper accepts both field naming conventions:

| coralReef name | toadStool name (preferred) | QMD field |
|---------------|---------------------------|-----------|
| `gprs` | `gpr_count` | GPR count |
| `shared_memory` | `shared_mem_bytes` | Shared memory bytes |
| `barriers` | `barrier_count` | Barrier count |
| `local_memory` | `local_mem_bytes` | Per-thread local memory |
| `wave_size` | `wave_size` | (same) |

When both names are present, the toadStool-native name takes precedence.

Applied across all three dispatch paths:
- `try_local_dispatch` (used by `compute.dispatch.submit`)
- `device.vfio.roundtrip`
- Internal shader dispatch

**E2E flow now possible**: coralReef `shader.compile.wgsl` → hotSpring passes
`CompilationInfoResponse.shader_info` directly to `compute.dispatch.submit` or
`device.vfio.roundtrip` → toadStool builds QMD with correct GPR/shared_mem/barrier
counts → GPFIFO submit → GPU execution.

---

## Hardware E2E Validation Path

With items 1+2 shipped, the full pipeline is IPC-ready:

```
hotSpring exp184 GR entries
    ↓ device.gr.init
coralReef shader.compile.wgsl
    ↓ shader binary + shader_info
toadStool compute.dispatch.submit (or device.vfio.roundtrip)
    ↓ QMD build → GPFIFO → PBDMA
GPU execution
    ↓ sync → readback
hotSpring validates results
```

**Ready on each GPU:**
- RTX 5060: wgpu/Vulkan baseline — no GR init needed
- Titan V: `device.gr.init` with GV100 entries → then dispatch
- K80: Kepler NoAcr — no GR init needed (direct GR channel)

---

## Metrics

| Metric | Value |
|--------|-------|
| JSON-RPC methods (direct) | 83 |
| Lib tests | 8,849 |
| New tests this session | 8 |
| Clippy warnings | 0 |
| `cargo deny check bans` | Clean |
| Production files >800L | 0 |
