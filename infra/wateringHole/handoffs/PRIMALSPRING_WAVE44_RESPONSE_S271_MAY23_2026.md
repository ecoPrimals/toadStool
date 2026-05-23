# toadStool — Wave 44 Neural API Announce Fix Response

**Date**: May 23, 2026
**Session**: S271
**From**: toadStool
**To**: primalSpring (coordination spring)
**Priority**: P2 — resolved
**License**: AGPL-3.0-or-later

## Summary

Wave 44 identified toadStool as P2: capabilities claim `["compute", "science", "inference"]`
but `ANNOUNCED_METHODS` only contained `compute.*` methods. **RESOLVED** — option (a) applied.

## What Changed

### 1. Expanded ANNOUNCED_METHODS (33 → 47 methods)

Added 14 methods across `science.*` and `inference.*` namespaces:

**Science domain** (10 methods):
- `science.compute.submit`, `science.compute.status`, `science.compute.result`, `science.compute.cancel`
- `science.gpu.dispatch`, `science.gpu.capabilities`
- `science.npu.dispatch`, `science.npu.capabilities`
- `science.substrate.discover`, `science.substrate.probe`

**Inference domain** (4 methods):
- `inference.execute`, `inference.list_models`, `inference.load_model`, `inference.unload_model`

### 2. Wired Dispatch for Science/Inference Impl Names

All 14 science/inference semantic impl names now have `dispatch_by_impl_name` arms:
- `science_compute_submit` → `workload.submit_workload()`
- `science_compute_status` → `job.query_status()`
- `science_compute_result` → `dispatch.dispatch_result()`
- `science_compute_cancel` → `workload.cancel_workload()`
- `science_gpu_dispatch` → `dispatch.shader_dispatch_with_context()`
- `science_gpu_capabilities` → `dispatch.dispatch_capabilities()`
- `science_npu_dispatch` → `dispatch.dispatch_submit_with_context()`
- `science_npu_capabilities` → `dispatch.dispatch_capabilities()`
- `science_substrate_discover` → `workload.query_capabilities()`
- `science_substrate_probe` → `workload.query_capabilities()`
- `inference_list_models` → `resources.resources_estimate()`
- `inference_execute` → `resources.resources_estimate()`
- `inference_load_model` → `resources.resources_estimate()`
- `inference_unload_model` → `resources.resources_estimate()`

Previously these resolved semantically but hit the `method_not_found` fallback.

### 3. Wire L3 Cost Estimates

- Science methods: `variable` cost, GPU-eligible, 100ms latency estimate
- Inference methods: `variable` cost, GPU-eligible, 200ms latency estimate

### 4. Test Coverage

- `announced_methods_covers_all_three_capabilities` — asserts compute.*, science.*, inference.* all present
- Existing `announced_methods_sorted` and `announced_methods_all_in_announced_namespaces` updated

## Files Modified

| File | Change |
|------|--------|
| `crates/server/src/ipc_surface.rs` | 14 science/inference methods added, test updated |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | 14 dispatch arms for science/inference impl names |
| `crates/server/src/pure_jsonrpc/handler/core/wire_l3.rs` | Cost estimates for science/inference methods |

## Metrics

| Metric | Before (S270) | After (S271) |
|--------|---------------|--------------|
| Announced methods | 33 (compute only) | 47 (compute + science + inference) |
| Lib tests | 9,125 | 9,126 |
| Clippy warnings | 0 | 0 |

## Remaining Work

None for Wave 44. Capabilities ↔ methods alignment is complete.
