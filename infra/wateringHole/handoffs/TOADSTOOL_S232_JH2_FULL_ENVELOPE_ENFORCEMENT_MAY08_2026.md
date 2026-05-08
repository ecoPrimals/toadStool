# ToadStool S232 — JH-2 Full Envelope Enforcement (Phase 2)

**Date**: May 8, 2026
**Trigger**: primalSpring Phase 60 audit — JH-2 remaining: cpu/timeout enforcement + pipeline/shader bypass
**Depends on**: S231 (JH-2 Phase 1: mem + allowlist), S230 (error codes), S229 (JH-0)
**Status**: **JH-2 toadStool: COMPLETE**

## What the Audit Found

primalSpring identified two gaps in S231's JH-2 implementation:

1. `ResourceEnvelope` carried `cpu_cores` but `enforce_envelope()` only checked `mem_mb`
2. `pipeline_submit` called `dispatch_submit` with anonymous context (bypassing envelope), and `shader_dispatch` had no envelope check at all

## What Changed

### 1. Full Resource Enforcement (`dispatch/submit.rs`)

`enforce_envelope()` now checks **all three dimensions**:

| Dimension | Check | Error |
|-----------|-------|-------|
| `mem_mb` | binary size (rounded up to MB) ≤ limit | `RESOURCE_EXHAUSTED` (-32004) |
| `cpu_cores` | workgroup_total (x×y×z) ≤ cores × 1024 | `RESOURCE_EXHAUSTED` (-32004) |
| `max_timeout_ms` | requested timeout_ms ≤ limit | `RESOURCE_EXHAUSTED` (-32004) |

Signature changed from `(ctx, binary_size, timeout_ms)` → `(ctx, binary_size, workgroup_total, timeout_ms)`.

### 2. `max_timeout_ms` field (`method_gate.rs`)

`ResourceEnvelope` gains a new optional field:
```rust
pub max_timeout_ms: Option<u64>,
```

### 3. CallerContext Threaded Through All Dispatch Paths

| Entry Point | Before | After |
|-------------|--------|-------|
| `compute.dispatch.submit` (in `handle_method`) | `dispatch_submit()` (anonymous) | `dispatch_submit_with_context(&caller_ctx)` |
| `shader.dispatch` (in `handle_method`) | `shader_dispatch()` (anonymous) | `shader_dispatch_with_context(&caller_ctx)` |
| `compute.dispatch.pipeline.submit` (in `handle_method`) | `pipeline_submit()` (anonymous) | `pipeline_submit_with_context(&caller_ctx)` |
| Pipeline internal stages (`execute_stage_method`) | `dispatch_submit()` / `shader_dispatch()` (anonymous) | `dispatch_submit_with_context(ctx)` / `shader_dispatch_with_context(ctx)` |
| Semantic registry dispatch | `dispatch_submit()` etc. | `_with_context(ctx)` variants |

### 4. New Context-Aware Methods

- `DispatchHandler::shader_dispatch_with_context(params, ctx)` — calls `enforce_envelope` before GPU dispatch
- `DispatchHandler::pipeline_submit_with_context(params, ctx)` — forwards context to each stage
- `DispatchHandler::execute_stage_method(method, params, ctx)` — per-stage forwarding

The anonymous-context convenience wrappers (`dispatch_submit`, `shader_dispatch`, `pipeline_submit`) are retained for test use with `#[cfg_attr(not(test), expect(dead_code))]`.

## Test Coverage

- 7 new tests (774 total in toadstool-server):
  - `envelope_cpu_cores_allows_within_bounds` / `rejects_over_bounds`
  - `envelope_timeout_allows_within_bounds` / `rejects_over_bounds`
  - `envelope_all_dimensions_checked` (combined mem + cpu + timeout)
  - `dispatch_submit_rejects_timeout_over_envelope`
  - `shader_dispatch_enforces_envelope`
- All existing 56 dispatch tests pass
- Workspace `cargo clippy --workspace -- -D warnings`: clean

## Files Changed

| File | Change |
|------|--------|
| `method_gate.rs` | `ResourceEnvelope::max_timeout_ms` field |
| `dispatch/submit.rs` | Full 3-dimension `enforce_envelope()`, workgroup_total plumbing |
| `dispatch/shader_dispatch.rs` | `shader_dispatch_with_context()`, `enforce_envelope` call |
| `dispatch/pipeline.rs` | `pipeline_submit_with_context()`, `execute_stage_method(ctx)` |
| `dispatch/tests.rs` | 7 new envelope tests, updated helpers |
| `handler/mod.rs` | All dispatch paths use `_with_context(&caller_ctx)` |
| `auth.rs` | Updated test for `max_timeout_ms` |
| `README.md` | S232 entry |
| `NEXT_STEPS.md` | S232 entry |

## JH-2 Completion Status

All four enforcement dimensions are now active across all three dispatch entry points:

| | `compute.dispatch.submit` | `shader.dispatch` | `pipeline.*` stages |
|---|:---:|:---:|:---:|
| `mem_mb` | S231 | **S232** | **S232** |
| `cpu_cores` | **S232** | **S232** | **S232** |
| `max_timeout_ms` | **S232** | **S232** | **S232** |
| `method_allowlist` | S231 (gate) | S231 (gate) | S231 (gate) |

**toadStool JH-2: COMPLETE.** Remaining dependency: BearDog JH-1 for real token issuance → `CallerContext` population.
