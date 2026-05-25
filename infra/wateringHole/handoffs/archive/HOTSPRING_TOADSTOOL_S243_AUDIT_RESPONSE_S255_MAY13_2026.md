# hotSpring compute trio audit — toadStool response (S255, May 13, 2026)

Response to hotSpring's S243 compute trio audit. This audit was written against
S243 state; toadStool is now at S255. Most blockers have been resolved in
S245–S254 sessions. Below is an item-by-item resolution status.

---

## Audit item resolution matrix

| hotSpring blocker | Status | Resolved in | Notes |
|---|---|---|---|
| `toadstool-cylinder` crate | **RESOLVED** | S245 | 153 .rs files, 520 tests. DRM/MMIO/AMD/NVIDIA/VFIO modules absorbed from coral-driver |
| Coral-driver absorption | **RESOLVED** | S245–S250 | Hardware modules absorbed across Batches 1–7. `VfioResourceHandle` `Option<i32>` → `OwnedFd` (S253) |
| `ember.swap` + `sovereign.boot` RPC | **RESOLVED** | S252 + S255 | `device.swap` + `device.warm_catch` registered as direct JSON-RPC methods (S252). `ember.swap` and `sovereign.boot` added as semantic aliases → `device_swap` (S255) |
| USERD_TARGET encoding fix | **IN-TREE** | S245–S250 | Encoding implemented in `vfio/channel/page_tables.rs` (runlist DW0 USERD_TARGET) and `vfio/channel/registers.rs` (RAMFC PBDMA target). E2E dispatch blocked on FECS compute context init (GspBridge dependency) |

## Detail: what shipped since S243

### S245–S250: toadstool-cylinder + coral-driver absorption (Phase C)

- **153 .rs files, 520 tests** in `crates/core/cylinder/`
- DRM device discovery, BAR0 MMIO (read/write/batch), falcon PIO
- AMD PM4 compute queue, NVIDIA QMD/GPFIFO/pushbuf
- VFIO container/group/device lifecycle, IOMMU DMA mapping
- Process-level fork isolation (`fork_isolated_raw`) for BAR0 fault containment
- `GspBridge` trait boundary for GSP-dependent firmware ops
- `NvVfioComputeDevice` skeleton implementing `ComputeDevice` (S254)
- USERD_TARGET encoding in runlist + RAMFC (both PCCSR and PBDMA variants)

### S252: Diesel Engine Migration Batch 1–2

- `device.swap` + `device.warm_catch` JSON-RPC handlers
- `SysfsBar0Rw` read-write BAR0 mmap for register writes
- 6 MMIO/Falcon RPCs: `mmio.read32`, `mmio.write32`, `mmio.batch`, `mmio.pramin.read32`, `mmio.bar0.probe`, `mmio.falcon.status`
- `TOADSTOOL_RUN_DIR` socket layout replacing `CORALREEF_RUN_DIR`

### S253: Phase C completion

- `VfioResourceHandle` evolved to `OwnedFd` (RAII fd ownership)
- `SwapOrchestrator` real quiesce/persist/restore via sysfs PCI unbind/rebind
- `toadstool device` CLI: `swap`, `list`, `status`, `warm` subcommands
- `CORALREEF_*` env vars deprecated with `TOADSTOOL_*` primaries + `tracing::warn!`

### S254: Phase D factory wiring

- `LocalDeviceFactory` registered at `DispatchHandler` startup
- AMD DRM dispatch live via `AmdDevice`
- `NvVfioComputeDevice` implements `ComputeDevice` — FECS-gated

### S255: hotSpring audit response (this session)

- `ember.swap` → `device_swap` semantic alias (hotSpring callers no longer get `method_not_found`)
- `sovereign.boot` → `device_swap` semantic alias (hotSpring GlowplugClient compatibility)
- USERD_TARGET NEXT_STEPS entry corrected: encoding is in toadStool-cylinder, not blocked on coralReef
- Method count corrected: 77 direct JSON-RPC methods + semantic registry

## Remaining blocker: FECS compute context init

VFIO PBDMA E2E dispatch is blocked on **FECS compute context initialization**.
The firmware loads but compute context never becomes ready. Production paths:

1. **Warm-handoff from nouveau/nvidia-470** — catch an already-initialized context
2. **Real GspBridge** — either coralReef IPC or local GSP firmware handling
3. **PBDMA doorbell without FECS** — theoretical but unvalidated

`NvVfioComputeDevice.dispatch()` returns `DriverError::Unsupported` with
detailed FECS blocker message until `set_fecs_ready(true)` is called.

See: `crates/core/cylinder/src/nv/compute_device.rs`

## Per-device subprocess isolation

The audit requests per-device subprocess isolation for multi-GPU dispatch.
Current state:

- **In-process dispatch** via `Box<dyn ComputeDevice>` per GPU (Phase D factory)
- **Fork isolation** (`fork_isolated_raw`) for BAR0 MMIO fault containment — forks child, SIGKILL on timeout
- **Architectural slot** for full per-GPU long-lived subprocesses described in NEXT_STEPS but not yet implemented as OS-level process-per-device

For concurrent dispatch to Titan V + K80 + RTX 5060, the in-process path
provides thread-level isolation. OS-level subprocess isolation per GPU is
a Phase E item if hardware testing surfaces fault propagation between devices.

## Method surface for hotSpring callers

**77 direct JSON-RPC methods** + semantic aliases. Key methods for hotSpring:

| Method | Status | Notes |
|---|---|---|
| `device.swap` | LIVE | Also reachable as `ember.swap`, `sovereign.boot` (semantic aliases) |
| `device.warm_catch` | LIVE | Warm-catch pipeline |
| `ember.list` | LIVE | Device listing |
| `ember.status` | LIVE | Per-device status |
| `ember.reacquire` | LIVE | Re-acquire VFIO fd |
| `mmio.read32` | LIVE | BAR0 register read |
| `mmio.write32` | LIVE | BAR0 register write |
| `mmio.batch` | LIVE | Batched register ops |
| `mmio.pramin.read32` | LIVE | PRAMIN window read |
| `mmio.bar0.probe` | LIVE | BAR0 availability probe |
| `mmio.falcon.status` | LIVE | Falcon engine status |
| `compute.dispatch.submit` | LIVE | Dispatch pipeline (local → IPC fallback) |
| `compute.dispatch.capabilities` | LIVE | Phase D capabilities with `render_node`/`device_id` |
| `shader.dispatch` | LIVE | Shader dispatch via cylinder |
| `toadstool.validate` | LIVE | Pre-flight validation |
| `gpu.query_info` | LIVE | GPU introspection |
| `gpu.query_memory` | LIVE | Memory query |
| `gpu.query_telemetry` | LIVE | Telemetry |

## Quality metrics (S255)

| Metric | Value |
|---|---|
| Lib tests | 8,832 |
| Clippy warnings | 0 |
| `cargo deny check bans` | clean |
| JSON-RPC methods (direct) | 77 |
| Unsafe blocks | 46 (all SAFETY-documented) |
| Cylinder tests | 520 |
| Production files >800 lines | 0 |

## What hotSpring should rewire now

1. `GlowplugClient` calling `sovereign.boot` → now routed via semantic alias to `device.swap`
2. `GlowplugClient` calling `ember.swap` → now routed via semantic alias to `device.swap`
3. Socket paths: use `TOADSTOOL_RUN_DIR` (`/run/toadstool/`), not `CORALREEF_RUN_DIR`
4. Capability registry: update to **77 methods** (was 74 in S252 handoff)
5. `dispatch_capabilities()` response now includes Phase D factory status

## coralReef items (from hotSpring audit)

Not toadStool's domain. For reference:
- PTX SM120/Blackwell texture ops — coralReef compiler work
- `naga::Module` direct ingest — coralReef H2
- USERD_TARGET: if compiler-side encoding, stays in coralReef; driver-side encoding is **already in toadStool-cylinder**

## barraCuda items (from hotSpring audit)

Not toadStool's domain. Shared blocker (E2E dispatch test) depends on FECS resolution.

---

**Filed by**: toadStool S255 | **Quality gates**: all green | **Push**: SSH
