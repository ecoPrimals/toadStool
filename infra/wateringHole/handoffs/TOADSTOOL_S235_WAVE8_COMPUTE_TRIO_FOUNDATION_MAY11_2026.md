# ToadStool S235 — Wave 8 Compute Trio Foundation

**Date**: May 11, 2026
**Session**: S235
**Audit source**: primalSpring Wave 8 Compute Trio Evolution sprint

---

## Summary

Foundation fixes for the Wave 8 Compute Trio audit. Three items shipped:

1. **BrainChip vendor ID alignment** — canonical `0x1E7C` everywhere
2. **Trio-standard IPC contract for `compute.dispatch.submit`** — backward-compatible
3. **Gate 2 `dispatch_capabilities` hardware info** — `gpu_count`, `architectures`, `vfio_status`

Multi-session coral diesel engine absorption roadmap documented (Phases A-D).

---

## Changes

### 1. BrainChip Vendor ID Fix

| File | Before | After |
|------|--------|-------|
| `crates/core/common/src/pci_discovery.rs` | `vendors::BRAINCHIP = 0x1e96` | `0x1E7C` |
| `crates/server/src/pure_jsonrpc/handler/hw_learn/helpers.rs` | `0x1e64 => "Brainchip"` | `0x1E7C => "BrainChip"` |

Canonical source: `crates/neuromorphic/akida-driver/src/lib.rs` (`0x1E7C`).
`crates/toadstool-core/src/hardware.rs` already correct (`"1e7c"`).

### 2. Trio-Standard IPC Contract

`crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` — backward-compatible evolution:

| Field | Legacy | Trio Standard | Behavior |
|-------|--------|---------------|----------|
| Binary input | `binary` (u8 array) | `binary_b64` (base64 string) | `binary_b64` preferred; `binary` fallback |
| Shader metadata | not accepted | `shader_info { gprs, shared_memory, barriers, workgroup, wave_size }` | Optional, threaded to dispatch metadata |
| Dispatch dimensions | `workgroup_size` (array) | `dispatch_dims` (array) | `dispatch_dims` preferred; `workgroup_size` fallback |
| Buffer format | `buffers` (mixed) | `buffers[{ binding, data_b64, size, usage }]` | `data_b64` auto-decoded to `data` |
| Response timing | not returned | `timing { dispatch_ms, readback_ms }` | Always included |

Helper functions extracted: `resolve_binary_param()`, `resolve_workgroup_size()`, `resolve_buffers()`.

### 3. Gate 2 Capabilities

`crates/server/src/pure_jsonrpc/handler/dispatch/capabilities.rs` — enhanced `dispatch_capabilities`:

- `gpu_count` — total discovered GPUs
- `architectures` — deduplicated compute architecture strings (e.g. `["sm75", "sm80"]`)
- `vfio_status.available` / `vfio_status.device_count` — VFIO readiness summary
- Per-GPU `architecture` field on both `vfio_gpus[]` and `drm_gpus[]` entries

Architecture detection via `gpu_architecture()` — maps (vendor, device_id) to compute
target strings (sm35..sm89 for NVIDIA, rdna2/rdna3/gcn for AMD, xe for Intel).

---

## Tests

- 9 new trio-contract tests (binary_b64, dispatch_dims, shader_info, data_b64 buffers, timing, error cases)
- Updated dispatch_capabilities test for new Gate 2 fields
- All existing 100+ dispatch tests pass unchanged

---

## Absorption Roadmap (Future Sessions)

| Phase | Scope | Key deliverable |
|-------|-------|-----------------|
| A: ember | `HeldDevice` → `ResourceHandle` | Production VFIO fd holding |
| B: glowplug | `sovereign_boot` → `SwapOrchestrator` | Device lifecycle ownership |
| C: cylinder + driver | Per-device subprocess + VFIO/DRM/GPFIFO | Universal dispatch subprocess |
| D: local dispatch | `dispatch_submit_with_context` → local execution | Gate 4 E2E path |

---

## Verification

```
cargo test -p toadstool-common --lib    # 1081 passed (vendor ID)
cargo test -p toadstool-server --lib    # 109 dispatch tests (trio contract + Gate 2)
cargo clippy -p toadstool-server -- -D warnings  # clean
```
