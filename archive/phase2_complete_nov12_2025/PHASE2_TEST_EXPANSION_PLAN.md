# 📋 PHASE 2: TEST EXPANSION PLAN

**Goal**: 42.84% → 50% Test Coverage  
**Duration**: 2-3 months  
**Effort**: ~200 new tests  
**Investment**: $20k-40k  
**Status**: 🔄 **Ready to Begin**

---

## 🎯 OBJECTIVES

### Primary Goal
Increase test coverage from **42.84%** to **50%** (minimum 7.16% increase)

### Secondary Goals
1. Cover ALL zero-coverage critical files (Priority 1)
2. Establish E2E test framework
3. Establish chaos test framework
4. Set up continuous coverage monitoring

---

## 📊 CURRENT STATE

### Coverage Breakdown (llvm-cov)
```
Total Regions: 50,924
Covered:       21,815 (42.84%)
Missed:        29,109 (57.16%)

Total Lines:   39,283
Covered:       16,746 (42.63%)
Missed:        22,537 (57.37%)
```

### Zero Coverage Files (~8,000 lines)

**Priority 1 - Business Critical**:
- `crates/cli/src/executor/executor_impl.rs` - 938 lines
- `crates/core/toadstool/src/universal.rs` - 788 lines
- `crates/core/toadstool/src/ecosystem.rs` - 643 lines
- `crates/management/monitoring/src/lib.rs` - 634 lines

**Priority 2 - Hardening Modules**:
- `crates/core/toadstool/src/performance_hardening.rs` - 597 lines
- `crates/core/toadstool/src/security_hardening.rs` - 437 lines
- `crates/core/toadstool/src/production_hardening.rs` - 394 lines

**Priority 3 - Distributed Systems**:
- `crates/distributed/src/songbird_integration/*` - ~1,500 lines
- `crates/distributed/src/substrate_detection.rs` - 439 lines
- `crates/distributed/src/universal/substrate.rs` - 411 lines
- `crates/distributed/src/crypto_lock.rs` - 381 lines

**Priority 4 - Server Components**:
- `crates/cli/src/monitoring.rs` - 522 lines
- `crates/server/src/websocket.rs` - 244 lines
- `crates/server/src/background.rs` - 229 lines

---

## 📅 TIMELINE & MILESTONES

### Month 1: Foundation (Weeks 1-4)

**Week 1: Setup & Planning**
- [ ] Set up continuous coverage monitoring (CI/CD integration)
- [ ] Create test template library
- [ ] Set up mock/fixture infrastructure
- [ ] Create coverage dashboard

**Week 2-3: Priority 1 Files (Part 1)**
- [ ] Add 50 tests for `executor_impl.rs` → 30% coverage
- [ ] Add 40 tests for `universal.rs` → 25% coverage
- [ ] Target: 44% overall coverage

**Week 4: Priority 1 Files (Part 2)**
- [ ] Add 30 tests for `ecosystem.rs` → 25% coverage
- [ ] Add 20 tests for `monitoring/lib.rs` → 20% coverage
- [ ] Target: 45% overall coverage

**Milestone 1**: 45% coverage ✅

---

### Month 2: Expansion (Weeks 5-8)

**Week 5-6: Hardening Modules**
- [ ] Add 25 tests for `performance_hardening.rs` → 30% coverage
- [ ] Add 20 tests for `security_hardening.rs` → 30% coverage
- [ ] Add 15 tests for `production_hardening.rs` → 25% coverage
- [ ] Target: 47% overall coverage

**Week 7: Distributed Systems (Part 1)**
- [ ] Add 30 tests for `songbird_integration/*` → 15% coverage
- [ ] Add 15 tests for `substrate_detection.rs` → 20% coverage
- [ ] Target: 48% overall coverage

**Week 8: E2E Framework Setup**
- [ ] Docker test environment
- [ ] Basic E2E infrastructure
- [ ] 3 working E2E tests
- [ ] Target: 48.5% overall coverage

**Milestone 2**: 48% coverage ✅

---

### Month 3: Completion (Weeks 9-12)

**Week 9-10: Server Components**
- [ ] Add 15 tests for `cli/monitoring.rs` → 20% coverage
- [ ] Add 10 tests for `server/websocket.rs` → 25% coverage
- [ ] Add 10 tests for `server/background.rs` → 25% coverage
- [ ] Target: 49% overall coverage

**Week 11: Distributed Systems (Part 2)**
- [ ] Add 15 tests for `universal/substrate.rs` → 20% coverage
- [ ] Add 10 tests for `crypto_lock.rs` → 15% coverage
- [ ] Target: 49.5% overall coverage

**Week 12: Chaos Framework & Buffer**
- [ ] Chaos test infrastructure
- [ ] 3 working chaos tests
- [ ] Buffer for any missed targets
- [ ] Documentation updates
- [ ] Target: 50% overall coverage ✅

**Milestone 3**: **50% coverage** 🎉

---

## 📝 TEST DISTRIBUTION

### Total New Tests: ~200

| Category | Tests | Target Coverage |
|----------|-------|-----------------|
| **Priority 1: Business Critical** | 140 | 25-30% |
| - executor_impl.rs | 50 | 30% |
| - universal.rs | 40 | 25% |
| - ecosystem.rs | 30 | 25% |
| - monitoring/lib.rs | 20 | 20% |
| **Priority 2: Hardening** | 60 | 25-30% |
| - performance_hardening.rs | 25 | 30% |
| - security_hardening.rs | 20 | 30% |
| - production_hardening.rs | 15 | 25% |
| **Priority 3: Distributed** | 70 | 15-20% |
| - songbird_integration/* | 30 | 15% |
| - substrate_detection.rs | 15 | 20% |
| - universal/substrate.rs | 15 | 20% |
| - crypto_lock.rs | 10 | 15% |
| **Priority 4: Server** | 35 | 20-25% |
| - cli/monitoring.rs | 15 | 20% |
| - server/websocket.rs | 10 | 25% |
| - server/background.rs | 10 | 25% |
| **E2E & Chaos** | 6 | Foundation |
| - E2E tests | 3 | Framework |
| - Chaos tests | 3 | Framework |

---

## 🛠️ TEST TYPES BREAKDOWN

### Unit Tests (~170)
Focus on:
- Function-level behavior
- Error handling paths
- Edge cases
- Input validation
- State transitions

### Integration Tests (~24)
Focus on:
- Module interactions
- Configuration loading
- Resource management
- API contracts

### E2E Tests (~3)
Focus on:
- Full system workflows
- Real runtime execution
- End-to-end scenarios

### Chaos Tests (~3)
Focus on:
- Failure recovery
- Resource pressure
- Network issues

---

## 📋 PRIORITY 1 DETAILED PLAN

### executor_impl.rs (938 lines → 50 tests)

**Focus Areas**:
1. **Workload Execution** (20 tests)
   - Start/stop/pause/resume
   - Different runtime types
   - Error handling
   - Resource allocation

2. **State Management** (15 tests)
   - State transitions
   - Concurrent executions
   - State persistence
   - Recovery scenarios

3. **Resource Management** (10 tests)
   - Resource limits
   - Resource cleanup
   - Resource contention
   - Resource monitoring

4. **Error Scenarios** (5 tests)
   - Execution failures
   - Timeout handling
   - Invalid workloads
   - Recovery mechanisms

**Test Files**:
- `crates/cli/tests/executor_impl_basic_tests.rs` (basic functionality)
- `crates/cli/tests/executor_impl_state_tests.rs` (state management)
- `crates/cli/tests/executor_impl_resource_tests.rs` (resource management)
- `crates/cli/tests/executor_impl_error_tests.rs` (error handling)

---

### universal.rs (788 lines → 40 tests)

**Focus Areas**:
1. **Universal Adapter** (15 tests)
   - Runtime selection
   - Workload routing
   - Fallback mechanisms
   - Capability detection

2. **Platform Detection** (10 tests)
   - OS detection
   - Architecture detection
   - Runtime availability
   - Feature detection

3. **Compatibility Layer** (10 tests)
   - OS compatibility
   - Runtime compatibility
   - Version compatibility
   - Feature compatibility

4. **Error Handling** (5 tests)
   - Unsupported platforms
   - Missing runtimes
   - Configuration errors
   - Fallback failures

**Test Files**:
- `crates/core/toadstool/tests/universal_adapter_tests.rs`
- `crates/core/toadstool/tests/universal_platform_tests.rs`
- `crates/core/toadstool/tests/universal_compat_tests.rs`

---

### ecosystem.rs (643 lines → 30 tests)

**Focus Areas**:
1. **Service Discovery** (10 tests)
   - Service registration
   - Service lookup
   - Service health checks
   - Service removal

2. **Service Communication** (10 tests)
   - Inter-service calls
   - Message passing
   - Protocol handling
   - Error propagation

3. **Ecosystem Management** (10 tests)
   - Ecosystem lifecycle
   - Service dependencies
   - Configuration updates
   - Health monitoring

**Test Files**:
- `crates/core/toadstool/tests/ecosystem_discovery_tests.rs`
- `crates/core/toadstool/tests/ecosystem_communication_tests.rs`
- `crates/core/toadstool/tests/ecosystem_management_tests.rs`

---

### monitoring/lib.rs (634 lines → 20 tests)

**Focus Areas**:
1. **Metrics Collection** (8 tests)
   - Metric registration
   - Metric recording
   - Metric aggregation
   - Metric export

2. **Health Monitoring** (6 tests)
   - Health checks
   - Status updates
   - Alert generation
   - Health aggregation

3. **Performance Monitoring** (6 tests)
   - Performance metrics
   - Resource usage
   - Bottleneck detection
   - Performance reports

**Test Files**:
- `crates/management/monitoring/tests/metrics_tests.rs`
- `crates/management/monitoring/tests/health_tests.rs`
- `crates/management/monitoring/tests/performance_tests.rs`

---

## 🧪 TEST INFRASTRUCTURE

### Test Templates

**Unit Test Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_success() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.field, expected_value);
    }
    
    #[test]
    fn test_function_error_handling() {
        // Arrange
        let invalid_input = create_invalid_input();
        
        // Act
        let result = function_under_test(invalid_input);
        
        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ExpectedErrorType));
    }
}
```

**Integration Test Template**:
```rust
#[tokio::test]
async fn test_integration_scenario() {
    // Arrange
    let context = setup_test_context().await;
    
    // Act
    let result = execute_scenario(&context).await;
    
    // Assert
    assert!(result.is_ok());
    validate_scenario_outcome(&context).await;
    
    // Cleanup
    teardown_test_context(context).await;
}
```

### Mock/Fixture Infrastructure

**Create**:
- `crates/testing/src/fixtures/executors.rs` - Executor fixtures
- `crates/testing/src/fixtures/workloads.rs` - Workload fixtures
- `crates/testing/src/fixtures/ecosystems.rs` - Ecosystem fixtures
- `crates/testing/src/mocks/runtimes.rs` - Runtime mocks
- `crates/testing/src/mocks/services.rs` - Service mocks

---

## 📊 COVERAGE MONITORING

### Continuous Integration
```yaml
# .github/workflows/coverage.yml
name: Coverage
on: [push, pull_request]
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov --lib --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

### Coverage Dashboard
- Set up Codecov or Coveralls
- Daily coverage reports
- Per-PR coverage diffs
- Coverage trend graphs

### Coverage Gates
- **Minimum**: No PR can decrease coverage
- **Target**: Each PR should increase coverage by 0.1%+
- **Ideal**: All new code has 80%+ coverage

---

## 💰 RESOURCE ALLOCATION

### Team Structure
- **Lead Developer** (50% time) - Architecture, reviews
- **2x Mid-Level Developers** (100% time) - Test implementation
- **QA Engineer** (50% time) - Test validation, coverage monitoring

### Weekly Effort
- **Test Writing**: 60 hours/week
- **Code Review**: 10 hours/week
- **Infrastructure**: 10 hours/week
- **Monitoring**: 5 hours/week

**Total**: ~85 hours/week × 12 weeks = ~1,020 hours

### Budget Breakdown
- **Personnel**: $15k-30k (depends on rates)
- **Infrastructure**: $2k-5k (CI/CD, monitoring tools)
- **Tools**: $1k-2k (coverage tools, testing frameworks)
- **Buffer**: $2k-3k (unexpected issues)

**Total**: $20k-40k

---

## 🎯 SUCCESS CRITERIA

### Phase 2 Complete When:
- ✅ Overall coverage ≥ 50%
- ✅ All Priority 1 files have ≥ 20% coverage
- ✅ E2E framework operational (3 tests working)
- ✅ Chaos framework operational (3 tests working)
- ✅ Coverage monitoring in place
- ✅ Test documentation complete
- ✅ All tests passing (100%)
- ✅ Zero production warnings maintained

---

## 🚀 NEXT STEPS

### Immediate (This Week)
1. **Set up coverage monitoring** (Day 1-2)
   - Install coverage tools in CI/CD
   - Set up coverage dashboard
   - Configure coverage gates

2. **Create test infrastructure** (Day 3-4)
   - Test templates
   - Mock/fixture library
   - Test helpers

3. **Begin Priority 1** (Day 5)
   - Start executor_impl.rs tests
   - First 10-15 tests

### Week 2 Kickoff
- Team alignment meeting
- Review test templates
- Assign initial test files
- Set up daily standups

---

## 📞 TRACKING & REPORTING

### Daily
- Coverage percentage
- Tests added/passing
- Blockers identified

### Weekly
- Coverage progress chart
- Tests added by category
- Review completed PRs
- Adjust plan if needed

### Monthly
- Milestone review
- Budget vs actual
- Timeline adjustments
- Phase 3 planning begins (Month 3)

---

## 🔗 RELATED DOCUMENTS

- `FRESH_COMPREHENSIVE_AUDIT_NOV_12_2025.md` - Initial audit
- `EXECUTION_SUMMARY_NOV_12_2025.md` - Current state
- `STAGING_DEPLOYMENT_READINESS.md` - Deployment status

---

**🍄 Phase 2: Foundation for Production Readiness**

*Status: 🔄 Ready to Begin*  
*Timeline: 2-3 months*  
*Goal: 42.84% → 50% coverage*  
*Investment: $20k-40k*

---

**END OF PHASE 2 PLAN**

