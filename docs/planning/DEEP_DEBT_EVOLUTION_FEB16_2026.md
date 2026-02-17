# Deep Debt Evolution Session - February 16, 2026

**Session Focus**: Systematic evolution of technical debt toward modern idiomatic Rust patterns

---

## Completed Work

### P0: Clippy Error Fix
- **File**: `crates/barracuda/src/ops/kriging_f64.rs:364`
- **Issue**: Manual swap pattern flagged by clippy
- **Fix**: Replaced with `lu.swap(k * n + j, max_row * n + j)`

### P1: Capability-Based Discovery Evolution
Evolved hardcoded primal names to capability-based discovery.

**Files Modified**:
- `crates/distributed/src/beardog_integration/client.rs`
  - Added `new_async()` with capability-based discovery
  - Deprecated sync `new()` for backward compatibility
  
- `crates/distributed/src/crypto_integration/client.rs`
  - Now uses endpoint's actual socket path, not hardcoded name
  
- `crates/core/toadstool/src/ecosystem/communication.rs`
  - Added `extract_socket_path()` helper for capability-based path resolution
  - Updated `create_client_for_service()` to use endpoint metadata
  
- `crates/integration/beardog/src/discovery.rs`
  - Fallback now uses generic `crypto.sock` instead of hardcoded `beardog.sock`
  
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs`
  - Added `new_async()` for capability-based discovery
  
- `crates/core/toadstool/src/biomeos_integration/storage_backend.rs`
  - Added `new_async()` using `discover_storage_socket()`
  
- `crates/core/toadstool/src/biomeos_integration/agent_backend.rs`
  - Added `new_async()` using Custom capability for ML services

**Pattern**: All sync constructors deprecated with `#[deprecated]`, new async versions use capability discovery from `toadstool_common::primal_sockets::discover_*_socket()`.

**Manager Layer Updates**:
- `crates/core/toadstool/src/biomeos_integration/agents.rs`
  - Added `with_ml_service()` async constructor
  - Updated `discover_with_config()` to use capability-based discovery first
  - Deprecated `with_squirrel()` sync constructor
  
- `crates/core/toadstool/src/biomeos_integration/auth.rs`
  - Added `with_crypto_service()` async constructor
  - Updated `discover_with_config()` to use capability-based discovery first
  - Deprecated `with_beardog()` sync constructor
  
- `crates/core/toadstool/src/biomeos_integration/storage.rs`
  - Added `with_storage_service()` async constructor
  - Deprecated `with_nestgate()` sync constructor

### P1: Unsafe Code Evolution
Evolved `crates/runtime/gpu/src/unified_memory/backends/cpu.rs`:
- Created `AlignedBuffer` RAII wrapper for memory allocation
- Encapsulates unsafe code in single, audited location
- Uses `NonNull<u8>` for compile-time null safety
- Automatic cleanup via `Drop` implementation
- Reduced unsafe surface area significantly

### P1: Mock Isolation Verification
Verified all mocks are properly isolated to test code:
- `crates/server/src/mocks.rs` - All gated with `#[cfg(test)]`
- `crates/testing/` - Test-only crate
- Production code paths contain no mocks

### P2: Large File Analysis
Analyzed files exceeding 1000 lines:
- `batched_eigh_gpu.rs` (2054 lines): GPU kernel + comprehensive tests - cohesive unit
- `cg_gpu.rs` (2011 lines): Conjugate gradient solver - cohesive unit
- `env_config.rs` (1158 lines): 715 code + 443 tests - acceptable

**Decision**: Large files have good cohesion; excess is comprehensive tests. No arbitrary splitting - would hurt maintainability.

### P2: Panic/Unwrap Verification
Verified panic!/unwrap usage:
- All `panic!()` calls in test code (acceptable)
- All `.unwrap()` and `.expect()` calls in test code
- Production code uses proper `Result` propagation

### P2: External Dependencies Analysis
Identified dependency issues:
- `renderdoc-sys` pulled by wgpu 0.19 (transitive)
- wgpu 0.19 vs workspace 22 version mismatch in barracuda

**Action**: Documented TODO for wgpu upgrade (requires API compatibility work):
```toml
# TODO: Migrate to workspace wgpu v22 in future release (API compatibility work)
wgpu = { version = "0.19", default-features = false, features = ["wgsl"] }
```

---

## Test Infrastructure Status

### E2E Tests (10 files)
- `tests/e2e_tests.rs`
- `tests/e2e_comprehensive_tests.rs`
- `tests/e2e_concurrent_integration_suite.rs`
- `tests/e2e_composition_workflow.rs`
- `tests/e2e_primal_discovery_workflow.rs`
- `crates/cli/tests/e2e_workflow_week3.rs`
- `crates/cli/tests/e2e_biome_lifecycle_tests.rs`
- `crates/barracuda/tests/e2e_math_pipeline.rs`
- `crates/runtime/secure_enclave/tests/e2e_test.rs`
- `crates/testing/tests/e2e_tests.rs`

### Chaos Tests (9 files)
- `tests/chaos_engineering_scenarios.rs`
- `crates/testing/tests/chaos_engineering.rs`
- `crates/testing/tests/chaos_expansion_tests.rs`
- `crates/testing/tests/chaos_network_tests.rs`
- `crates/testing/tests/chaos_tower_tests.rs`
- `crates/distributed/tests/chaos_testing_suite.rs`
- `crates/cli/tests/chaos_executor_stress_tests.rs`
- `crates/cli/tests/chaos_network_scenarios_week4.rs`
- `crates/cli/tests/chaos_resource_scenarios_week4.rs`

### Health Check & Capabilities Query (Continued)

**Files Modified**:
- `crates/distributed/src/beardog_integration/client.rs`
  - `health_check()` now probes endpoints via `beardog.health` RPC
  - Updates `healthy` and `latency_ms` based on actual response
  - Added `query_capabilities_async()` for runtime capability discovery
  - Calls `beardog.capabilities` RPC to get actual service capabilities

**Before**: `health_check()` just returned discovered endpoints without probing.
**After**: Actual RPC-based health probing with latency measurement.

**Before**: `capabilities()` returned hardcoded defaults (trait lifetime constraint).
**After**: `query_capabilities_async()` queries service at runtime.

---

## Remaining Work

### P2 (Medium Priority)
1. **wgpu Upgrade to v22**: Requires API migration work
2. **Test Coverage Enforcement**: Set CI to fail at <90% (currently warning only)

### P3 (Enhancement)
1. Implement NPU driver backends
2. Implement display backend
3. Add GPU spin-orbit Rust wiring
4. Unify `approx` crate versions (0.4.0 vs 0.5.1 duplicate)
5. Unix socket health ping in `integration/protocols/client.rs` (Phase 3)

---

## Principles Applied

1. **Capability-Based Discovery**: Primals discover services by capability, not hardcoded name
2. **Self-Knowledge Only**: Each primal knows only itself; discovers others at runtime
3. **Safe Rust First**: Minimize unsafe; encapsulate when necessary with RAII
4. **No Production Mocks**: All mocks isolated to `#[cfg(test)]`
5. **Result Propagation**: No panic in production code
6. **Deep Cohesion**: Don't split files arbitrarily; maintain logical units
