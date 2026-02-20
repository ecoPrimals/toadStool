# Pending Integration Tests

These test files reference ToadStool APIs that are **planned but not yet
implemented**. They are quarantined here so they do not block CI while the
corresponding APIs are being built.

## Status

### ✅ Unblocked in Session 22 — moved to `tests/`

| File | What was done |
|---|---|
| `error_handling_tests.rs` | `ToadStoolError::Runtime` + `NotFound` variants added to `toadstool-common` |
| `resource_requirements_tests.rs` | Test rewritten to use real nested API; `ResourceRequirements::validate()` added |
| `security_context_tests.rs` | Test rewritten to use real `SecurityContext` API; `has_permission()` added |
| `config_management_tests.rs` | Test rewritten to use real `ToadStoolConfig` + non-optional `NetworkConfig` |
| `evolution_fault_tests.rs` | Self-contained (std/tokio only); buggy assertions fixed |
| `evolution_chaos_tests.rs` | Self-contained (std/tokio only); health-drain overflow and zero-sum bugs fixed |

### ✅ Unblocked in Session 23 — moved to `tests/`

| File | What was done |
|---|---|
| `runtime_execution_tests.rs` | Rewritten to use actual `RuntimeOrchestrator::new(strategy)`, `WorkloadSpec`, `ExecutionRequest`, `ExecutionResponse` APIs. URL-based workload used to avoid filesystem dependency. 20 tests pass. |

### ✅ Unblocked in Session 24 — moved to `tests/`

| File | What was done |
|---|---|
| `error_paths_discovery_tests.rs` | Rewrote using `toadstool::self_identity::{Capability, DiscoveredService}` (not fictitious `primal_identity`); `SelfIdentity::new()` (sync); corrected `DiscoveredService` struct fields. 10 tests pass. |
| `fault_tests.rs` | Created `tests/chaos/fault_injection.rs` (10 tests) + `tests/chaos/resilience_tests.rs` (9 tests) using real `toadstool_testing::chaos` API. 19 tests pass. |
| `security_tests.rs` | Created `tests/security/penetration_tests.rs` (13 tests) using real `SecurityContext`, `Capability`, `IsolationLevel` API. 13 tests pass. |

### Still blocked

| File | Blocking API | Upstream crate |
|---|---|---|
| `e2e_*` (5 files) | `ecosystem::discovery`, `composition_engine`, + missing e2e/ sub-module files | `toadstool` |
| `fhe_integration_example.rs` | `barracuda::ops::fhe_ntt` (NTT-based homomorphic ops) | `barracuda` |
| `comprehensive_test_runner.rs` | Multiple future APIs | multiple |
| `ecosystem_tests.rs` | `integration/ecosystem_integration.rs` sub-module missing | local |

> **Note**: `fault_tests.rs` and `security_tests.rs` were removed from `pending/` in Session 24 —
> the stale router stubs were deleted after their sub-modules were created in `tests/chaos/` and
> `tests/security/`. The active `tests/fault_tests.rs` and `tests/security_tests.rs` are live.

## How to resume a test

1. Implement the missing API in the upstream crate
2. Move the test file back to `tests/` (one level up)
3. Fix any remaining compilation errors
4. Verify `cargo test -p toadstool-integration-tests --test <name>` passes

## See also

`DEBT.md` — D-S16-004 (resolved: crate structure) and the individual debt items
for each unimplemented API.
