# 🎯 Test Coverage Roadmap - December 5, 2025

**Current Coverage**: 41.9% (17,902 / 42,685 lines)  
**Target Coverage**: 90%  
**Gap**: 48.1 percentage points  
**Timeline**: 4-6 weeks

---

## 📊 CURRENT STATE (Measured via llvm-cov)

### Overall Metrics
- **Lines**: 41.9% (17,902 / 42,685)
- **Functions**: 43.8% (2,384 / 5,444)
- **Regions**: 42.3% (23,423 / 55,402)

### Module Breakdown

#### ✅ Good Coverage (>70%)
- `auto_config/hardware.rs`: 83.3%
- `auto_config/intent.rs`: 96.2%
- `auto_config/types.rs`: 100.0%
- `auto_config/templates.rs`: 80.2%
- `auto_config/capability_traits.rs`: 70.8%

#### ⚠️ Needs Improvement (40-70%)
- `auto_config/lib.rs`: 56.5%
- `auto_config/natural_language/mod.rs`: 50.8%
- `config_utils_simple_tests.rs`: Various (1.87-60%)

#### 🔴 Critical Gaps (<40%)
- **`auto_config/intelligent.rs`: 10.8%** ← PRIORITY 1
- **`auto_config/squirrel_mcp.rs`: 12.8%** ← PRIORITY 2
- **`auto_config/ecosystem.rs`: 39.8%** ← PRIORITY 3

---

## 🎯 WEEK 1 PRIORITIES (41.9% → 55%)

### Priority 1: intelligent.rs (10.8% → 70%)

**File**: `crates/auto_config/src/intelligent.rs` (925 lines)  
**Current**: 10.8% coverage  
**Target**: 70%  
**Impact**: High - Core auto-configuration logic

**Untested Critical Paths**:
1. `auto_configure()` - Zero-touch configuration (main entry point)
2. `generate_intelligent_config()` - Config generation pipeline
3. `generate_optimal_config()` - Optimal config based on system analysis
4. `validate_configuration()` - Config validation
5. `optimize_for_hardware_class()` - Performance-based optimization
6. `apply_platform_specific_settings()` - Platform optimizations
7. `integrate_ecosystem_services()` - Service integration
8. `apply_security_defaults()` - Security configuration

**Test Plan**:
```rust
// New tests needed:
#[tokio::test]
async fn test_auto_configure_success() {
    // Test full auto-configuration flow
}

#[tokio::test]
async fn test_generate_intelligent_config_phases() {
    // Test each phase of config generation
}

#[tokio::test]
async fn test_generate_optimal_config_hardware_classes() {
    // Test config generation for different hardware profiles
}

#[tokio::test]
async fn test_validate_configuration_success() {
    // Test config validation passes for valid configs
}

#[tokio::test]
async fn test_validate_configuration_failures() {
    // Test config validation catches invalid configs
}

#[tokio::test]
async fn test_optimize_for_low_end_hardware() {
    // Test optimizations for limited resources
}

#[tokio::test]
async fn test_optimize_for_high_end_hardware() {
    // Test optimizations for powerful systems
}

#[tokio::test]
async fn test_platform_specific_settings() {
    // Test Linux/macOS/Windows specific settings
}

#[tokio::test]
async fn test_ecosystem_integration() {
    // Test integration with discovered services
}

#[tokio::test]
async fn test_security_defaults() {
    // Test security configuration defaults
}
```

**Estimated Effort**: 6-8 hours  
**Coverage Gain**: ~55 percentage points in this file

---

### Priority 2: squirrel_mcp.rs (12.8% → 60%)

**File**: `crates/auto_config/src/squirrel_mcp.rs` (275 lines)  
**Current**: 12.8% coverage  
**Target**: 60%  
**Impact**: Medium - AI integration

**Untested Paths**:
1. `handle_configure_request()` - Main request handler
2. `parse_natural_language()` - NL parsing
3. `apply_ai_suggestions()` - AI-driven configuration
4. `generate_mcp_response()` - Response generation

**Test Plan**:
```rust
#[tokio::test]
async fn test_handle_configure_request_simple() {
    // Test basic configuration request
}

#[tokio::test]
async fn test_parse_natural_language_intents() {
    // Test NL intent recognition
}

#[tokio::test]
async fn test_ai_suggestions_integration() {
    // Test AI-suggested configurations
}

#[tokio::test]
async fn test_mcp_response_format() {
    // Test MCP response structure
}
```

**Estimated Effort**: 3-4 hours  
**Coverage Gain**: ~45 percentage points in this file

---

### Priority 3: ecosystem.rs (39.8% → 70%)

**File**: `crates/auto_config/src/ecosystem.rs`  
**Current**: 39.8% coverage  
**Target**: 70%  
**Impact**: Medium - Service discovery

**Untested Paths**:
1. Service discovery edge cases
2. Network scanning failures
3. Service health check timeouts
4. Multiple service instances

**Test Plan**:
```rust
#[tokio::test]
async fn test_discover_services_with_failures() {
    // Test discovery with some services unavailable
}

#[tokio::test]
async fn test_network_scan_timeout() {
    // Test timeout handling
}

#[tokio::test]
async fn test_multiple_service_instances() {
    // Test handling multiple instances of same service
}

#[tokio::test]
async fn test_service_health_checks() {
    // Test health check integration
}
```

**Estimated Effort**: 2-3 hours  
**Coverage Gain**: ~30 percentage points in this file

---

## 📅 WEEK 2 PRIORITIES (55% → 70%)

### Client Core Module

**Target**: Core client functionality  
**Current**: Unknown (need to analyze)  
**Goal**: 70%

**Investigation needed**:
1. Identify untested client methods
2. Focus on request/response handling
3. Test error scenarios
4. Test retry logic

**Estimated Effort**: 8-10 hours

---

### Config Utils Module

**File**: `crates/core/config/src/config_utils.rs`  
**Current**: 1.87% (claimed, but tests exist)  
**Goal**: 70%

**Focus**:
1. Environment variable handling
2. Port allocation logic
3. Configuration validation
4. Error handling

**Estimated Effort**: 4-6 hours

---

## 📅 WEEK 3-4 PRIORITIES (70% → 85%)

### Integration Tests

**Focus**:
1. End-to-end workflows
2. Multi-module integration
3. Error propagation across modules
4. Performance under load

**Estimated Effort**: 12-15 hours

---

### Property-Based Testing

**Focus**:
1. Configuration generation with random inputs
2. Invariant testing (all configs are valid)
3. Fuzzing edge cases

**Estimated Effort**: 6-8 hours

---

## 📅 WEEK 5-6 PRIORITIES (85% → 90%)

### Edge Cases & Error Paths

**Focus**:
1. Network failures
2. Hardware detection errors
3. Resource exhaustion
4. Concurrent access patterns

**Estimated Effort**: 8-10 hours

---

### Chaos & Fault Testing

**Focus**:
1. Injected failures
2. Timeout scenarios
3. Partial system availability
4. Recovery mechanisms

**Estimated Effort**: 6-8 hours

---

## 🔧 TESTING PATTERNS TO APPLY

### Pattern 1: Unit Tests for Core Logic

```rust
#[test]
fn test_calculate_optimal_workers() {
    let cores = 8;
    let memory_gb = 16.0;
    let workers = calculate_optimal_workers(cores, memory_gb);
    assert_eq!(workers, 6); // Example: cores - 2
}
```

### Pattern 2: Integration Tests with Mocks

```rust
#[tokio::test]
async fn test_auto_configure_with_mock_hardware() {
    let mut auto_config = IntelligentAutoConfig::new();
    // Mock hardware detection
    let config = auto_config.generate_intelligent_config().await.unwrap();
    assert!(config.app.worker_threads > 0);
}
```

### Pattern 3: Property-Based Tests

```rust
#[test]
fn test_config_always_valid() {
    proptest!(|(cores in 1..128u32, memory in 1.0..1024.0f64)| {
        let config = generate_config_for_hardware(cores, memory);
        assert!(config.validate().is_ok());
    });
}
```

### Pattern 4: Error Path Tests

```rust
#[tokio::test]
async fn test_auto_configure_handles_hardware_failure() {
    // Simulate hardware detection failure
    let result = auto_configure_with_failing_hardware().await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::HardwareDetection(_)));
}
```

---

## 📊 EXPECTED COVERAGE PROGRESSION

| **Week** | **Starting** | **Target** | **Gain** | **Focus** |
|----------|--------------|------------|----------|-----------|
| Week 1 | 41.9% | 55% | +13.1% | intelligent.rs, squirrel_mcp.rs, ecosystem.rs |
| Week 2 | 55% | 70% | +15% | client core, config_utils |
| Week 3 | 70% | 78% | +8% | Integration tests |
| Week 4 | 78% | 85% | +7% | Property-based testing |
| Week 5 | 85% | 88% | +3% | Edge cases & error paths |
| Week 6 | 88% | 90% | +2% | Chaos & fault testing |

**Final**: 90% coverage ✨

---

## 🎯 SUCCESS METRICS

### Code Coverage
- [ ] Overall coverage ≥ 90%
- [ ] No module < 70% coverage
- [ ] Critical paths 100% covered

### Test Quality
- [ ] All tests pass consistently
- [ ] No flaky tests
- [ ] Fast test execution (< 10s for unit tests)
- [ ] Clear test names and documentation

### Regression Prevention
- [ ] Error paths tested
- [ ] Edge cases covered
- [ ] Integration scenarios validated
- [ ] Performance benchmarks established

---

## 🚀 GETTING STARTED (Next Session)

### Step 1: Verify Baseline (5 minutes)

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
cargo llvm-cov --lib --workspace --json --output-path coverage-baseline.json
python3 -c "import json; d=json.load(open('coverage-baseline.json')); print(f\"Coverage: {d['data'][0]['totals']['lines']['percent']:.1f}%\")"
```

### Step 2: Create Test File (10 minutes)

```bash
# Create new test file for intelligent.rs critical paths
cat > crates/auto_config/tests/intelligent_critical_paths_tests.rs << 'EOF'
//! Critical path tests for intelligent auto-configuration
//! Target: Increase intelligent.rs coverage from 10.8% to 70%

use toadstool_auto_config::IntelligentAutoConfig;

#[tokio::test]
async fn test_auto_configure_basic_success() {
    // TODO: Implement
}

#[tokio::test]
async fn test_generate_intelligent_config_phases() {
    // TODO: Implement
}

// Add more tests...
EOF
```

### Step 3: Implement First Test (30 minutes)

Focus on `auto_configure()` - the main entry point

### Step 4: Verify Coverage Increase (5 minutes)

```bash
cargo llvm-cov --lib --workspace --json --output-path coverage-after-test.json
# Compare with baseline
```

### Step 5: Iterate

Repeat Steps 3-4 until reaching 70% for intelligent.rs

---

## 📝 NOTES

### Why Coverage Is Low Despite Tests Existing

1. **Tests exist but don't exercise main code paths**
   - Tests create objects but don't call main methods
   - Tests focus on struct validation, not business logic

2. **Integration tests aren't counted in lib coverage**
   - Many tests are in `tests/` directory (integration tests)
   - These don't show up in `--lib` coverage reports

3. **Async tests might have execution issues**
   - Some async tests might not complete properly
   - Timeout issues could skip code paths

### Recommendations

1. **Focus on main entry points first**
   - `auto_configure()` is the primary API
   - Test this thoroughly before helper methods

2. **Use test coverage as a guide, not a goal**
   - 90% coverage doesn't mean bug-free
   - Quality > quantity

3. **Write readable, maintainable tests**
   - Clear test names
   - Good documentation
   - One assertion per test when possible

---

**Created**: December 5, 2025  
**Status**: Ready for execution  
**Timeline**: 4-6 weeks to 90% coverage  
**Confidence**: HIGH - Clear path, achievable goals ✅

---

*"Every line tested is a line we trust. Let's build confidence one test at a time."* 🧪

