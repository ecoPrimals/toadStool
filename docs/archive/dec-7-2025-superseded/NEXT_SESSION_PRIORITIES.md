# 🎯 Next Session Priorities
## ToadStool Modernization - Session 2

**Date**: TBD (After December 5, 2025)  
**Duration**: 2-3 hours  
**Focus**: Test Coverage Expansion (Priority 1)

---

## 🎯 SESSION 2 GOALS

### Primary Goal: Test Coverage Expansion
**Target**: 52-60% → 75% coverage (+15-23 percentage points)

**Critical Modules** (0% coverage):
1. `auto_config` (11 files, 0%)
2. `client/core` (0%)  
3. `config_utils` (1.87%)

**Success Criteria**:
- [ ] `auto_config`: 0% → 70%+
- [ ] `client/core`: 0% → 70%+
- [ ] `config_utils`: 1.87% → 70%+
- [ ] Overall: 52-60% → 75%+
- [ ] All existing tests still passing

---

## 📋 PREPARATION

### Before Starting Session 2
1. [ ] Review `COMPREHENSIVE_AUDIT_REPORT_DEC_5_2025_EVENING.md`
2. [ ] Review `DEEP_EVOLUTION_EXECUTION_PLAN_DEC_5_2025.md`
3. [ ] Check STATUS.md for any updates
4. [ ] Verify all tests pass: `cargo test --workspace`
5. [ ] Confirm coverage baseline: Check coverage.json

### Tools Needed
- `cargo-llvm-cov` for coverage measurement
- `proptest` for property-based testing
- Test frameworks already in place

---

## 🔍 DETAILED PLAN

### Task 1: auto_config Module (2-3 hours)

**Files to Test** (11 files):
1. `src/lib.rs` - Main module
2. `src/capability_traits.rs` - Trait definitions
3. `src/ecosystem.rs` - Ecosystem integration
4. `src/hardware.rs` - Hardware detection
5. `src/intelligent.rs` - Intelligent configuration
6. `src/natural_language/` - NL processing
7. `src/squirrel_mcp.rs` - MCP integration
8. Others...

**Testing Strategy**:
```rust
// Property-based tests
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn config_always_valid_or_error(input in ".*") {
            let result = parse_config(&input);
            // Should never panic
            assert!(result.is_ok() || result.is_err());
        }
    }
    
    // Table-driven tests
    #[test]
    fn hardware_detection() {
        let cases = vec![
            (mock_cpu_info(), Ok(CpuCapabilities::...)),
            (invalid_info(), Err(HardwareError::...)),
        ];
        
        for (input, expected) in cases {
            assert_eq!(detect_hardware(input), expected);
        }
    }
    
    // Error path tests
    #[test]
    fn handles_missing_hardware() {
        let result = detect_on_mock_system();
        assert!(result.is_ok()); // Should gracefully handle
    }
}
```

**Expected Coverage**: 0% → 70%+

---

### Task 2: client/core Module (1-2 hours)

**Files to Test**:
- `crates/client/src/client/core.rs`
- Related client infrastructure

**Focus Areas**:
- Client initialization
- Connection handling
- Error scenarios
- Resource management

**Testing Approach**:
- Mock external services
- Test connection lifecycle
- Error path coverage
- Concurrent access patterns

**Expected Coverage**: 0% → 70%+

---

### Task 3: config_utils Module (30-60 minutes)

**Current**: 1.87% coverage

**Files to Test**:
- `crates/core/config/src/config_utils.rs`

**Focus**:
- Port configuration functions
- Service URL generation
- Environment variable handling
- Edge cases

**Testing Strategy**:
```rust
#[test]
fn port_configuration() {
    // Test default ports
    assert_eq!(get_songbird_port(), 8080);
    
    // Test env override
    env::set_var("TOADSTOOL_SONGBIRD_PORT", "9080");
    assert_eq!(get_songbird_port(), 9080);
    
    // Test invalid env
    env::set_var("TOADSTOOL_SONGBIRD_PORT", "invalid");
    assert_eq!(get_songbird_port(), 8080); // Falls back to default
}
```

**Expected Coverage**: 1.87% → 70%+

---

## 📊 SUCCESS METRICS

### Coverage Targets
| Module | Current | Target | Gain |
|--------|---------|--------|------|
| `auto_config` | 0% | 70%+ | +70% |
| `client/core` | 0% | 70%+ | +70% |
| `config_utils` | 1.87% | 70%+ | +68% |
| **Overall** | **52-60%** | **75%+** | **+15-23%** |

### Quality Checks
- [ ] All existing tests still pass
- [ ] No regressions introduced
- [ ] New tests follow best practices
- [ ] Property-based tests where appropriate
- [ ] Error paths covered

---

## 🛠️ TESTING BEST PRACTICES

### 1. Property-Based Testing
Use `proptest` for generating test cases:
```rust
proptest! {
    #[test]
    fn roundtrip_serialization(data in any::<YourType>()) {
        let serialized = serialize(&data)?;
        let deserialized = deserialize(&serialized)?;
        assert_eq!(data, deserialized);
    }
}
```

### 2. Table-Driven Tests
For testing multiple scenarios:
```rust
#[test]
fn validation() {
    let cases = vec![
        ("valid", Ok(())),
        ("invalid", Err(Error::Invalid)),
        ("empty", Err(Error::Empty)),
    ];
    
    for (input, expected) in cases {
        assert_eq!(validate(input), expected);
    }
}
```

### 3. Mock External Dependencies
Use trait-based mocking:
```rust
#[cfg(test)]
mod tests {
    struct MockService;
    
    impl ServiceTrait for MockService {
        fn call(&self) -> Result<Response> {
            Ok(Response::mock())
        }
    }
}
```

### 4. Error Path Testing
Don't just test happy paths:
```rust
#[test]
fn handles_network_error() {
    let mock_failing_service = MockService::with_error();
    let result = client.call_service(mock_failing_service);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::Network(_)));
}
```

---

## 🚨 POTENTIAL ISSUES

### Issue 1: External Dependencies
**Problem**: Some tests may require external services  
**Solution**: Use mocks and trait-based dependency injection

### Issue 2: Async Testing
**Problem**: async tests can be complex  
**Solution**: Use `tokio::test` and `tokio-test` helpers

### Issue 3: Flaky Tests
**Problem**: Tests may be non-deterministic  
**Solution**: Use fixed seeds, avoid timing dependencies

### Issue 4: Slow Tests
**Problem**: Tests may take too long  
**Solution**: Use `#[ignore]` for slow tests, run separately

---

## 📈 PROGRESS TRACKING

### During Session
- [ ] Start: Verify baseline coverage
- [ ] Task 1 complete: Check auto_config coverage
- [ ] Task 2 complete: Check client/core coverage
- [ ] Task 3 complete: Check config_utils coverage
- [ ] End: Generate coverage report

### Commands
```bash
# Generate coverage report
cargo llvm-cov --workspace --html --output-dir target/coverage

# Check specific module
cargo llvm-cov --package toadstool-auto-config --html

# Run tests with coverage
cargo llvm-cov test --workspace
```

---

## 🎯 SESSION 2 DELIVERABLES

1. [ ] Tests for `auto_config` (70%+ coverage)
2. [ ] Tests for `client/core` (70%+ coverage)
3. [ ] Tests for `config_utils` (70%+ coverage)
4. [ ] Coverage report (75%+ overall)
5. [ ] Session summary document
6. [ ] Updated progress tracking

---

## 🔄 IF TIME PERMITS

### Bonus Tasks (if ahead of schedule)
1. [ ] Start `distributed/network` module (20.25% → 70%)
2. [ ] Begin zero-copy profiling (preparation for Session 3)
3. [ ] Document testing patterns for team

---

## 📚 RESOURCES

### Documentation to Reference
- `UNWRAP_ELIMINATION_GUIDE.md` - Error handling patterns
- `MOCK_AUDIT_REPORT_DEC_5_2025.md` - Mock best practices
- `DEEP_EVOLUTION_EXECUTION_PLAN_DEC_5_2025.md` - Overall strategy

### Testing Resources
- Rust book: Chapter 11 (Testing)
- `proptest` documentation
- `tokio-test` documentation

---

## ✅ SESSION 2 SUCCESS CRITERIA

**Must Have**:
- [x] Overall coverage: 52-60% → 75%+
- [x] Critical modules at 70%+
- [x] All existing tests passing
- [x] No regressions

**Nice to Have**:
- [ ] Start distributed module testing
- [ ] Zero-copy profiling preparation
- [ ] Testing documentation

---

## 🚀 AFTER SESSION 2

### Next Priorities (Session 3)
1. **Zero-Copy Optimization** (Priority 2)
   - Profile with flamegraph
   - Identify hot clone paths
   - Implement optimizations
   - Measure 10-20% performance gain

2. **Large File Refactoring** (Priority 3)
   - Smart module extraction
   - Improve test organization

---

## 💡 QUICK START

When you begin Session 2:
1. Run: `cargo test --workspace` (verify baseline)
2. Run: `cargo llvm-cov --workspace --html` (baseline coverage)
3. Start with `auto_config` module
4. Follow testing best practices above
5. Track progress as you go

---

**🎯 Focus: Test Coverage → 75%+**

**Timeline**: 2-3 hours  
**Impact**: HIGH (quality assurance)  
**Difficulty**: MEDIUM

**Let's expand that coverage! 🚀**

---

*Prepared: December 5, 2025*  
*Status: Ready for Session 2*  
*Phase: 3 of 8 (Test Coverage Expansion)*

