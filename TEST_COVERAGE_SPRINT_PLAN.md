# Test Coverage Sprint Plan

**Date:** November 16, 2025 - Late Evening  
**Goal:** Continue P1 coverage improvements  
**Current Coverage:** 52.02%  
**Target:** 55-60% by end of sprint

---

## Sprint Status

### Completed ✅
1. **BYOB Core Platform** (Phase 3 Evening)
   - byob_impl.rs: 0% → 66.67%
   - deployment.rs: 0% → 93.60%
   - 23 comprehensive tests
   - Duration: 2-3 hours

### In Progress 🔄
2. **Zero-Config System** (Current)
   - Target: 0% → 60%
   - Size: 1,442 lines across 7 files
   - Critical user-facing feature
   - Est: 15-25 hours

---

## Component Analysis

### Zero-Config System (1,442 lines)

**Files:**
- `discovery.rs` (467 lines) - System & ecosystem discovery
- `types.rs` (433 lines) - Data structures [35 tests exist]
- `deployment.rs` (168 lines) - Service deployment
- `configuration.rs` (150 lines) - Config generation
- `core.rs` (139 lines) - 5-phase orchestration
- `verification.rs` (55 lines) - Health checks
- `mod.rs` (30 lines) - Module organization

**Current Tests:**
- ✅ 35 struct/serialization tests (types)
- ❌ 0 functional tests (core process)
- ❌ 0 discovery tests  
- ❌ 0 deployment tests
- ❌ 0 verification tests

**Priority Tests Needed:**
1. **Core Bootstrap Process** (HIGH)
   - 5-phase deployment flow
   - Timing validation (<60s)
   - Error handling
   
2. **System Discovery** (HIGH)
   - CPU detection
   - Memory detection
   - Storage detection
   - Network detection
   - Container runtime detection

3. **Ecosystem Discovery** (MEDIUM)
   - Songbird detection
   - BearDog detection
   - NestGate detection

4. **Configuration Generation** (MEDIUM)
   - Auto-config creation
   - Optimization based on system
   - Serialization

5. **Service Deployment** (HIGH)
   - Deployment execution
   - Service lifecycle
   - Error recovery

6. **Health Verification** (MEDIUM)
   - Health checks
   - Service validation

---

## Remaining Sprint Targets

### High Priority (Next Up)
1. **CLI Ecosystem** (13.71% → 70%)
   - ecosystem/integrator_impl.rs (1,206 lines)
   - Est: 30-40 hours

2. **Network Config** (0% → 60%)
   - Est: 15-25 hours

### Medium Priority
3. **Client Core** (0% → 60%)
   - Needs HTTP mocking setup
   - Est: 15-20 hours

### Lower Priority (P2)
4. **Unwrap Reduction** (2,255 → 800)
   - Use migrator tool
   - Est: 40-60 hours

---

## Time Estimates

### Completed Today
- Reality Check: 3-4h
- P0 Fixes: 2h
- BYOB Tests: 2-3h
- Doc Cleanup: 0.5h
- **Total: ~8-10h**

### Remaining for 60% Coverage
- Zero-Config: 15-25h
- CLI Ecosystem: 30-40h
- Network Config: 15-25h
- **Subtotal: 60-90h**

### For 70% Coverage (Production)
- Above + Client Core: 15-20h
- Above + Additional polish: 10-20h
- **Total: 85-130h (2-3 weeks)**

---

## Strategy

### Tonight's Sprint
Given the late hour, focus on:
1. Create comprehensive zero-config functional tests
2. Target 20-30 high-quality tests
3. Focus on core bootstrap process
4. Aim for 40-50% zero-config coverage

### Tomorrow/Next Session
1. Complete zero-config (60% coverage)
2. Begin CLI ecosystem tests
3. Continue momentum

---

## Quality Standards (from BYOB)

✅ **Established Patterns:**
- Comprehensive > shallow (23 thorough > 100 basic)
- Error message validation
- Edge cases covered
- Integration scenarios tested
- Reusable test infrastructure
- No unwraps in test code

✅ **Apply to Zero-Config:**
- Mock system detection
- Test all 5 phases
- Validate timing requirements
- Test error scenarios
- Test edge cases (no resources, missing ecosystem)

---

## Success Metrics

### For This Sprint
- [ ] 20-30 zero-config functional tests created
- [ ] Core bootstrap process tested
- [ ] System discovery tested
- [ ] Configuration generation tested
- [ ] Overall coverage: 52.02% → 53-54%

### For Full Zero-Config
- [ ] 60% coverage of zero-config module
- [ ] All 5 phases tested
- [ ] Error scenarios covered
- [ ] Performance validated (<60s requirement)
- [ ] All tests passing

---

**Status:** Sprint plan ready  
**Next:** Create zero-config functional tests  
**Est. Time:** 1-2 hours for initial batch

