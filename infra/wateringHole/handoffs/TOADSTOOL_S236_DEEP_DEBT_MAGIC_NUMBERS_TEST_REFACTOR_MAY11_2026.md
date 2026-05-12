# ToadStool S236 — Deep Debt: Magic Numbers + Match Safety + Test Refactor

**Date**: May 11, 2026
**Session**: S236
**Scope**: Magic number extraction, match exhaustiveness, dispatch test smart-refactor
**Commit**: `d2984551`

---

## Summary

Comprehensive deep debt audit across the entire workspace, followed by targeted fixes
in three areas: magic numbers, match safety, and large test file refactoring.

## Audit Findings (No Action Needed)

| Category | Finding |
|----------|---------|
| Large files (>800L) | Only 4 files exceed 800 LOC, all are test files. Production code is well-factored |
| Unsafe code | 46 blocks, all in hw containment zones (VFIO/mmap/volatile MMIO/plugin FFI), all SAFETY-documented |
| Production mocks | Zero — all `Mock*`/`Stub*` types are `#[cfg(test)]` gated. `StubRuntimeEngine` is a correct sentinel type param |
| TODOs in production | Zero |
| Clippy | Clean workspace-wide (`-D warnings`) |
| `ring` dependency | Lockfile artifact for optional/conditional resolution; not compiled. `cargo tree -i ring` returns empty |

## Changes

### 1. Magic numbers → named constants

Extracted 9 numeric literals into named constants:

| File | Constants |
|------|-----------|
| `crates/core/config/src/discovery_defaults.rs` | `DISCOVERY_TIMEOUT_SECS` (5), `DISCOVERY_REFRESH_SECS` (30), `DISCOVERY_CACHE_TTL_SECS` (300), `DISCOVERY_MAX_RETRIES` (3), `DISCOVERY_RETRY_DELAY_SECS` (1) |
| `crates/core/toadstool/src/discovery/mod.rs` | `DEFAULT_DISCOVERY_INTERVAL_SECS` (30), `DEFAULT_SERVICE_TIMEOUT_SECS` (300), `DEFAULT_MAX_SERVICES` (100) |
| `crates/auto_config/src/ecosystem/discoverer.rs` | `EcosystemDiscoverer::DEFAULT_TIMEOUT_SECS` (30) |

### 2. Match safety — unreachable!() elimination

`crates/core/nvpmu/src/dma.rs` `allocate_huge()`: Replaced two-pass match pattern
(guard match + value match with `unreachable!()` for `Standard`) with a single
exhaustive match where `Standard` returns early via `return self.allocate(size)`.
Eliminates potential panic if enum is extended.

### 3. Dispatch tests smart-refactor (1020 LOC → 4 submodules)

`crates/server/src/pure_jsonrpc/handler/dispatch/tests.rs` (1020 LOC monolith) refactored
into `tests/` directory:

| Submodule | Responsibility | Tests |
|-----------|---------------|-------|
| `mod.rs` | Shared helpers (`test_handler`, `submit_params`) | — |
| `core_dispatch.rs` | Capabilities, submit, status, result, forward, crypto path | ~25 |
| `shader.rs` | `shader.dispatch` binary formats, compile_result, readback, job tracking | ~18 |
| `envelope.rs` | JH-2 resource envelope enforcement (mem, cpu, timeout, integrated) | ~16 |
| `trio_contract.rs` | Wave 8 IPC contract (binary_b64, dispatch_dims, shader_info, timing) | ~9 |

72 dispatch tests pass. Clippy clean.

## Quality Gates

- `cargo clippy --workspace -- -D warnings`: **zero errors**
- `cargo test --workspace --lib`: **all pass**
- Zero production TODOs, zero production mocks, zero production unwraps

## Downstream Impact

None. All changes are internal (constant naming, match patterns, test organization).
No API, IPC, or behavioral changes.
