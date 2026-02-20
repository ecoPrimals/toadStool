# Pending Integration Tests

These test files reference ToadStool APIs that are **planned but not yet
implemented**. They are quarantined here so they do not block CI while the
corresponding APIs are being built.

## Status

| File | Blocking API | Upstream crate |
|---|---|---|
| `config_management_tests.rs` | `NetworkConfig.host/port/tls_enabled` | `toadstool-config` |
| `error_handling_tests.rs` | `ToadStoolError::Runtime`, `ToadStoolError::NotFound` | `toadstool-common` |
| `resource_requirements_tests.rs` | `ResourceUsage` struct | `toadstool` |
| `runtime_execution_tests.rs` | `RuntimeOrchestrator`, `WorkloadType` | `toadstool` |
| `security_context_tests.rs` | `SecurityContext`, `SecuritySettings` | `toadstool` |
| `security_tests.rs` | `SecurityContext` | `toadstool` |
| `e2e_*` | `ecosystem::discovery`, `composition_engine` | `toadstool` |
| `evolution_*` | `ToadStoolError::NotFound`, chaos infra | `toadstool` |
| `fault_tests.rs` | `ToadStoolError::Runtime` | `toadstool` |
| `fhe_integration_example.rs` | `barracuda::ops::fhe_ntt` | `barracuda` |
| `comprehensive_test_runner.rs` | Multiple future APIs | multiple |

## How to resume a test

1. Implement the missing API in the upstream crate
2. Move the test file back to `tests/` (one level up)
3. Fix any remaining compilation errors
4. Verify `cargo test -p toadstool-integration-tests --test <name>` passes

## See also

`DEBT.md` — D-S16-004 (resolved: crate structure) and the individual debt items
for each unimplemented API.
