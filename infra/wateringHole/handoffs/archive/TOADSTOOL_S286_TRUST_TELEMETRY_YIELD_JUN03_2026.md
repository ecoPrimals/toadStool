# toadStool S286 — Cross-Gate Trust Verification + Dispatch Telemetry + Yield-to-Owner Audit

**Date**: June 3, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Cascade**: primalSpring on eastGate → strandGate compute trio
**Status**: Software-only evolution complete. Hardware validation PAUSED.

## Mission (from primalSpring cascade)

While biomeGate hardware is offline for kernel recovery, advance software-only evolution:
1. Cross-gate trust validation (Dark Forest Invariant 3: Provenance)
2. Dispatch telemetry schema evolution (36-dim perceptron for ml.mlp_train)
3. Yield-to-owner audit (multi-gate mesh correctness)

## Completed

### 1. Cross-Gate Trust Verification

- `DispatchTrustLevel` enum: `Anonymous`, `LocalTransport`, `BtspVerified`, `MutuallyAuthenticated`
- `CallerContext` evolved: `gate_id: Option<String>`, `trust_level: DispatchTrustLevel`
- `ConnectionTrustHints` + `ConnectionTransport` for connection-layer trust extraction
- `extract_caller_context()` maps connection hints → CallerContext:
  - Unix socket → `LocalTransport` + local gate_id
  - BTSP-verified → `BtspVerified` + local gate_id
  - Anonymous fallback otherwise
- `dispatch.verify_trust` JSON-RPC method (Protected): returns trust assessment without dispatching
- `RemoteDispatcher::forward()` injects `_dispatch_trust.source_gate_id` provenance

**Key files**: `method_gate.rs`, `dispatch/trust.rs`, `handler/mod.rs`, `cross_gate/dispatcher.rs`

### 2. Dispatch Telemetry Schema

- `DispatchTelemetryRecord` — 36-field struct aligned with barraCuda ml.mlp_train:
  - dims 0-3: identity (gate_of_origin, trust_level, dispatch_mode, method)
  - dims 4-8: timing (queue_wait, dispatch, readback, total, timeout)
  - dims 9-14: workload shape (binary_size, workgroup xyz, buffer count/bytes)
  - dims 15-20: hardware (vendor, device_id, bdf, vram total/used, thermal)
  - dims 21-25: resource envelope (mem, cpu, timeout limits, tenant, priority)
  - dims 26-30: outcome (success, error_code, retried, forwarded, remote_gate)
  - dims 31-35: mesh context (local_gate_id, hop_count, yield_strategy, guest_load, timestamp)
- `dispatch.telemetry.schema` JSON-RPC method (Public): returns field list + dimension count

**Key files**: `dispatch/telemetry.rs`

### 3. Yield-to-Owner Audit

**Findings**: `check_guest_load` did not distinguish owner vs guest dispatches. `GateGpuInfo` had no ownership flag. Owner-gate dispatches could be incorrectly throttled.

**Fixes**:
- `GateGpuInfo.is_owner: bool` — gate.update can advertise ownership
- `GateOwnership` shared state — tracks local vs hardware owner, env override via `TOADSTOOL_HARDWARE_OWNER_GATE_ID`
- `ResourceRequest` — `caller_gate_id`, `hardware_owner_gate_id`, `caller_is_hardware_owner()`
- `check_guest_load` — owner bypass at function top
- `pre_dispatch_resource_check` — async, wires CallerContext + `_dispatch_trust.source_gate_id`

**Key files**: `cross_gate/ownership.rs`, `cross_gate/types.rs`, `resource_orchestrator.rs`, `dispatch/state.rs`, `dispatch/device.rs`

## Remaining Gaps (for future sessions)

1. `DispatchTelemetryRecord` is defined but not yet emitted from submit/fan_out paths
2. `compute.submit` local path still bypasses orchestrator/yield entirely
3. `device.gr.init` / `ember.prepare_dma` do not run `pre_dispatch_resource_check`
4. Guest load counts per-tenant, not total GPU load across tenants on shared hardware
5. BTSP trust level is wired at connection layer but BearDog JH-1 not yet shipping tokens

## Metrics

- 19 trust/ownership/gate tests pass
- 3 telemetry schema tests pass
- 9 yield tests pass (including owner bypass)
- Full workspace clippy -D warnings clean
- All workspace tests pass (exit 0)
