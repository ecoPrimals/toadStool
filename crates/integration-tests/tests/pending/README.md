# Pending Integration Tests

These test files reference ToadStool APIs that are **planned but not yet
fully integrated**. They are quarantined here so they do not block CI.

## Remaining Files

| File | Blocking on | Notes |
|---|---|---|
| `e2e_composition_workflow.rs` | Composition engine E2E path | APIs exist (`CompositionEngine`, `CompositionRequest`, `Constraint`); needs E2E wiring |
| `e2e_primal_discovery_workflow.rs` | API rename | Uses old `EcosystemDiscovery` — needs update to `EcosystemCoordinator::with_config()` |
| `fhe_integration_example.rs` | GPU hardware | `barracuda::ops::fhe_ntt` exists; test is `#[ignore]`, needs GPU to run |

## Graduated (moved to `tests/`)

| Session | Files |
|---|---|
| S22 | `error_handling_tests`, `resource_requirements_tests`, `security_context_tests`, `config_management_tests`, `evolution_fault_tests`, `evolution_chaos_tests` |
| S23 | `runtime_execution_tests` |
| S24 | `error_paths_discovery_tests`, `fault_tests` (→ `chaos/`), `security_tests` (→ `security/`) |

## Removed (S31h — stale/broken)

| File | Reason |
|---|---|
| `ecosystem_tests.rs` | Stub pointing at non-existent `integration/ecosystem_integration.rs` |
| `e2e_tests.rs` | Broken `#[path]` refs to non-existent `e2e/` dir; real tests graduated to `runtime_execution_tests.rs` |
| `comprehensive_test_runner.rs` | Fake runner with hardcoded metrics — no real test execution |
| `e2e_comprehensive_tests.rs` | Standalone assertions with no ToadStool APIs |
| `e2e_concurrent_integration_suite.rs` | Local mock types only — no ToadStool APIs |

## How to resume a test

1. Implement the missing API in the upstream crate
2. Move the test file back to `tests/` (one level up)
3. Fix any remaining compilation errors
4. Verify `cargo test -p toadstool-integration-tests --test <name>` passes
