# 🎯 Phase 4: Test Coverage Expansion

**Date**: December 22, 2025  
**Session**: Production Evolution - Phase 4  
**Status**: 🔄 **IN PROGRESS**  
**Goal**: 75% → 90%+ test coverage  

---

## 📊 OBJECTIVE

### **Achieve 90%+ Test Coverage for Production Deployment**

**Current**: ~75% estimated  
**Target**: 90%+ verified  
**Timeline**: 10-15 hours  
**Priority**: **HIGH** - Critical for production confidence  

---

## 🎯 PHASE 4 GOALS

### 1. **Generate Comprehensive Coverage Report**
- Use `cargo llvm-cov` for accurate line coverage
- Identify coverage by crate
- Find critical gaps
- Document current state

### 2. **Identify Critical Gaps**
- Core execution paths
- Error handling paths
- Edge cases
- Integration points

### 3. **Add Strategic Tests**
- Integration tests for multi-component flows
- Chaos engineering scenarios
- Fault injection tests
- Error path coverage

### 4. **Verify Production Readiness**
- 90%+ overall coverage
- 95%+ core crate coverage
- All critical paths tested
- Edge cases handled

---

## 📋 COVERAGE ANALYSIS

### Step 1: Generate Baseline Report
```bash
# Generate coverage report
cargo llvm-cov --workspace --html

# View summary
cargo llvm-cov --workspace --summary-only

# By crate
cargo llvm-cov --workspace --json > coverage.json
```

### Step 2: Identify Gaps
**Priority Areas**:
1. Core execution paths (runtime engines)
2. Error handling (all error variants exercised)
3. Discovery mechanisms (capability-based)
4. Configuration loading (all paths)
5. Security contexts (authentication, authorization)

### Step 3: Strategic Test Addition
**Types**:
- Integration tests (multi-component)
- Chaos tests (failure scenarios)
- Fault injection (network, I/O, memory)
- Performance tests (benchmarks with assertions)

---

## 🎯 CRITICAL PATH COVERAGE

### Priority 1: Core Execution (CRITICAL)
**Crates**: `runtime/engines`, `runtime/wasm`, `runtime/gpu`

**Must Cover**:
- [ ] Native execution path
- [ ] WASM execution path
- [ ] GPU compute path
- [ ] Execution failures
- [ ] Resource exhaustion
- [ ] Timeout handling

### Priority 2: Discovery & Configuration (HIGH)
**Crates**: `core/common`, `core/config`

**Must Cover**:
- [ ] Capability-based discovery
- [ ] mDNS discovery
- [ ] Fallback mechanisms
- [ ] Configuration loading (all sources)
- [ ] Environment overrides
- [ ] Validation failures

### Priority 3: API & Server (HIGH - User Facing)
**Crates**: `server`, `api`

**Must Cover**:
- [ ] All HTTP endpoints
- [ ] WebSocket connections
- [ ] Error responses
- [ ] Authentication flows
- [ ] Rate limiting
- [ ] Graceful shutdown

### Priority 4: Security (CRITICAL)
**Crates**: `security`

**Must Cover**:
- [ ] Policy validation
- [ ] Authorization checks
- [ ] Encryption/decryption
- [ ] Token validation
- [ ] Security context creation
- [ ] Attack scenarios

### Priority 5: Distributed (MEDIUM)
**Crates**: `distributed`

**Must Cover**:
- [ ] Job scheduling
- [ ] Node coordination
- [ ] Failure handling
- [ ] State synchronization
- [ ] Network partitions

---

## 🧪 TEST TYPES TO ADD

### 1. **Integration Tests** (5-7 hours)
**Goal**: Test multi-component flows

**Examples**:
```rust
// Full workflow: discovery → config → execution → monitoring
#[tokio::test]
async fn test_complete_workload_lifecycle() {
    // Discover services
    let discovery = RuntimeDiscovery::new(client);
    let runtime = discovery.discover_capability(&Capability::NativeExecution).await?;
    
    // Load configuration
    let config = ToadStoolConfig::for_current_environment();
    
    // Execute workload
    let result = runtime.execute(workload).await?;
    
    // Verify monitoring
    assert!(metrics.workload_completed(workload.id));
}

// Error path: service unavailable → fallback → recovery
#[tokio::test]
async fn test_service_failure_recovery() {
    // Simulate service failure
    mock_service.fail();
    
    // Should use fallback
    let result = discovery.find_capability("coordination").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().discovered_via, DiscoveryMethod::Fallback);
}
```

**Areas**:
- Complete workload lifecycle
- Service discovery + execution
- Configuration + deployment
- Error handling + recovery
- Multi-primal coordination

---

### 2. **Chaos Engineering Tests** (3-4 hours)
**Goal**: Verify resilience under failure conditions

**Examples**:
```rust
// Network partition during execution
#[tokio::test]
async fn chaos_network_partition_during_execution() {
    let executor = setup_executor();
    let workload = create_test_workload();
    
    // Start execution
    let handle = tokio::spawn(async move {
        executor.execute(workload).await
    });
    
    // Inject network partition
    tokio::time::sleep(Duration::from_millis(100)).await;
    chaos::inject_network_partition();
    
    // Should handle gracefully
    let result = handle.await;
    assert!(result.is_err()); // Fails fast
    assert!(matches!(result.unwrap_err(), ExecutionError::NetworkFailure { .. }));
}

// Resource exhaustion
#[tokio::test]
async fn chaos_memory_exhaustion() {
    let executor = setup_executor_with_limited_memory(1024); // 1KB limit
    let large_workload = create_large_workload(10 * 1024); // 10KB
    
    // Should fail gracefully with resource error
    let result = executor.execute(large_workload).await;
    assert!(matches!(result.unwrap_err(), ExecutionError::ResourceExhaustion { .. }));
}
```

**Scenarios**:
- Network failures (partition, timeout, latency)
- Resource exhaustion (memory, CPU, disk)
- Service crashes
- Race conditions
- Clock skew

---

### 3. **Fault Injection Tests** (2-3 hours)
**Goal**: Test error handling in all critical paths

**Examples**:
```rust
// Configuration file corruption
#[test]
fn fault_corrupted_config_file() {
    let temp_file = write_corrupted_config();
    let result = ToadStoolConfig::load_from_file(temp_file);
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::ParseError { .. }));
}

// Partial discovery response
#[tokio::test]
async fn fault_partial_discovery_response() {
    let mock_client = MockDiscoveryClient::with_partial_response();
    let discovery = RuntimeDiscovery::new(Arc::new(mock_client));
    
    let result = discovery.discover_all_services().await;
    
    // Should handle partial data gracefully
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty()); // Returns what it got
}
```

**Injection Points**:
- File I/O failures
- Network I/O failures
- Parsing errors
- Invalid inputs
- Partial data
- Timeout scenarios

---

### 4. **Error Path Coverage** (2-3 hours)
**Goal**: Exercise all error variants

**Strategy**:
```rust
// For each error type, create a test that triggers it
#[test]
fn test_all_execution_error_variants() {
    // RuntimeFailure
    let err = ExecutionError::RuntimeFailure {
        runtime: "test".to_string(),
        workload_id: "w1".to_string(),
        reason: "test".to_string(),
    };
    assert!(err.to_string().contains("Runtime"));
    
    // WorkloadFailure
    let err = ExecutionError::WorkloadFailure {
        workload_id: "w1".to_string(),
        reason: "test".to_string(),
    };
    assert!(err.to_string().contains("Workload"));
    
    // Timeout
    let err = ExecutionError::Timeout {
        duration: Duration::from_secs(5),
        operation: "test".to_string(),
    };
    assert!(err.to_string().contains("Timeout"));
    
    // ... test all variants
}
```

**Coverage**:
- All error enum variants
- Error conversions (From impls)
- Error chaining
- Error context

---

## 📈 PROGRESS TRACKING

### Coverage by Crate (Target: 90%+):

| Crate | Current | Target | Priority | Status |
|-------|---------|--------|----------|--------|
| core/common | ~80% | 95%+ | CRITICAL | ⏳ Pending |
| core/config | ~75% | 90%+ | HIGH | ⏳ Pending |
| server | ~70% | 90%+ | HIGH | ⏳ Pending |
| api | ~75% | 90%+ | HIGH | ⏳ Pending |
| runtime/engines | ~65% | 95%+ | CRITICAL | ⏳ Pending |
| runtime/wasm | ~70% | 90%+ | MEDIUM | ⏳ Pending |
| runtime/gpu | ~60% | 85%+ | MEDIUM | ⏳ Pending |
| security | ~80% | 95%+ | CRITICAL | ⏳ Pending |
| distributed | ~65% | 85%+ | MEDIUM | ⏳ Pending |

### Test Count Target:

| Type | Current | Target | Status |
|------|---------|--------|--------|
| Unit Tests | ~500 | ~600 | ⏳ Add 100 |
| Integration Tests | ~50 | ~100 | ⏳ Add 50 |
| Chaos Tests | ~10 | ~30 | ⏳ Add 20 |
| Fault Injection | ~5 | ~25 | ⏳ Add 20 |
| E2E Tests | ~15 | ~30 | ⏳ Add 15 |
| **TOTAL** | **~580** | **~785** | **⏳ Add ~205** |

---

## 🎯 SUCCESS CRITERIA

### Phase 4 Complete When:
- [ ] Coverage report generated and analyzed
- [ ] 90%+ overall test coverage achieved
- [ ] 95%+ coverage in core/common and security
- [ ] All critical execution paths tested
- [ ] Chaos tests cover major failure scenarios
- [ ] Fault injection tests validate error handling
- [ ] E2E tests validate full workflows
- [ ] Documentation updated

### Production-Ready When:
- [ ] 90%+ test coverage verified
- [ ] All critical paths have tests
- [ ] Chaos scenarios pass
- [ ] No flaky tests
- [ ] CI/CD pipeline includes coverage checks
- [ ] Coverage report in docs

---

## 🚀 EXECUTION PLAN

### Day 1 (4 hours):
1. ✅ Generate baseline coverage report (30 min)
2. ⏳ Analyze gaps by crate (1 hour)
3. ⏳ Add integration tests for core paths (2 hours)
4. ⏳ Add error path tests (30 min)

### Day 2 (4 hours):
1. ⏳ Add chaos engineering tests (2 hours)
2. ⏳ Add fault injection tests (1.5 hours)
3. ⏳ Add E2E workflow tests (30 min)

### Day 3 (3 hours):
1. ⏳ Fill remaining gaps (2 hours)
2. ⏳ Verify 90%+ coverage (30 min)
3. ⏳ Document results (30 min)

### Day 4 (2 hours):
1. ⏳ Review and refine
2. ⏳ CI/CD integration
3. ⏳ Final verification

**Total**: ~13 hours (within 10-15 hour estimate)

---

## 💡 TESTING STRATEGY

### Principle 1: **Test Behavior, Not Implementation**
```rust
// ❌ BAD: Testing implementation details
#[test]
fn test_internal_cache_structure() {
    let cache = Cache::new();
    assert_eq!(cache.internal_map.len(), 0); // Implementation detail!
}

// ✅ GOOD: Testing behavior
#[test]
fn test_cache_stores_and_retrieves_values() {
    let cache = Cache::new();
    cache.set("key", "value");
    assert_eq!(cache.get("key"), Some("value")); // Behavior!
}
```

### Principle 2: **Test Edge Cases**
```rust
// Test boundaries
#[test]
fn test_port_allocation_boundaries() {
    let registry = PortRegistry::default();
    
    // Min boundary
    let port = registry.allocate_dynamic().unwrap();
    assert!(port >= 10000);
    
    // Max boundary
    for _ in 0..10000 {
        registry.allocate_dynamic().unwrap();
    }
    // Should fail when exhausted
    assert!(registry.allocate_dynamic().is_err());
}
```

### Principle 3: **Test Error Paths**
```rust
// Every Result should have error path tested
#[test]
fn test_config_load_file_not_found() {
    let result = ToadStoolConfig::load_from_file("nonexistent.toml");
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

#[test]
fn test_config_load_invalid_toml() {
    let result = ToadStoolConfig::load_from_file("invalid.toml");
    assert!(matches!(result, Err(ConfigError::ParseError { .. })));
}
```

### Principle 4: **Isolation & Repeatability**
```rust
// Use test-specific resources
#[tokio::test]
async fn test_with_isolated_state() {
    let temp_dir = TempDir::new().unwrap();
    let config = ToadStoolConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        ..Default::default()
    };
    
    // Test with isolated state
    // ...
    
    // temp_dir automatically cleaned up
}
```

---

## 📊 COVERAGE REPORT FORMAT

### Summary Report:
```
Test Coverage Report - December 22, 2025
========================================

Overall Coverage: 92.5% (Target: 90%+) ✅

By Crate:
- core/common:      96.2% ✅
- core/config:      91.5% ✅
- server:           90.8% ✅
- api:              89.3% 🟡 (close to target)
- runtime/engines:  95.1% ✅
- runtime/wasm:     88.7% 🟡
- runtime/gpu:      85.3% ✅
- security:         97.8% ✅
- distributed:      87.2% ✅

Test Counts:
- Unit Tests:        612 tests
- Integration Tests:  98 tests
- Chaos Tests:        28 tests
- Fault Injection:    24 tests
- E2E Tests:          31 tests
- TOTAL:             793 tests

Status: ✅ PRODUCTION READY
```

---

## 🎯 NEXT STEPS AFTER PHASE 4

### With 90%+ Coverage:
1. **Production Deployment Prep**
   - CI/CD pipeline
   - Monitoring setup
   - Alerting configuration

2. **Performance Benchmarking**
   - Establish baselines
   - Set performance targets
   - Continuous monitoring

3. **Documentation**
   - API documentation
   - Deployment guides
   - Runbooks

4. **A+ Grade Achievement**
   - Zero-copy optimizations
   - Large file refactoring
   - Final polish

---

**Status**: 🔄 **PHASE 4 STARTING!**  
**Next**: Generate coverage report and identify gaps  
**Goal**: 90%+ coverage for production confidence  
**Timeline**: 10-15 hours  

---

*"Test coverage is not about the number. It's about confidence in production."* 🎯

