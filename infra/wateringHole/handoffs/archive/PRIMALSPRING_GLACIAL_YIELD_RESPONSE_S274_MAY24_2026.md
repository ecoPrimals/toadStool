# toadStool S274 — Glacial Horizon: Yield-to-Owner Dispatch

**Date**: May 24, 2026
**Session**: S274
**From**: toadStool team
**To**: primalSpring (downstream audit)
**Audit**: primalSpring glacial horizon — `max_guest_load` yield semantics

---

## Summary

Evolved `max_guest_load` from types-only (S269) to enforced. The
`ResourceOrchestrator` now branches on `YieldStrategy` when GPU-bound
workloads exceed the configured threshold, enabling yield-to-owner
semantics for shared-hardware covalent deployments (flockGate).

## What Was Shipped (S269 — types only)

- `GuestLoadPolicy` struct with `max_concurrent_gpu` and `yield_strategy`
- `YieldStrategy` enum: `Queue`, `Reject`, `DeferUntilPowerCycle`
- `TenantQuota.max_guest_load: Option<GuestLoadPolicy>` field
- Serde (de)serialize support
- Documentation and design intent

## What S274 Adds

### Enforcement in `check_quota()`

`ResourceOrchestrator::check_quota()` now calls `check_guest_load()` after
standard quota checks. When `max_guest_load` is `Some(policy)`:

1. Count GPU-bound workloads from `TenantUsage.device_allocations`
2. Compare against `policy.max_concurrent_gpu`
3. If at/above threshold, apply yield strategy:
   - **`Queue`** → `GuestLoadExceeded` error with "caller should retry"
   - **`Reject`** → `GuestLoadExceeded` error (immediate rejection)
   - **`DeferUntilPowerCycle`** → `GuestLoadExceeded` error with
     "caller should retry after host power-cycle window"

### Error Type

New `OrchestrationError::GuestLoadExceeded(String)` variant — distinct
from `QuotaExceeded` so callers can distinguish yield semantics from
hard quota violations.

### Public API

`GuestLoadPolicy` and `YieldStrategy` now re-exported from crate root
(`toadstool_runtime_orchestration::{GuestLoadPolicy, YieldStrategy}`).

### Tests (10 new)

| Test | Validates |
|------|-----------|
| `guest_load_reject_strategy` | Reject yields `GuestLoadExceeded` |
| `guest_load_queue_strategy` | Queue yields `GuestLoadExceeded` with retry message |
| `guest_load_defer_power_cycle_strategy` | DeferUntilPowerCycle yields correct error |
| `guest_load_under_threshold_passes` | Below threshold: allocation succeeds |
| `guest_load_none_means_unlimited` | `None` policy: no enforcement |
| `guest_load_release_allows_reallocation` | Release → re-allocate succeeds |
| `guest_load_default_strategy_is_queue` | Default `YieldStrategy` is `Queue` |
| `guest_load_policy_serde_roundtrip` | JSON serialize/deserialize roundtrip |
| `yield_strategy_serde_names` | Wire names: `queue`, `reject`, `defer_until_power_cycle` |

## Server Dispatch Wiring (S274 continued)

`ResourceOrchestrator` is now wired into the server dispatch path for
local and LAN deployments (flockGate not required for local enforcement):

- `DispatchHandler` gains `resource_orchestrator: Option<Arc<ResourceOrchestrator>>`
- `pre_dispatch_resource_check(bdf)` called before `acquire_device_handle` in
  `device_vfio_open` and `device_vfio_roundtrip`
- `TOADSTOOL_DEPLOYMENT_MODEL` env var: `multi` → `LocalMulti`, `rental` →
  `CloudRental`, else `LocalDirect` (default, zero overhead — no orchestrator)
- `LocalDirect` = `None` orchestrator, all pre-dispatch checks are no-ops
- `GuestLoadExceeded` → JSON-RPC error `-32003` (`CAPABILITY_NOT_AVAILABLE`)
- `QuotaExceeded` → JSON-RPC error `-32004` (`RESOURCE_EXHAUSTED`)
- GPU devices discovered via `toadstool_sysmon::discover_gpus()` at startup
- Caller identity is `"anonymous"` until BearDog JH-1 ships `CallerContext`

### Dispatch integration tests (9 new)

| Test | Validates |
|------|-----------|
| `no_orchestrator_pre_dispatch_is_noop` | `LocalDirect` no-op path |
| `orchestrator_allows_dispatch_within_quota` | Allocation succeeds under quota |
| `guest_load_reject_returns_capability_not_available` | `-32003` on reject |
| `guest_load_queue_returns_capability_not_available` | `-32003` on queue |
| `quota_exceeded_returns_resource_exhausted` | `-32004` on quota exceeded |
| `local_direct_handler_has_no_orchestrator` | Default has `None` orchestrator |
| `multi_handler_reports_local_multi_model` | Model accessor |
| `guest_load_under_threshold_allows_dispatch` | Below threshold passes |
| `guest_load_defer_power_cycle_returns_error` | `-32003` on defer |

## What Remains (deferred)

| Item | Blocked on |
|------|------------|
| Power-cycle event hook → `DeferUntilPowerCycle` | Host suspend/resume event source |
| Cross-gate load reporting via `gate.queue_depth` | flockGate primal coordination |
| Authenticated tenant identity in `CallerContext` | BearDog JH-1 (ionic tokens) |

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,149+ |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 88 |
| Clippy warnings | 0 |
| Orchestration crate tests | 86 (was 67) |
| Dispatch orchestrator tests | 9 (new) |

---

Ready for downstream primalSpring audit.
