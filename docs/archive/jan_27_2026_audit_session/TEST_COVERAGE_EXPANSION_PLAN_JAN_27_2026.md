# 🧪 Test Coverage Expansion Plan - January 27, 2026

**ToadStool v4.19.0-dev**

**Current Coverage**: ~75-76%  
**Target Coverage**: 90%  
**Gap**: ~14-15%  
**Timeline**: 6-8 weeks

---

## 🎯 **OBJECTIVE**

Expand test coverage from 75% to 90% using **llvm-cov**, focusing on:

1. **Runtime engines** (Native, GPU, Adaptive, WASM)
2. **Integration scenarios** (Multi-primal, Distributed, Security)
3. **Chaos & fault injection** (Network failures, Resource exhaustion, Recovery)

**All tests must follow Deep Debt principles:**
- ✅ Real implementations (no mocks in production code)
- ✅ E2E workflows (not just unit coverage)
- ✅ Error scenarios (fault injection)
- ✅ Capability-based (runtime discovery)

---

## 📋 **THREE-PHASE ROADMAP**

### **Phase 1: Runtime Engine Tests** (2-3 weeks) → +5%

**Priority**: 🔴 **HIGH** - Core functionality

**Target Modules**:
```
crates/runtime/native/        ⚠️ Partial coverage
crates/runtime/gpu/           ⚠️ Partial coverage  
crates/runtime/adaptive/      ⚠️ Partial coverage
crates/runtime/wasm/          ⚠️ Partial coverage
```

**Test Scenarios to Add**:

#### 1.1 Native Runtime E2E
```rust
// tests/e2e/native_runtime_workflows.rs

#[tokio::test]
async fn test_native_binary_execution_e2e() {
    // Setup: Create test binary
    let binary = create_test_binary().await;
    
    // Execute: Run via Native runtime
    let runtime = NativeRuntime::new().await?;
    let result = runtime.execute(binary).await?;
    
    // Verify: Check output, exit code, resource usage
    assert!(result.exit_code == 0);
    assert!(result.stdout.contains("SUCCESS"));
}

#[tokio::test]
async fn test_native_resource_limits_enforcement() {
    // Test memory/CPU limits actually enforced
}

#[tokio::test]
async fn test_native_error_handling() {
    // Test binary crashes, timeouts, OOM scenarios
}
```

#### 1.2 GPU Runtime Integration
```rust
// tests/integration/gpu_runtime_selection.rs

#[tokio::test]
async fn test_gpu_backend_selection() {
    // Test Vulkan → OpenCL → WebGPU fallback chain
    let runtime = GpuRuntime::new().await?;
    let backend = runtime.selected_backend();
    assert!(backend.is_available());
}

#[tokio::test]
async fn test_gpu_unified_memory_e2e() {
    // Test CPU ↔ GPU data transfer workflows
}

#[tokio::test]
async fn test_gpu_error_recovery() {
    // Test GPU OOM, device loss, driver errors
}
```

#### 1.3 Adaptive Runtime
```rust
// tests/integration/adaptive_runtime_selection.rs

#[tokio::test]
async fn test_adaptive_selects_best_runtime() {
    // Given: Workload with GPU requirements
    // When: Adaptive runtime selects backend
    // Then: GPU runtime chosen if available, else fallback
}

#[tokio::test]
async fn test_adaptive_runtime_migration() {
    // Test workload migration between runtimes
}
```

#### 1.4 WASM Runtime
```rust
// tests/integration/wasm_runtime_errors.rs

#[tokio::test]
async fn test_wasm_trap_handling() {
    // Test WASM traps, panics, memory violations
}

#[tokio::test]
async fn test_wasm_component_model() {
    // Test component model linking, exports
}
```

**Expected Gain**: +5% coverage

---

### **Phase 2: Integration Tests** (2-3 weeks) → +10%

**Priority**: 🟡 **MEDIUM-HIGH** - System integration

**Target Modules**:
```
crates/distributed/           ⚠️ Partial coverage
crates/security/              ⚠️ Partial coverage
crates/cli/                   ⚠️ Partial coverage (integration paths)
crates/server/                ⚠️ Partial coverage (multi-client)
```

**Test Scenarios to Add**:

#### 2.1 Multi-Primal E2E
```rust
// tests/e2e/multi_primal_workflows.rs

#[tokio::test]
async fn test_beardog_integration_e2e() {
    // Given: BearDog running (or mocked at IPC level)
    // When: ToadStool requests crypto operation
    // Then: JSON-RPC over Unix socket succeeds
    
    let client = create_unix_socket_client("/tmp/beardog.sock").await?;
    let response = client.call_method("crypto.encrypt", params).await?;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_songbird_discovery_e2e() {
    // Test discovery via Songbird
}

#[tokio::test]
async fn test_nestgate_storage_e2e() {
    // Test artifact storage via NestGate
}
```

#### 2.2 Distributed Coordination
```rust
// tests/integration/distributed_coordination.rs

#[tokio::test]
async fn test_multi_node_workload_distribution() {
    // Test workload splitting across multiple ToadStool nodes
}

#[tokio::test]
async fn test_distributed_resource_pooling() {
    // Test resource aggregation across cluster
}

#[tokio::test]
async fn test_distributed_failure_handling() {
    // Test node failures, network partitions
}
```

#### 2.3 Security Integration
```rust
// tests/integration/security_workflows.rs

#[tokio::test]
async fn test_secure_enclave_execution() {
    // Test encrypted workload execution
}

#[tokio::test]
async fn test_security_policy_enforcement() {
    // Test policy violations, rejections
}

#[tokio::test]
async fn test_cryptographic_operations() {
    // Test signing, verification, encryption
}
```

#### 2.4 CLI Integration
```rust
// tests/integration/cli_workflows.rs

#[tokio::test]
async fn test_cli_run_command_e2e() {
    // Test: toadstool run workload.yaml
}

#[tokio::test]
async fn test_cli_daemon_mode() {
    // Test: toadstool daemon + client interactions
}

#[tokio::test]
async fn test_cli_status_monitoring() {
    // Test: toadstool status shows accurate state
}
```

**Expected Gain**: +10% coverage

---

### **Phase 3: Chaos & Fault Injection** (1-2 weeks) → +5%

**Priority**: 🟢 **MEDIUM** - Reliability & resilience

**Target**: Error paths, recovery scenarios, chaos testing

**Test Scenarios to Add**:

#### 3.1 Network Chaos
```rust
// tests/chaos/network_chaos.rs

#[tokio::test]
async fn test_unix_socket_disconnection() {
    // Given: Active IPC connection
    // When: Socket forcibly closed
    // Then: Graceful error, automatic reconnection
}

#[tokio::test]
async fn test_network_partition_recovery() {
    // Test discovery after network partition heals
}

#[tokio::test]
async fn test_slow_network_timeouts() {
    // Test timeout handling, backoff strategies
}
```

#### 3.2 Resource Exhaustion
```rust
// tests/chaos/resource_exhaustion.rs

#[tokio::test]
async fn test_memory_pressure_handling() {
    // Test workload rejection under memory pressure
}

#[tokio::test]
async fn test_disk_full_scenarios() {
    // Test artifact storage failures
}

#[tokio::test]
async fn test_cpu_throttling() {
    // Test behavior under CPU contention
}
```

#### 3.3 Fault Injection
```rust
// tests/chaos/fault_injection.rs

#[tokio::test]
async fn test_runtime_crash_recovery() {
    // Inject runtime crash, verify recovery
}

#[tokio::test]
async fn test_malformed_rpc_requests() {
    // Test malformed JSON-RPC handling
}

#[tokio::test]
async fn test_timeout_scenarios() {
    // Test various timeout conditions
}
```

#### 3.4 Recovery Scenarios
```rust
// tests/chaos/recovery_tests.rs

#[tokio::test]
async fn test_workload_resumption_after_failure() {
    // Test checkpointing and resume
}

#[tokio::test]
async fn test_state_recovery_after_crash() {
    // Test state persistence and recovery
}

#[tokio::test]
async fn test_automatic_failover() {
    // Test failover to backup resources
}
```

**Expected Gain**: +5% coverage

---

## 🛠️ **IMPLEMENTATION APPROACH**

### Week-by-Week Breakdown

#### **Week 1-2: Phase 1 Start**
```
Sprint Goal: +2-3% coverage (Runtime E2E)

Day 1-2:  Native runtime E2E tests
Day 3-4:  GPU runtime integration tests
Day 5-6:  Adaptive runtime tests
Day 7-8:  WASM runtime error paths
Day 9-10: Measure coverage, identify gaps
```

#### **Week 3-4: Phase 1 Complete + Phase 2 Start**
```
Sprint Goal: +5% total coverage

Day 1-2:  Complete Phase 1 gaps
Day 3-4:  Multi-primal E2E (BearDog, Songbird)
Day 5-6:  Distributed coordination tests
Day 7-8:  Security integration tests
Day 9-10: CLI integration tests
```

#### **Week 5-6: Phase 2 Complete + Phase 3 Start**
```
Sprint Goal: +12% total coverage

Day 1-2:  Complete Phase 2 gaps
Day 3-4:  Network chaos tests
Day 5-6:  Resource exhaustion tests
Day 7-8:  Fault injection tests
Day 9-10: Recovery scenario tests
```

#### **Week 7-8: Phase 3 Complete + Polish**
```
Sprint Goal: 90% total coverage achieved

Day 1-3:  Complete Phase 3 gaps
Day 4-5:  Fill remaining coverage holes
Day 6-7:  Documentation updates
Day 8-10: Final coverage report, celebration! 🎉
```

---

## 📊 **COVERAGE MEASUREMENT**

### Using llvm-cov

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Run tests with coverage
cargo llvm-cov --workspace --html

# Open coverage report
open target/llvm-cov/html/index.html

# Generate summary
cargo llvm-cov --workspace --summary-only

# Expected output:
# Filename                      Regions    Missed Regions     Cover
# ------------------------------------------------------------------------
# crates/runtime/native/        120        12                 90.00%
# crates/runtime/gpu/           200        20                 90.00%
# ...
# ------------------------------------------------------------------------
# TOTAL                         10,000     1,000              90.00%
```

### Coverage Gates

```yaml
# .github/workflows/ci.yml
- name: Check coverage
  run: |
    cargo llvm-cov --workspace --summary-only | tee coverage.txt
    COVERAGE=$(grep "TOTAL" coverage.txt | awk '{print $NF}' | sed 's/%//')
    if (( $(echo "$COVERAGE < 90.0" | bc -l) )); then
      echo "Coverage $COVERAGE% is below 90% threshold"
      exit 1
    fi
```

---

## 🎯 **SUCCESS CRITERIA**

### Phase 1 Success (Week 2)
- ✅ Runtime engine tests: 40+ new tests
- ✅ Coverage increase: +2-3%
- ✅ All runtime E2E workflows covered
- ✅ Error paths tested

### Phase 2 Success (Week 4)
- ✅ Integration tests: 60+ new tests
- ✅ Coverage increase: +7-10% (cumulative)
- ✅ Multi-primal workflows tested
- ✅ Distributed scenarios covered

### Phase 3 Success (Week 6)
- ✅ Chaos tests: 40+ new tests
- ✅ Coverage increase: +12-15% (cumulative)
- ✅ Fault injection comprehensive
- ✅ Recovery scenarios validated

### Final Success (Week 8)
- ✅ **Total coverage: ≥90%**
- ✅ All modules: ≥85%
- ✅ Critical paths: 100%
- ✅ Documentation updated

---

## 🚀 **NEXT STEPS**

### Immediate (This Week)
1. ✅ Install `cargo-llvm-cov`
2. ✅ Baseline coverage measurement
3. ✅ Identify lowest-covered modules
4. ✅ Start Phase 1: Native runtime E2E tests

### Short-term (Next 2 Weeks)
1. ⏳ Complete Phase 1 (Runtime tests)
2. ⏳ Measure coverage gain
3. ⏳ Start Phase 2 (Integration tests)

### Medium-term (Next 6 Weeks)
1. ⏳ Complete Phases 1-2-3
2. ⏳ Achieve 90% coverage
3. ⏳ Update documentation
4. ⏳ Celebrate! 🎉

---

## 📚 **RESOURCES**

### Documentation
- `TESTING.md` - Current test suite documentation
- `TEST_COVERAGE_ANALYSIS_JAN_26_2026.md` - Detailed coverage analysis
- `DEEP_DEBT_EXECUTION_COMPLETE_JAN_27_2026.md` - Deep Debt status

### Tools
- `cargo-llvm-cov` - Coverage measurement
- `cargo-nextest` - Faster test execution
- `cargo-watch` - Continuous testing

### Examples
- `tests/e2e/unified_memory_e2e_tests.rs` - Excellent E2E example (16/16 passing)
- `tests/chaos/network_failures_month2.rs` - Chaos testing example

---

## 🎉 **MOTIVATION**

**Why 90% coverage matters:**

1. ✅ **Confidence**: Deploy to production with certainty
2. ✅ **Refactoring**: Change code safely
3. ✅ **Regression prevention**: Catch bugs early
4. ✅ **Documentation**: Tests as executable specs
5. ✅ **Quality signal**: World-class software standard

**ToadStool is already S++ (99.5%). With 90% coverage, it will be truly legendary.**

---

**Status**: ✅ **READY TO EXECUTE**  
**Timeline**: 6-8 weeks  
**Expected Result**: ≥90% test coverage 🎯

---

*"Great code is tested code. Let's make ToadStool legendary."* 🍄🧪✨
