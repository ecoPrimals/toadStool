# primalSpring Glacial Debt Escalation — toadStool Response (S254)

**Date**: May 13, 2026
**Session**: S254
**From**: primalSpring Tier 2 convergence audit (413-method registry, interstadial exit)
**Action**: Phase D factory wired, NvVfioComputeDevice skeleton created

---

## Audit Cross-Reference

toadStool was listed under **Node atomic — thermal niche**:

| Audit Item | Status | Detail |
|---|---|---|
| Phase C complete (S245-S250) | **CONFIRMED** | 520 cylinder tests, `toadstool.validate` live |
| `toadstool.validate` wired to 7 springs | **CONFIRMED** | Tier 2 Science API pre-flight |
| `barracuda.precision.route` implemented | **N/A** | barraCuda responsibility |
| Titan V FECS warm dispatch blocker | **DOCUMENTED** | See "FECS Blocker" below |
| Phase D factory hook-up | **RESOLVED** (S254) | `LocalDeviceFactory` wired for AMD |
| VFIO PBDMA E2E | **PARTIAL** | Register/channel code exists; E2E gated by FECS |
| `SovereignDevice` through trio IPC | **RESOLVED** (S254) | `NvVfioComputeDevice` implements `ComputeDevice` trait |

---

## What Shipped (S254)

### 1. Phase D Factory Hook-Up

`LocalDeviceFactory` was implemented but **never wired** in production — `local_device_factory` stayed `None`, so `try_local_dispatch` always returned `None` (fall-through to coral_client IPC).

**Fixed**: `create_cylinder_device_factory()` now resolves BDF → sysfs DRM render node → driver detection → `ComputeDevice`:

- **AMD (`amdgpu`)**: Creates `AmdDevice::open_path(render_path)` — full DRM compute dispatch (GEM buffers, PM4 command streams, fence sync)
- **NVIDIA (`nouveau`)**: Logged as FECS-gated, returns `None` until FECS bridge resolved
- **Other**: Returns `None` with debug log

Factory is registered at `DispatchHandler` construction via `set_local_device_factory`. Capabilities report `local_dispatch: true` on Linux.

**Files**: `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs`, `crates/server/src/pure_jsonrpc/handler/mod.rs`

### 2. NvVfioComputeDevice

New `ComputeDevice` implementation for NVIDIA GPUs via VFIO:

- `NvVfioComputeDevice::new(bdf)` — cold init with `HardwareCapabilities::UNKNOWN`
- `NvVfioComputeDevice::with_sm(bdf, sm)` — init with known SM version → generation profile → real capabilities
- `probe_capabilities()` — reads BOOT0 via sysfs BAR0, derives SM → profile → caps
- `set_fecs_ready(bool)` — gates dispatch methods
- All `ComputeDevice` methods return `DriverError::Unsupported` until FECS is ready
- 5 unit tests covering cold dispatch, alloc, capabilities, and FECS flag

**File**: `crates/core/cylinder/src/nv/compute_device.rs`

---

## FECS Blocker (Remaining)

**Titan V warm dispatch fails at FECS compute context initialization.**

Root cause documented in `gsp_bridge.rs`: `StubGspBridge` cannot upload FECS firmware. Three production paths exist:

1. **Warm-handoff from nouveau/nvidia-470** — FECS state preserved across driver switch
2. **coralReef IPC** — real `GspBridge` impl via `compute.firmware.*` JSON-RPC
3. **Local absorption** — toadStool absorbs `vfio_compute` with `NvGspBridge`

This is the **critical path** for sovereign NV dispatch E2E. The PBDMA register infrastructure, channel orchestration, and FECS warm-detection code all exist in cylinder — only the firmware boot bridge is missing.

---

## Quality Gates

- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings
- `cargo test --workspace --lib`: all pass (5 new NvVfioComputeDevice tests), 0 failures
- `cargo deny check bans`: clean

---

## Downstream Impact

- **hotSpring**: Phase D now live for AMD GPUs. NVIDIA VFIO dispatch remains FECS-gated.
- **primalSpring**: `local_dispatch: true` in capabilities report. Factory hook-up resolved.
- **coralReef**: FECS bridge is the remaining dependency. `GspBridge` trait is the contract.
