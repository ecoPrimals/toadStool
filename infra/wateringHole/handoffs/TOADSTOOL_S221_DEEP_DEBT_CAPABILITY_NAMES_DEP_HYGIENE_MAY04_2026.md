# ToadStool S221 — Deep Debt: Capability Names + Dep Hygiene + Stub Evolution + Coverage

**Date**: May 4, 2026
**Session**: S221

---

## Summary

Comprehensive deep-debt sweep targeting hardcoded primal names, dependency
hygiene, production stub dead code, unsafe audit reconciliation, and test
coverage expansion.

---

## Changes

### 1. Hardcoded Primal Names → Capability-Based (10 files)

All `barraCuda/coralReef` references in production error messages, deprecation
attributes, and doc comments evolved to capability-based language
(`gpu.dispatch.opencl capability provider via IPC`).

Files: `gpu/compiler.rs`, `gpu/engine/init.rs`, `gpu/types.rs`,
`gpu/unified_memory/manager.rs`, `gpu/unified_memory/types.rs`,
`distributed/universal/types/specialized.rs`, `core/toadstool/workload/types.rs`,
`auto_config/hardware/gpu.rs`, `cli/zero_config/types.rs`,
`cli/utils/error_formatting.rs`.

Self-references to `toadStool` remain correct (primal knows its own identity).

### 2. Dependency Hygiene: reqwest 0.12 → 0.13

`toadstool-runtime-edge` upgraded `reqwest` from 0.12 to 0.13. The 0.13 release
defaults to `aws-lc-rs` instead of `ring` as the rustls crypto provider. `ring`
is no longer in the resolved workspace dependency graph, aligning with the
`deny.toml` ring ban.

### 3. Production Stub Evolution: Migration Dead Code

`verify_migration_success` was always returning `Ok(true)`, making the failure
branch in `execute_live_migration` dead code. Evolved to return `Ok(false)` with
a `tracing::warn` about unimplemented verification. The failure path is now live
and honest.

### 4. Unsafe Audit Reconciliation: 49 → 46

Literal `unsafe {}` block count is **46** (not 49 as documented since S216).
All are kernel FFI / mmap / ioctl / volatile MMIO / plugin loading. All
SAFETY-documented. None replaceable with safe Rust without performance loss.
Updated across DEBT.md, README.md, CONTEXT.md, NEXT_STEPS.md.

### 5. Coverage Push: +20 Tests

- `cli/daemon/routes.rs`: 11 new unit tests (identity.get, health aliases,
  metrics, get/delete workload params, unknown method, invalid JSON-RPC version)
- `cli/zero_config/configuration.rs`: 9 new tests with `temp_env::async_with_vars`
  (biome CPU scaling, runtime docker/cuda, security/network/storage provider env gates)

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | **22,580 tests, 0 failures** |
| `cargo build --workspace` | Clean |
| `cargo clippy --workspace` | 0 new warnings |
| `cargo fmt --all --check` | 0 diffs |

---

## Files Modified (18)

- 10 GPU/distributed/core/cli files: primal name → capability language
- `crates/runtime/edge/Cargo.toml`: reqwest 0.12 → 0.13
- `crates/cli/src/universal/operations/migration.rs`: dead code fix
- `crates/cli/src/daemon/routes.rs`: +11 tests
- `crates/cli/src/zero_config/configuration.rs`: +9 tests
- Root docs: DEBT.md, README.md, CONTEXT.md, NEXT_STEPS.md
