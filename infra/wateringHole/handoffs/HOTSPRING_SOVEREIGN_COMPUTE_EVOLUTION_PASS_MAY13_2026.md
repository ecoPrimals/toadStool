# hotSpring Sovereign Compute Evolution Pass — toadStool Response

**Date**: May 13, 2026
**Session**: S251
**From**: hotSpring hardware validation (Titan V GV100, Tesla K80, RTX 5060)
**Action**: toadStool daemon parity gaps resolved (C1–C7)

---

## Audit Context

hotSpring validated the sovereign compute pipeline on three GPU generations.
Results:
- Warm VFIO open: **WORKS**
- WGSL→SASS compilation: **WORKS** (3 shaders including lattice QCD)
- Dispatch readback: **FAILS** at FECS compute context init
- Cylinder translation bug: **FIXED** (`device.*` → `ember.*` method routing)
- Warm API: added to `coral-gpu` (`from_vfio_warm*`)

Full daemon parity audit identified 8 gaps. This session resolves C1–C7.

---

## Gaps Resolved (C1–C7)

### C1: Buffer alloc/upload/readback in local dispatch
**File**: `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs`

`try_local_dispatch` previously passed `&[]` for buffers and never called
`readback()`. Now implements the full `ComputeDevice` lifecycle:
alloc → upload → dispatch → sync → readback → free.

- Buffer descriptors (`size`, `direction`, `domain`, `data`) parsed from JSON
- Input buffers (`in`/`inout`) uploaded before dispatch
- Output buffers (`out`/`inout`) read back after sync
- Results returned as `data_b64` (base64) in response
- `readback_ms` timing now reflects actual GPU readback

### C2: `shader.dispatch` Phase D integration
**File**: `crates/server/src/pure_jsonrpc/handler/dispatch/shader_dispatch.rs`

`shader.dispatch` now calls `acquire_device_handle()` + `try_local_dispatch()`
before falling through to coral_client IPC. Matches the `compute.dispatch.submit`
Phase D path — all dispatch handlers have consistent diesel integration.

### C3: `ember.reacquire` JSON-RPC handler
**Files**: `handler/mod.rs`, `core/mod.rs`, `core/identity.rs`, `core/wire_l3.rs`

Wired `ember.reacquire` as a JSON-RPC method. Delegates to
`GlowPlugClient::reacquire()` which was already implemented but not routed.
Accepts `{ "bdf": "..." }` params. Registered in:
- `DIRECT_JSONRPC_METHODS` (67 methods total)
- `capabilities.list` ember section
- Wire L3 cost map (medium cost, mutating, device scope)

### C4: `device.*` → `ember.*` semantic aliases
**File**: `crates/core/toadstool/src/semantic_methods/mappings_extended.rs`

Added semantic mappings for the diesel-mode translation surface:
- `device.list` → `ember_list`
- `device.status` → `ember_status`
- `device.reacquire` → `ember_reacquire`

Plus corresponding `dispatch_by_impl_name` routes in `handler/mod.rs`.
External callers (hotSpring `ToadStoolDispatchClient`) can now use either
`device.*` or `ember.*` namespace.

### C5: Dispatch capabilities phase metadata
**File**: `crates/server/src/pure_jsonrpc/handler/dispatch/capabilities.rs`

Updated `ember.phase` from `"B"` to `"D"` in `dispatch_capabilities` response.
Added `ember.local_dispatch` boolean indicating whether a local device factory
is configured for Phase D sovereign dispatch.

### C6: GspBridge boundary documentation
**File**: `crates/core/cylinder/src/nv/gsp_bridge.rs`

Documented `StubGspBridge` as an intentional sentinel (not a test mock), with
explicit reference to the hotSpring May 2026 hardware validation context.
Explains the three production paths for FECS boot:
1. coralReef provides real `GspBridge` impl via IPC
2. toadStool absorbs `vfio_compute` with local `NvGspBridge` impl
3. Warm-handoff from nouveau/nvidia-470 preserves FECS state

### C7: Test + phase assertion fix
**File**: `tests/trio_contract.rs`

Updated ember phase assertion from `"B"` to `"D"` and added
`local_dispatch` field presence check.

---

## Quality Gates

| Gate | Result |
|------|--------|
| `cargo clippy --workspace --lib -- -D warnings` | 0 warnings |
| `cargo test --workspace --lib` | 8,809 pass, 0 fail |
| `cargo deny check bans` | clean |
| JSON-RPC methods | 67 direct + 118 semantic (3 new `device.*` aliases) |

---

## FECS Dispatch Readback — Root Cause

The dispatch readback failure at FECS compute context init is caused by
`StubGspBridge` — the sentinel implementation that returns
`DriverError::Unsupported` for all firmware operations. Sovereign init
stages 1-3 (bar0_probe, pmc_enable, memory training) work normally, but
stage 4 (falcon_boot → FECS) requires one of:

1. **coralReef IPC bridge**: coralReef's `FECS/GPCCS cold silicon init`
   (Pass 12 remaining work) provides firmware via `compute.firmware.*`
2. **Warm handoff**: nouveau/nvidia-470 trains FECS, then
   `reset_method=none` preserves state across VFIO bind
3. **Local absorption**: `vfio_compute` module fully absorbed with
   `NvGspBridge` implementation

Path (2) is validated by hotSpring experiment 171+ and the
`VfioChannel::create_warm` / `open_no_busmaster` warm paths in cylinder.

---

## Files Changed

| File | Change |
|------|--------|
| `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` | Full buffer lifecycle in `try_local_dispatch` |
| `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` | Pass buffer_descs to local dispatch, real readback_ms |
| `crates/server/src/pure_jsonrpc/handler/dispatch/shader_dispatch.rs` | Phase D local path for shader.dispatch |
| `crates/server/src/pure_jsonrpc/handler/dispatch/capabilities.rs` | Phase D metadata |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | `ember.reacquire` + device.* impl_name routes |
| `crates/server/src/pure_jsonrpc/handler/core/mod.rs` | `ember.reacquire` in DIRECT_JSONRPC_METHODS |
| `crates/server/src/pure_jsonrpc/handler/core/identity.rs` | Ember methods list |
| `crates/server/src/pure_jsonrpc/handler/core/wire_l3.rs` | `ember.reacquire` cost entry |
| `crates/core/toadstool/src/semantic_methods/mappings_extended.rs` | `device.*` semantic aliases |
| `crates/core/cylinder/src/nv/gsp_bridge.rs` | StubGspBridge documentation |
| `crates/server/src/pure_jsonrpc/handler/dispatch/tests/trio_contract.rs` | Phase D assertion |
