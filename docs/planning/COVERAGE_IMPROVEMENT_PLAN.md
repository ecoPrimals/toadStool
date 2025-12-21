# 📊 TEST COVERAGE IMPROVEMENT PLAN
**From 42% → 90% Coverage**

**Current State**: 42.24% (21,913 / 51,875 lines)  
**Target**: 90% (46,688 / 51,875 lines)  
**Gap**: **24,775 lines need tests**

---

## 🎯 PRIORITY AREAS (0% Coverage → High Impact)

### 1. CLI Operations (0% coverage, ~3,000 lines)

**Files with 0% coverage**:
- `cli/src/universal/manager_impl.rs` - 0/425 lines (15 functions)
- `cli/src/universal/operations/migration.rs` - 0/256 lines (46 functions)
- `cli/src/universal/operations/utilities.rs` - 0/432 lines (9 functions)
- `cli/src/universal/operations/benchmarking.rs` - 0/266 lines (17 functions)
- `cli/src/universal/operations/capabilities.rs` - 0/119 lines (7 functions)
- `cli/src/universal/operations/detection.rs` - 0/94 lines (10 functions)
- `cli/src/universal/operations/federation.rs` - 0/71 lines (12 functions)

**Impact**: These are core features users interact with  
**Effort**: Medium (integration tests needed)  
**Priority**: **CRITICAL**

### 2. Core ToadStool Library (0% coverage, ~1,500 lines)

**Files with 0% coverage**:
- `core/toadstool/src/ecosystem.rs` - 0/655 lines (51 functions)
- `core/toadstool/src/performance_hardening.rs` - 0/593 lines (67 functions)
- `core/toadstool/src/os_layer/manager.rs` - 0/133 lines (8 functions)
- `core/toadstool/src/lib.rs` - 0/45 lines (6 functions)

**Impact**: Foundation of the system  
**Effort**: High (complex domain logic)  
**Priority**: **CRITICAL**

### 3. Network Configuration (0% coverage, ~350 lines)

**Files with 0% coverage**:
- `cli/src/network_config/configurator/core.rs` - 0/269 lines
- `cli/src/network_config/configurator/traffic.rs` - 0/44 lines
- `cli/src/network_config/configurator/service_mesh.rs` - 0/32 lines
- `cli/src/network_config/configurator/security.rs` - 0/26 lines
- `cli/src/network_config/configurator/discovery.rs` - 0/22 lines

**Impact**: Networking is crucial for distributed systems  
**Effort**: Medium  
**Priority**: **HIGH**

### 4. Zero-Config System (0% coverage, ~1,100 lines)

**Files with 0% coverage**:
- `cli/src/zero_config/discovery.rs` - 0/483 lines (37 functions)
- `cli/src/zero_config/deployment.rs` - 0/160 lines (28 functions)
- `cli/src/zero_config/configuration.rs` - 0/140 lines (12 functions)
- `cli/src/zero_config/core.rs` - 0/109 lines (8 functions)
- `cli/src/zero_config/types.rs` - 0/49 lines (11 functions)

**Impact**: Zero-config is a key differentiator  
**Effort**: High (async, discovery protocols)  
**Priority**: **HIGH**

### 5. Templates & Generators (0% coverage, ~1,200 lines)

**Files with 0% coverage**:
- `cli/src/templates/specialized_templates.rs` - 0/677 lines (9 functions)
- `cli/src/templates/rendering.rs` - 0/203 lines (3 functions)
- `cli/src/templates/basic_templates.rs` - 0/175 lines (2 functions)
- `cli/src/templates/generator_impl.rs` - 0/140 lines (7 functions)

**Impact**: Template generation for users  
**Effort**: Low (mostly string generation, easy to test)  
**Priority**: **MEDIUM**

### 6. Security Modules (0% coverage, ~900 lines)

**Files with 0% coverage**:
- `security/policies/src/manager.rs` - 0/181 lines (29 functions)
- `security/policies/src/evaluator.rs` - 0/155 lines (8 functions)
- `security/policies/src/executor.rs` - 0/130 lines (4 functions)
- `security/sandbox/src/linux.rs` - 0/132 lines (21 functions)
- `security/sandbox/src/manager.rs` - 0/88 lines (18 functions)
- `security/sandbox/src/helpers.rs` - 0/84 lines (7 functions)

**Impact**: Security is non-negotiable  
**Effort**: High (requires privileges, isolation)  
**Priority**: **CRITICAL**

### 7. Server Components (0% coverage, ~700 lines)

**Files with 0% coverage**:
- `server/src/websocket.rs` - 0/244 lines (11 functions)
- `server/src/background.rs` - 0/229 lines (16 functions)
- `server/src/errors.rs` - 0/36 lines (2 functions)
- `server/src/mocks.rs` - 0/24 lines (8 functions)

**Impact**: Server mode for long-running biomes  
**Effort**: Medium (async, WebSocket testing)  
**Priority**: **HIGH**

### 8. Monitoring & Management (0% coverage, ~1,100 lines)

**Files with 0% coverage**:
- `management/monitoring/src/lib.rs` - 0/571 lines (46 functions)
- `management/analytics/src/lib.rs` - 0/101 lines (11 functions) [actually 18.23%]
- `management/monitoring/src/types.rs` - 0/63 lines (4 functions)

**Impact**: Observability for operations  
**Effort**: Medium  
**Priority**: **MEDIUM**

### 9. Integration Protocols (0% coverage, ~650 lines)

**Files with 0% coverage**:
- `integration/protocols/src/lib.rs` - 0/453 lines (38 functions)
- `integration/nestgate/src/client.rs` - 0/367 lines (35 functions)
- `integration/protocols/src/transport.rs` - 0/227 lines (33 functions)

**Impact**: Integration with external systems  
**Effort**: High (network, external dependencies)  
**Priority**: **MEDIUM**

---

## 🏆 HIGH PERFORMERS (Already Good)

**Keep these strong**:
- `client/src/lib.rs` - **94.90%** ✅
- `testing/src/properties.rs` - **96.34%** ✅
- `testing/src/integration/integration_impl.rs` - **88.10%** ✅
- `core/toadstool/src/error.rs` - **100%** ✅
- `core/common/src/error_codes.rs` - **93.26%** ✅
- `core/toadstool/src/os_layer/platform.rs` - **100%** ✅
- `core/toadstool/src/byob/config.rs` - **100%** ✅
- `server/src/handlers.rs` - **94.16%** ✅

---

## 📋 EXECUTION STRATEGY

### Phase 1: Quick Wins (Week 1) - Target: 55%

**Focus**: Template generation, simple utilities  
**Files**: 5-7 files with ~1,500 lines  
**Estimated Gain**: +13%

1. Template generators (string generation)
2. Type conversions and constructors
3. Configuration builders

**Tests to Write**:
```rust
#[test]
fn test_basic_template_generation() { ... }

#[test]
fn test_specialized_template_with_gpu() { ... }

#[test]
fn test_template_rendering_escaping() { ... }
```

### Phase 2: Core Functionality (Week 2) - Target: 70%

**Focus**: Core ToadStool, CLI operations  
**Files**: 10-12 files with ~3,500 lines  
**Estimated Gain**: +15%

1. `core/toadstool/src/ecosystem.rs`
2. `cli/src/universal/manager_impl.rs`
3. `cli/src/universal/operations/*.rs`

**Tests to Write**:
```rust
#[tokio::test]
async fn test_universal_compute_migration() { ... }

#[tokio::test]
async fn test_substrate_detection() { ... }

#[tokio::test]
async fn test_federation_peer_connection() { ... }
```

### Phase 3: Security & Networking (Week 3) - Target: 80%

**Focus**: Security, network config, zero-config  
**Files**: 15-18 files with ~2,500 lines  
**Estimated Gain**: +10%

1. Security policies and sandbox
2. Network configurators
3. Zero-config discovery

**Tests to Write**:
```rust
#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_sandbox_isolation() { ... }

#[tokio::test]
async fn test_mdns_discovery() { ... }

#[tokio::test]
async fn test_service_mesh_config() { ... }
```

### Phase 4: Integration & Polish (Week 4) - Target: 90%

**Focus**: Integration tests, edge cases, monitoring  
**Files**: Remaining files with ~1,500 lines  
**Estimated Gain**: +10%

1. Integration protocols
2. Monitoring and analytics
3. Server components
4. Edge cases and error paths

**Tests to Write**:
```rust
#[tokio::test]
async fn test_end_to_end_biome_lifecycle() { ... }

#[tokio::test]
async fn test_websocket_real_time_updates() { ... }

#[tokio::test]
async fn test_performance_monitoring_accuracy() { ... }
```

---

## 🛠️ TEST INFRASTRUCTURE IMPROVEMENTS

### 1. Test Utilities

**Create**:
- `tests/support/fixtures.rs` - Common test fixtures
- `tests/support/mocks.rs` - Mock external services
- `tests/support/assertions.rs` - Custom assertions

### 2. Integration Test Harness

**Enhance**:
- Docker Compose for integration tests
- In-memory service discovery for testing
- Fake external APIs (Songbird, BearDog, NestGate)

### 3. Coverage Automation

**Setup**:
```bash
# Add to CI/CD
cargo llvm-cov --lib --all-features --json --output-path coverage.json
cargo llvm-cov report --fail-under-lines 90
```

### 4. Property-Based Testing

**Expand**:
- Already have `properties.rs` at 96%
- Add property tests for:
  - Capability matching
  - Configuration validation
  - Resource limit enforcement

---

## 🚧 KNOWN CHALLENGES

### 1. **Security Tests Require Privileges**

**Problem**: Sandbox tests need Linux capabilities  
**Solution**: 
- Mock low-level OS calls
- Add `#[cfg(target_os = "linux")]` guards
- Document manual testing procedures

### 2. **Network Tests Need Isolation**

**Problem**: Port conflicts, external dependencies  
**Solution**:
- Use dynamic port allocation (port 0)
- Mock external services
- Use in-process network simulation

### 3. **Async Test Complexity**

**Problem**: Timeouts, race conditions  
**Solution**:
- Use `tokio::time::pause()` for deterministic tests
- Add generous timeouts for CI
- Use `tokio::test` with `#[serial]` for shared resources

### 4. **GPU/Hardware Tests**

**Problem**: CI runners lack GPUs  
**Solution**:
- Mock hardware detection
- Use feature flags: `#[cfg(feature = "gpu-tests")]`
- Document manual testing on real hardware

---

## 📊 METRICS & TRACKING

### Coverage Goals by Module

| Module | Current | Target | Gap | Priority |
|--------|---------|--------|-----|----------|
| cli | ~30% | 90% | +60% | CRITICAL |
| core | ~50% | 95% | +45% | CRITICAL |
| distributed | ~60% | 90% | +30% | HIGH |
| security | 0% | 85% | +85% | CRITICAL |
| server | ~35% | 85% | +50% | HIGH |
| testing | ~85% | 95% | +10% | LOW |
| runtime | ~55% | 85% | +30% | HIGH |
| integration | ~40% | 80% | +40% | MEDIUM |
| management | ~25% | 75% | +50% | MEDIUM |

### Weekly Targets

- **Week 1**: 42% → 55% (+13%) - Quick wins
- **Week 2**: 55% → 70% (+15%) - Core logic
- **Week 3**: 70% → 80% (+10%) - Security/networking
- **Week 4**: 80% → 90% (+10%) - Integration/polish

---

## ✅ SUCCESS CRITERIA

### Minimum Requirements for 90%

1. ✅ All critical paths tested
2. ✅ All public APIs have tests
3. ✅ Error paths covered
4. ✅ Edge cases documented and tested
5. ✅ Integration tests for major workflows
6. ✅ CI enforces coverage threshold

### Quality Gates

- **Unit Test**: Every public function has ≥1 test
- **Integration**: Every module interaction tested
- **Property**: Core algorithms have property tests
- **Chaos**: Distributed components have chaos tests
- **Fault**: Error injection tests for resilience

---

## 🎓 TEST WRITING GUIDELINES

### 1. Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Unit tests - fast, isolated
    mod unit {
        use super::*;
        
        #[test]
        fn test_basic_functionality() { ... }
    }
    
    // Integration tests - slower, external deps
    mod integration {
        use super::*;
        
        #[tokio::test]
        async fn test_end_to_end() { ... }
    }
}
```

### 2. Test Naming Convention

```rust
// Pattern: test_<function>_<scenario>_<expected_outcome>
#[test]
fn test_biome_start_with_invalid_manifest_returns_error() { ... }

#[test]
fn test_capability_discovery_finds_all_matching_services() { ... }
```

### 3. Arrange-Act-Assert Pattern

```rust
#[test]
fn test_example() {
    // Arrange - setup
    let input = create_test_input();
    
    // Act - execute
    let result = function_under_test(input);
    
    // Assert - verify
    assert_eq!(result.status, Expected::Value);
}
```

---

## 🚀 IMMEDIATE ACTIONS

### Today (1-2 hours):

1. ✅ Write tests for template generators (5-7 tests)
2. ✅ Add unit tests for capability taxonomy (3-4 tests)
3. ✅ Create integration test for federation (1 test)

### This Week:

1. Add tests for all `cli/src/universal/operations/*` files
2. Add tests for `core/toadstool/src/ecosystem.rs`
3. Set up coverage CI check

### This Month:

1. Achieve 70% coverage milestone
2. Add security and network tests
3. Comprehensive integration test suite

---

**Status**: Plan ready for execution  
**Estimated Time**: 4 weeks (80-100 hours)  
**Expected Outcome**: 42% → 90% coverage


