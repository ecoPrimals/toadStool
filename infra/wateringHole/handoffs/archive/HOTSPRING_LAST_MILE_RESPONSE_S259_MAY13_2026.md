# hotSpring Last Mile Audit Response — toadStool S259

**Date**: May 13, 2026
**Audit**: hotSpring Sovereign Compute — Upstream Evolution Blurb
**Responding primal**: toadStool
**Session**: S259

---

## Audit Items — Resolution Status

### 1. Wire `NvVfioComputeDevice` as JSON-RPC endpoints — RESOLVED

**What hotSpring asked**: Surface the S258 library code as daemon IPC: `device.vfio.open`, `device.vfio.roundtrip`, and `compute.dispatch.submit` must return a `job_id`.

**What toadStool shipped**:

- **`device.vfio.open`** (direct + semantic `ember.vfio.open`): Opens VFIO device by BDF via `LocalDeviceFactory`, returns capabilities (vendor, device name, generation, F64 support, shared mem).
- **`device.vfio.roundtrip`** (direct + semantic `ember.vfio.roundtrip`): Full alloc→upload→dispatch→sync→readback in one call. Returns `job_id` + inline results + timing.
- **`compute.dispatch.submit`**: Already returns `job_id` (established S258). Phase D local dispatch now goes through QMD-based path with `open_vfio()` called in factory.
- **Factory wiring**: `try_vfio_nvidia()` now calls `open_vfio()` after warm FECS detection — devices returned from the factory are dispatch-ready.

**Callable methods**: Any spring can now dispatch on VFIO-bound NVIDIA GPUs via IPC. No longer limited to in-process hotSpring use.

### 2. Compute QMD builder — RESOLVED

**What hotSpring asked**: Build Compute Queue Meta Data (SET_SHADER_PROGRAM, LAUNCH, QMD headers) and package into GPFIFO push buffers.

**What toadStool shipped**:

The QMD builder infrastructure already existed in `cylinder/src/nv/qmd/` (all versions v2.1–v5.0) and `cylinder/src/nv/pushbuf.rs`. This session plumbed them through `NvVfioComputeDevice::dispatch()`:

1. **Shader binary upload** → DMA buffer at dedicated IOVA
2. **CBUF descriptor table** → 16-byte-stride entries (va_lo, va_hi, size, pad) for each user buffer
3. **Driver constants** → `encode_driver_constants()` writes grid dims for `@builtin(num_workgroups)`
4. **QMD build** → `build_qmd()` selects version per `GenerationProfile` (v2.2 Volta, v2.3 Ampere, v3.0 Ada, v5.0 Blackwell)
5. **Push buffer** → `PushBuf::compute_init()` (SET_OBJECT + SLM windows) + `PushBuf::compute_dispatch_with_launch()` (cache invalidate + PCAS/PCAS2)
6. **GPFIFO submit** → entry encoding + USERD GP_PUT + doorbell

The `sm` field on `NvVfioComputeDevice` tracks the SM version from BOOT0 probe, enabling generation-aware QMD version selection.

### 3. Socket permissions — RESOLVED

**What hotSpring asked**: Daemon creates sockets as `root:root 0600`. Non-root springs can't connect. Need configurable socket permissions.

**What toadStool shipped**:

- **JSON-RPC socket**: Already reads `TOADSTOOL_SOCKET_MODE` env var (default `0o600`). Set `TOADSTOOL_SOCKET_MODE=0660` for group access or `0666` for world-readable.
- **tarpc socket**: **Fixed** — was hardcoded `0o600`, now reads the same `TOADSTOOL_SOCKET_MODE` env var.
- **Directory permissions**: Parent directory created with `0o700` (JSON-RPC) — can be overridden by systemd `RuntimeDirectoryMode` if using a service file.

**For hotSpring**: Set `TOADSTOOL_SOCKET_MODE=0660` in your systemd unit or `.env` and ensure the calling user is in the same group. This replaces the `ExecStartPost` chmod workaround.

---

## coralReef items — noted, not our domain

hotSpring reported 2 shader compilation failures:
1. Subgroup operations (Discriminant(20)) — coralReef WGSL frontend
2. WGSL type error in `deformed_wavefunction_f64.wgsl` — coralReef lower_f64 pipeline

These are coralReef-owned. Noted in this handoff for cross-reference.

---

## Validation Points for hotSpring

### New IPC endpoints to test

```jsonc
// 1. Open VFIO device
{"jsonrpc": "2.0", "method": "device.vfio.open", "params": {"bdf": "0000:25:00.0"}, "id": 1}
// Returns: { "status": "ready"|"unavailable", "capabilities": {...} }

// 2. Full roundtrip
{"jsonrpc": "2.0", "method": "device.vfio.roundtrip", "params": {
  "bdf": "0000:25:00.0",
  "binary_b64": "<base64 shader binary>",
  "dispatch_dims": [64, 1, 1],
  "shader_info": {"gpr_count": 16, "workgroup": [64, 1, 1]},
  "buffers": [{"size": 256, "direction": "out"}]
}, "id": 2}
// Returns: { "job_id": "...", "status": "completed"|"failed", "output": {...} }

// 3. compute.dispatch.submit (existing, now QMD-based for VFIO)
// Same interface as before — now builds QMD internally
```

### Socket permissions test

```bash
# Set in systemd unit or environment
TOADSTOOL_SOCKET_MODE=0660

# Verify after daemon start
stat /run/toadstool/compute.sock
# Should show srw-rw---- (0660)
```

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo clippy --workspace --all-targets` | 0 warnings |
| `cargo test --workspace --lib` | all pass |
| `cargo deny check bans` | clean |
| JSON-RPC methods (direct) | 79 |
| New endpoints | `device.vfio.open`, `device.vfio.roundtrip` |
| New semantic aliases | `ember.vfio.open`, `ember.vfio.roundtrip` |

---

## Architecture Notes

### QMD dispatch flow

```
shader binary (from coralReef)
  → DMA upload (IOVA)
  → CBUF descriptor table (buffer IOVAs)
  → driver constants (grid dims)
  → QMD build (version-aware: v2.2/v2.3/v3.0/v5.0)
  → PushBuf (compute_init + compute_dispatch)
  → GPFIFO entry → USERD GP_PUT → doorbell
  → PBDMA reads GPFIFO → submits to GR engine
  → sync polls GP_GET
```

### IOVA layout

| Region | IOVA range | Purpose |
|--------|-----------|---------|
| Channel infra | 0x0000–0xFFFF | Instance block, page tables, runlist, fault buf |
| GPFIFO ring | 0x10000 | 512 entries × 8 bytes |
| USERD page | 0x11000 | GP_PUT/GP_GET |
| User buffers | 0x20000–0x1FFFFF | Shader, QMD, CBUFs, data buffers |

### Terminal goal (from hotSpring pattern)

Short term: nouveau warm-catch → toadStool VFIO dispatch.
Terminal: toadStool cold-boots GPUs from first principles (PMC_ENABLE, FECS_CPUCTL, PRAMIN, GPC_MASK) — no nouveau, no vendor driver. The warm-catch experiments are R&D for the register sequences that move into the diesel engine.
