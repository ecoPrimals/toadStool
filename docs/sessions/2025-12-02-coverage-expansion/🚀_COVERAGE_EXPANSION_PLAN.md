# 🚀 COVERAGE EXPANSION PLAN - December 2, 2025

**Start Date**: December 2, 2025  
**Current Coverage**: 42.48%  
**Target Coverage**: 50% (first milestone)  
**Status**: 🔄 **IN PROGRESS**

---

## 🎯 OBJECTIVE

Expand test coverage from **42.48% → 50%** by adding **200-300 focused unit tests** for low-coverage modules.

**Timeline**: 2-3 weeks  
**Priority**: HIGHEST (critical path to 90%)

---

## 📊 TARGET MODULES (0-22% Coverage)

### **Priority 1: API Handlers** (0-22%, Quick Wins)

| Handler | Current | Lines | Target | Effort |
|---------|---------|-------|--------|--------|
| `health.rs` | 0% | 55 | 90% | 1 hour |
| `metrics.rs` | 0% | 87 | 90% | 2 hours |
| `cluster.rs` | 0% | 64 | 80% | 2 hours |
| `helpers.rs` | 0% | 24 | 100% | 1 hour |
| `logs.rs` | 0% | 74 | 80% | 2 hours |
| `execution.rs` | 0% | 206 | 70% | 4 hours |
| `workload.rs` | 0% | 29 | 90% | 1 hour |

**Total**: 539 lines, ~13 hours effort

### **Priority 2: Server Modules** (0-40%)

| Module | Current | Lines | Target | Effort |
|--------|---------|-------|--------|--------|
| `server/src/background.rs` | 0% | 267 | 70% | 5 hours |
| `server/src/websocket.rs` | 2.98% | 168 | 60% | 4 hours |
| `server/src/handlers.rs` | ~20% | ~200 | 70% | 4 hours |

**Total**: 635 lines, ~13 hours effort

### **Priority 3: Security Modules** (0-40%)

| Module | Current | Lines | Target | Effort |
|--------|---------|-------|--------|--------|
| `security/sandbox/` | 0-20% | ~500 | 60% | 6 hours |
| `security/policies/` | 0-20% | ~600 | 60% | 7 hours |

**Total**: 1,100 lines, ~13 hours effort

---

## 📋 EXECUTION PLAN

### **Week 1: API Handlers** (Quick Wins)

**Goal**: Add 80-100 tests, boost coverage to 45%

**Day 1-2: Simple Handlers** (4 hours)
- ✅ Session started: December 2, 2025
- [ ] `health.rs` - health check endpoint
- [ ] `metrics.rs` - metrics collection
- [ ] `helpers.rs` - utility functions
- [ ] `workload.rs` - workload handlers

**Day 3-4: Complex Handlers** (6 hours)
- [ ] `cluster.rs` - cluster management
- [ ] `logs.rs` - log streaming
- [ ] `execution.rs` - execution management

**Expected**: 
- Coverage: 42.48% → 45%
- Tests added: 80-100
- Lines covered: +400-500

### **Week 2: Server & Security** (Medium Effort)

**Goal**: Add 100-120 tests, boost coverage to 48%

**Day 1-3: Server Modules** (13 hours)
- [ ] `background.rs` tests
- [ ] `websocket.rs` tests  
- [ ] `handlers.rs` expansion

**Day 4-5: Security Start** (7 hours)
- [ ] Sandbox tests
- [ ] Policy tests (start)

**Expected**:
- Coverage: 45% → 48%
- Tests added: 100-120
- Lines covered: +500-600

### **Week 3: Security & Polish** (Final Push)

**Goal**: Add 80-100 tests, reach 50%+

**Day 1-3: Security Completion** (10 hours)
- [ ] Complete sandbox coverage
- [ ] Complete policy coverage

**Day 4-5: Gap Fill** (6 hours)
- [ ] Identify remaining gaps
- [ ] Add targeted tests
- [ ] Verify 50% milestone

**Expected**:
- Coverage: 48% → 50%+
- Tests added: 80-100
- Lines covered: +400-500

---

## 🎯 TEST PATTERNS

### **Unit Test Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handler_success_case() {
        // ✅ MODERN CONCURRENT TEST
        // Arrange
        let handler = setup_handler();
        
        // Act
        let result = handler.handle_request(request).await;
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 200);
    }
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_handler_error_case() {
        // Test error handling
        let result = handler.handle_invalid(request).await;
        assert!(result.is_err());
    }
}
```

### **Testing Principles**:
1. ✅ **100% Concurrent** - No serial tests
2. ✅ **Event-Driven** - Barriers, channels, atomics
3. ✅ **Zero Sleeps** - Only justified performance/polling
4. ✅ **Comprehensive** - Success + error paths
5. ✅ **Fast** - <1s per test preferred

---

## 📈 PROGRESS TRACKING

### **Metrics**:
- **Start**: 42.48% (December 2, 2025)
- **Week 1 Target**: 45%
- **Week 2 Target**: 48%
- **Week 3 Target**: 50%+

### **Test Count**:
- **Current**: 118 passing
- **Week 1 Target**: 200+ passing
- **Week 2 Target**: 300+ passing
- **Week 3 Target**: 400+ passing

### **Daily Progress Log**:

**Day 1 (Dec 2, 2025)**:
- 🔄 Starting with health.rs
- Session begins...

---

## 🎊 SUCCESS CRITERIA

### **Must Have** (Week 1):
- [ ] Coverage ≥ 45%
- [ ] All API handlers ≥ 70% coverage
- [ ] 80+ new tests passing
- [ ] Zero test failures
- [ ] Clean clippy

### **Should Have** (Week 2):
- [ ] Coverage ≥ 48%
- [ ] Server modules ≥ 60% coverage
- [ ] 180+ new tests total
- [ ] Documentation updated

### **Nice to Have** (Week 3):
- [ ] Coverage ≥ 50%
- [ ] Security modules ≥ 60% coverage
- [ ] 250+ new tests total
- [ ] Coverage HTML report beautiful

---

## 🚀 NEXT ACTIONS

### **Immediate** (Now):
1. 🔄 Create tests for `health.rs`
2. Create tests for `metrics.rs`
3. Create tests for `helpers.rs`
4. Verify coverage increase

### **This Afternoon**:
5. Create tests for `workload.rs`
6. Start on `cluster.rs`
7. Run full coverage report

### **Tomorrow**:
8. Complete `cluster.rs` tests
9. Start on `logs.rs`
10. Begin `execution.rs` planning

---

**Status**: 🔄 IN PROGRESS  
**Current Focus**: health.rs handler tests  
**Next Milestone**: 45% coverage (Week 1)

🍄 **ToadStool - Coverage Expansion Begins!** ✨


