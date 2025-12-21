# 🚀 NEXT PRIORITIES - Dec 1, 2025

## Current Status
- ✅ **Coverage**: 60.12% (up from 56.70%)
- ✅ **Tests**: ALL 1727+ PASSING
- ✅ **Test Speed**: 9.4x faster (E2E modernized)
- 🎯 **Target**: 90% coverage (29.88% to go)

---

## Priority 1: Coverage Expansion (60% → 90%)

### Critical Low-Coverage Modules (Based on Historical Analysis)

#### **Tier 1 - CRITICAL (0-20% coverage)**
These modules have minimal or no test coverage and are production-critical:

1. **CLI Executor** (`cli/src/executor/executor_impl.rs`)
   - Lines: 938
   - Coverage: ~0%
   - Priority: **CRITICAL** - Core CLI functionality
   - Tests Needed: Execution lifecycle, error handling, resource management

2. **Core Ecosystem** (`core/toadstool/src/ecosystem.rs`)
   - Lines: 643
   - Coverage: ~0%
   - Priority: **CRITICAL** - Foundation layer
   - Tests Needed: Service discovery, health checks, coordination

3. **Server WebSocket** (`server/src/websocket.rs`)
   - Lines: 244
   - Coverage: ~0%
   - Priority: **HIGH** - Real-time communication
   - Tests Needed: Connection handling, message routing, error recovery

4. **Server Background** (`server/src/background.rs`)
   - Lines: 229
   - Coverage: ~30% (improved from 0%)
   - Priority: **HIGH** - Background services (partially covered)
   - Tests Needed: Remaining edge cases, error paths

#### **Tier 2 - HIGH PRIORITY (20-40% coverage)**
Modules with some coverage but significant gaps:

5. **Distributed Coordinator** (`distributed/`)
   - Current: ~43%
   - Target: 80%+
   - Tests Needed: Federation, fault tolerance, state recovery

6. **API Layer** (`api/`)
   - Current: ~45%
   - Target: 80%+
   - Tests Needed: Middleware chains, error handling, rate limiting

7. **Server Handlers** (already good at ~95%, maintain)

#### **Tier 3 - MEDIUM PRIORITY (40-60% coverage)**
8. **Zero-Config System** (`cli/src/zero_config/`)
   - Current: ~48%
   - Target: 75%+
   - Tests Needed: Auto-discovery, deployment scenarios

9. **Network Configuration** (`cli/src/network_config/`)
   - Current: ~51%
   - Target: 75%+
   - Tests Needed: Traffic management, service mesh

10. **Runtime Modules** (`runtime/wasm`, `runtime/container`)
    - Current: 48-51%
    - Target: 80%+
    - Tests Needed: Engine integration, error paths

---

## Priority 2: Remaining CLI Sleep Elimination

### Current State
- **CLI tests with sleeps**: ~15-20 files
- **Total sleeps**: ~100 (mostly in monitoring/stress tests)
- **Impact**: Low (these are intentional for burst/stress testing)

### Strategy
- Keep intentional timeouts for:
  - Burst tests (need real timing)
  - Stress tests (need sustained load)
  - Chaos tests (need unpredictable timing)
- Replace coordination sleeps with `Barrier` or `Notify`

---

## Priority 3: Zero-Copy Optimization (Phase 1)

### Target: 15-20% Performance Improvement

#### Quick Wins (1-2 days)
1. **Function Signatures**: `&str` instead of `String` for read-only params
2. **String Literals**: References instead of allocations
3. **Template Constants**: `const` and `&'static str`

#### Medium Wins (3-5 days)
4. **Borrowed Returns**: Return `&str` where possible
5. **Cow<str>**: For sometimes-owned strings
6. **HashMap Key Optimization**: Use `&str` keys with lifetimes

#### Advanced (1-2 weeks)
7. **Arc<str>**: Shared string ownership
8. **String Interning**: Common strings cached
9. **Slice References**: Array operations without cloning

### Measurement
- Baseline: Current benchmark results
- Target: 15-20% reduction in allocations
- Metric: Memory usage and execution time

---

## Recommended Execution Order

### Week 1: Coverage Sprint
**Goal**: 60% → 70% (+10%)

**Day 1-2**: CLI Executor Tests
- Test execution lifecycle
- Error handling paths
- Resource management

**Day 3-4**: Core Ecosystem Tests
- Service discovery
- Health checks
- Coordination logic

**Day 5**: Server WebSocket Tests
- Connection management
- Message routing
- Error recovery

**Expected**: +5,000 covered lines, +50 tests

### Week 2: Distributed & API Coverage
**Goal**: 70% → 80% (+10%)

**Day 1-3**: Distributed Module
- Federation scenarios
- Fault tolerance
- State recovery
- Worker management

**Day 4-5**: API Layer
- Middleware chains
- Rate limiting
- Error paths

**Expected**: +5,000 covered lines, +60 tests

### Week 3: Runtime & Config Coverage
**Goal**: 80% → 85% (+5%)

**Day 1-2**: Runtime Engines
- WASM edge cases
- Container scenarios
- Native execution paths

**Day 3-5**: Zero-Config & Network
- Auto-discovery
- Traffic management
- Service mesh

**Expected**: +2,500 covered lines, +40 tests

### Week 4: Edge Cases & Polish
**Goal**: 85% → 90% (+5%)

**Day 1-3**: Error Paths
- All error handling
- Timeout scenarios
- Resource exhaustion

**Day 4-5**: Integration Tests
- Full E2E scenarios
- Multi-component interaction

**Expected**: +2,500 covered lines, +30 tests

---

## Success Metrics

### Coverage Milestones
- ✅ **60%** - ACHIEVED (Dec 1, 2025)
- 🎯 **70%** - Week 1 target
- 🎯 **80%** - Week 2 target
- 🎯 **90%** - Month 1 target (Dec 31, 2025)

### Test Growth
- Current: 1,727 tests
- Target: ~1,900-2,000 tests (+173-273)
- Growth: ~10% per week

### Quality Metrics
- Pass Rate: 100% (maintain)
- Flakiness: 0 (maintain)
- Speed: <1min full suite (maintain)

---

## Resources Needed

### Documentation
- Coverage report HTML: `target/llvm-cov/html/index.html`
- Test patterns: Established in this session
- Mock patterns: Documented in session report

### Tools
- `cargo llvm-cov` - coverage measurement
- `tokio::sync::Notify` - event coordination
- `tokio::sync::Barrier` - multi-task sync
- `serial_test` - test isolation

### Reference Tests
- `tests/e2e/full_system_tests.rs` - Event-driven patterns
- `crates/server/tests/background_real_tests.rs` - Modern async
- `crates/cli/tests/integration_month2_tests.rs` - Mock Drop patterns

---

## Risk Mitigation

### Potential Blockers
1. **Disabled Test Files** - May need infrastructure updates
   - `chaos_testing_suite.rs.DISABLED`
   - `ecosystem_simple_concurrent_tests.rs.DISABLED`

2. **Complex Integration** - Some modules have external dependencies
   - Solution: Use mocks and stubs

3. **Time Constraints** - 90% is ambitious
   - Fallback: 80% is still world-class (TOP 5%)

### Contingency Plans
- If Week 1 < 70%: Extend to 5 weeks
- If stuck on module: Skip and return later
- If tests flaky: Add `#[serial]` or increase timeouts

---

## Bottom Line

**This is a comprehensive, achievable plan** to take ToadStool from **60% → 90% coverage** in **4 weeks**.

The modernization work completed today provides the **foundation** (event-driven patterns, proper mocks, test isolation) for **rapid coverage expansion**.

**Next Step**: Start with CLI Executor tests (highest impact, 938 lines, 0% coverage)

---

*Created: Dec 1, 2025*  
*Based on: Comprehensive audit and coverage analysis*

