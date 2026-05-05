# ToadStool S222 — primalSpring ironGate Audit Response

**Date**: May 5, 2026
**Session**: S222

---

## Summary

Responded to three operational gaps from the primalSpring ironGate provenance
pipeline validation audit, plus cross-cutting discovery hierarchy documentation.

---

## Changes

### 1. Gap 2: Sandbox `working_dir` Override (MEDIUM)

**Problem**: `IsolationLevel::Standard` forced `current_dir("/tmp")`, requiring
absolute paths in all workload TOMLs. No way to override.

**Solution**: `apply_security_context` now accepts an optional `working_dir`
from the workload spec. Resolution depends on isolation level:
- `None`: workload dir always honoured
- `Basic`/`Standard`: honoured if under `temp_dir()` or in `trusted_directories`
  (new field on `SecuritySpec`, wired to `filesystem_security.allowed_write_paths`)
- `Enhanced`/`Maximum`: ignored (always `temp_dir()`)

Also fixed: `convert_security_context` now parses the `SecuritySpec.isolation`
string to `IsolationLevel` (was hardcoded to `Standard`).

**Files**: `runtime/native/src/security.rs`, `runtime/native/src/engine.rs`,
`cli/src/executor/workload/conversion.rs`, `cli/src/executor/workload/spec.rs`,
integration test fixes in 2 test files.

### 2. Gap 8: Env Var Expansion in TOMLs (MEDIUM)

**Problem**: `${WETSPRING_DIR}` and `$HOME` passed as literal strings in
workload TOML files.

**Solution**: `load_workload_file` runs `expand_env_vars()` on raw content
before deserialization. Handles:
- `${VAR_NAME}` — braced, unambiguous
- `$VAR_NAME` — bare, terminated by non-alphanumeric
- `$$` — escape to literal `$`
- Undefined vars expand to empty string with debug log

**Files**: `cli/src/executor/workload/loading.rs`

### 3. `display.composite` + `transport.bridge` Specs (LOW)

**Problem**: No specs existed for these PG-42 follow-on capabilities.

**Solution**: Added future-work spec sections with JSON-RPC method tables,
design constraints, and prerequisites to:
- `specs/DISPLAY_BACKEND_SPEC.md` — `display.composite` multi-layer compositing
- `specs/HARDWARE_TRANSPORT_SPEC.md` — `transport.bridge` cross-transport bridging

### 4. Discovery Hierarchy Documentation (Cross-cutting)

Documented primalSpring's 5-tier discovery escalation hierarchy:
1. Songbird `ipc.resolve`
2. biomeOS Neural API `capability.discover`
3. UDS filesystem convention
4. Socket registry / manifests
5. TCP probing

toadStool implements tiers 1–4 via `resolve_capability_socket_fallback`.
Documentation added to `primal_sockets/paths.rs` module doc and `CONTEXT.md`.

### 5. Stale Reference Cleanup

- Fixed `barraCuda/coralReef` reference in `conversion.rs` GPU deprecation comment
  → `gpu.dispatch capability provider`

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | **22,821 tests, 0 failures** |
| `cargo build --workspace` | Clean |
| `cargo clippy --workspace` | 0 new warnings |
| `cargo fmt --all --check` | 0 diffs |

---

## Files Modified (12)

- `crates/runtime/native/src/security.rs`: working_dir resolution + 9 tests
- `crates/runtime/native/src/engine.rs`: wire working_dir to security context
- `crates/cli/src/executor/workload/spec.rs`: `trusted_directories` field
- `crates/cli/src/executor/workload/conversion.rs`: parse isolation + trusted dirs + 4 tests
- `crates/cli/src/executor/workload/loading.rs`: env var expansion + 7 tests
- `crates/cli/tests/executor_workload_comprehensive_tests.rs`: field fixes
- `crates/cli/tests/workload_executor_test.rs`: field fixes
- `crates/core/common/src/primal_sockets/paths.rs`: discovery hierarchy docs
- `specs/DISPLAY_BACKEND_SPEC.md`: `display.composite` spec
- `specs/HARDWARE_TRANSPORT_SPEC.md`: `transport.bridge` spec
- Root docs: DEBT.md, README.md, CONTEXT.md, NEXT_STEPS.md, DOCUMENTATION.md
