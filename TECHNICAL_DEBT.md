# 🔍 ToadStool Technical Debt Tracking

**Created**: December 21, 2025  
**Last Updated**: December 21, 2025  
**Status**: Active tracking of all technical debt

---

## 📊 Summary

| Category | Count | Priority | Estimated Effort |
|----------|-------|----------|------------------|
| **Production .unwrap()** | 947 | 🔴 HIGH | 2-3 weeks |
| **Unsafe without SAFETY** | 23 blocks | 🟡 MEDIUM | 1 day |
| **TODO Comments** | 104 | 🟢 LOW | Tracked |
| **Memory .clone()** | 2,459 | 🟡 MEDIUM | 3-4 weeks |
| **Test Coverage Gap** | 45% → 90% | 🔴 CRITICAL | 6-8 months |
| **Runtime Extensions** | 3 incomplete | 🟢 LOW | 3-4 weeks |

---

## 🔴 CRITICAL: Error Handling

### Production .unwrap() Audit

**Total**: 947 instances in production code (30% of 3,156 total)

**Status**: 🔴 IN PROGRESS

**High-Priority Files**:
```
crates/core/toadstool/     - 300+ unwraps
crates/distributed/        - 200+ unwraps
crates/cli/                - 150+ unwraps
crates/server/             - 45 unwraps
crates/core/config/        - 29 unwraps
crates/core/common/        - 11 unwraps
```

**Action Plan**:
1. Phase 1: Audit core/common (11 unwraps) - 1 day
2. Phase 2: Audit core/config (29 unwraps) - 1 day
3. Phase 3: Audit server (45 unwraps) - 2 days
4. Phase 4: Audit distributed (200+ unwraps) - 1 week
5. Phase 5: Audit cli (150+ unwraps) - 4 days
6. Phase 6: Audit core/toadstool (300+ unwraps) - 1 week

**Evolution Pattern**:
```rust
// OLD: Panic on error
let value = some_operation().unwrap();

// NEW: Proper error handling
let value = some_operation()
    .context("Failed to perform operation")?;

// OR: Explicit panic with context
let value = some_operation()
    .expect("Operation guaranteed to succeed: reason");
```

**Target Date**: January 15, 2026

---

## 🟡 MEDIUM: Memory Safety Documentation

### Unsafe Blocks Without SAFETY Comments

**Total**: 23 production blocks (60% of 38 total production unsafe)

**Status**: 🟡 PENDING

**Files Needing Documentation**:
```
crates/runtime/gpu/src/memory/pinned.rs (4 blocks need docs)
crates/runtime/gpu/src/backends/cuda_impl.rs (1 block needs docs)
crates/runtime/gpu/src/backends/opencl_impl.rs (1 block needs docs)
crates/runtime/wasm/src/cache_zero_unsafe.rs (5 blocks need docs)
crates/runtime/wasm/src/cache.rs (2 blocks need docs)
crates/runtime/wasm/src/cache_safe.rs (2 blocks need docs)
crates/runtime/wasm/src/lib.rs (8 blocks need docs)
```

**Required Pattern**:
```rust
// SAFETY: <Explain why this is safe>
// - Invariant 1: <condition that must hold>
// - Invariant 2: <condition that must hold>
// - Justification: <why the invariants hold>
unsafe {
    // ... unsafe operations
}
```

**Effort**: 1 day  
**Target Date**: December 28, 2025

---

## 🟡 MEDIUM: Memory Optimization

### Unnecessary .clone() Calls

**Total**: 2,459 instances  
**Unnecessary**: ~700-1,000 (30-40%)

**Status**: 🟡 PENDING

**Hot Paths** (Priority Order):
```
1. crates/core/toadstool/     - 300+ clones
2. crates/distributed/        - 200+ clones  
3. crates/cli/                - 250+ clones
4. crates/server/             - 150+ clones
```

**Evolution Patterns**:
```rust
// Pattern 1: Unnecessary string clone
// OLD:
fn process(name: String) { } // Requires ownership
let result = process(service.name.clone());

// NEW:
fn process(name: &str) { } // Just needs to read
let result = process(&service.name);

// Pattern 2: Config passing
// OLD:
execute_job(config.clone()) // Allocates every time

// NEW:
execute_job(&config) // Zero-copy reference

// Pattern 3: Shared ownership
// OLD:
let data1 = expensive_data.clone();
let data2 = expensive_data.clone();

// NEW:
let data = Arc::new(expensive_data);
let data1 = Arc::clone(&data); // Just increments ref count
let data2 = Arc::clone(&data);

// Pattern 4: Conditional ownership
// OLD:
fn maybe_owned(s: String) -> String { /* might modify */ }

// NEW:
fn maybe_owned(s: Cow<'_, str>) -> Cow<'_, str> { /* zero-copy when unmodified */ }
```

**Effort**: 3-4 weeks for hot paths  
**Target Date**: February 15, 2026

---

## 🔴 CRITICAL: Test Coverage

### Gap: 45% → 90%

**Status**: 🔴 IN PROGRESS

**Coverage Breakdown** (Estimated):
```
crates/core/toadstool/     - 50% coverage
crates/distributed/        - 40% coverage
crates/cli/                - 35% coverage
crates/runtime/gpu/        - 60% coverage
crates/runtime/wasm/       - 70% coverage
crates/runtime/native/     - 55% coverage
```

**Phase 1: Critical Paths** (Target: 70%, +25%)
- 400-500 unit tests
- Focus: execution, coordination, resource management
- Effort: 2-3 months

**Phase 2: Integration** (Target: 80%, +10%)
- 200-300 integration tests
- Focus: primal communication, discovery, config
- Effort: 2 months

**Phase 3: E2E & Chaos** (Target: 90%, +10%)
- 100-150 E2E tests
- 100-150 chaos/fault injection tests
- Focus: failure modes, recovery, edge cases
- Effort: 2-3 months

**Total Effort**: 6-8 months  
**Target Date**: August 2026

---

## 🟢 LOW: TODO Comments

### 104 TODO Comments (None Blocking)

**Status**: 🟢 TRACKED

**By Priority**:

**Future Enhancements** (86 comments, 82%):
- GPU tower selection optimization
- Remote execution improvements
- Configuration expansions
- Performance tuning opportunities

**Missing Integrations** (18 comments, 17%):
- mDNS integration to discovery system
- Songbird discovery client wiring
- BearDog discovery integration

**Non-Critical** (0 blocking):
- All TODOs are future work
- No production blockers

**Action**: Document and track, no immediate work needed

---

## 🟢 LOW: Runtime Extensions

### Incomplete Runtimes (Documented Roadmap)

**Status**: 🟢 PLANNED

**Python Runtime** ⏳
- Status: Not integrated into UniversalJobType
- Effort: 4-6 hours
- Priority: LOW (documented roadmap)
- Target: Q1 2026

**Container Runtime** ⏳
- Status: Not integrated into UniversalJobType
- Effort: 6-8 hours
- Priority: LOW (documented roadmap)
- Target: Q1 2026

**GPU Runtime** ⏳
- Status: Partial implementation, not in UniversalJobType
- Effort: 8-12 hours
- Priority: LOW (documented roadmap)
- Target: Q1 2026

---

## 🔄 EVOLUTION GOALS

### Hardcoding → Capability Discovery

**Status**: 🟡 IN PROGRESS

**Current State**:
- Configuration-based defaults ✅
- Environment variable overrides ✅
- Runtime discovery system exists ✅
- **Gap**: Not fully wired throughout codebase

**Target State**:
```rust
// OLD: Hardcoded service addresses
let songbird_url = "http://localhost:8080";
let beardog_url = "http://localhost:8081";

// NEW: Capability-based discovery
let orchestrator = discovery.find_capability("orchestration").await?;
let security = discovery.find_capability("security").await?;
```

**Files to Evolve**:
- `crates/distributed/src/beardog_integration/client.rs` - Wire discovery
- `crates/core/config/src/lib.rs` - Use discovery for defaults
- `crates/cli/src/executor/` - Discovery-based execution

**Effort**: 1-2 weeks  
**Target Date**: January 15, 2026

### Mocks → Real Implementations

**Status**: ✅ EXCELLENT (95% isolated to tests)

**Production Mocks** (~5%, 50 instances):
- All properly feature-gated: `#[cfg(feature = "mock-networking")]`
- Used only for development/testing
- No production path uses mocks

**Action**: Monitor, maintain isolation

### Unsafe → Safe + Fast

**Status**: 🟡 ONGOING

**Current Unsafe** (56 blocks, 38 production):
- GPU FFI: 11 blocks (performance-critical, may need to stay)
- WASM serialization: 15 blocks (Wasmtime API requirement)
- Memory pinning: 7 blocks (GPU driver interface)
- Other: 5 blocks

**Evolution Strategy**:
1. **Document all** - Add SAFETY comments (1 day)
2. **Analyze each** - Can it be safe? (2 days)
3. **Evolve where possible** - Use safe alternatives (1 week)
4. **Accept FFI reality** - Some unsafe required for GPU/WASM

**Target**: Minimize unsafe, maximize safety docs  
**Effort**: 1-2 weeks  
**Target Date**: January 30, 2026

---

## 📊 Progress Tracking

### Weekly Updates

**Week of Dec 21, 2025**:
- ✅ Comprehensive audit completed
- ✅ TECHNICAL_DEBT.md created
- ✅ Clippy errors fixed (3 instances)
- 🔄 Unwrap audit started (phase 1)

**Week of Dec 28, 2025** (Planned):
- ⬜ SAFETY comments added (38 blocks)
- ⬜ Unwrap audit: core/common complete (11 unwraps)
- ⬜ Unwrap audit: core/config complete (29 unwraps)

**Week of Jan 4, 2026** (Planned):
- ⬜ Unwrap audit: server complete (45 unwraps)
- ⬜ Wire capability discovery to CLI
- ⬜ Start test coverage push (first 50 tests)

---

## 🎯 Milestones

### Q4 2025 (Dec 21 - Dec 31)
- [x] Comprehensive audit
- [x] Clippy errors fixed
- [x] Technical debt documented
- [ ] SAFETY comments added
- [ ] Unwrap audit phase 1 (40 unwraps)

### Q1 2026 (Jan - Mar)
- [ ] Unwrap audit complete (947 unwraps)
- [ ] Capability discovery wired throughout
- [ ] Test coverage +200 tests (→ 60%)
- [ ] Clone optimization phase 1 (hot paths)
- [ ] Runtime extensions complete

### Q2 2026 (Apr - Jun)
- [ ] Test coverage +400 tests (→ 80%)
- [ ] Clone optimization complete
- [ ] Unsafe evolution complete
- [ ] Performance profiling done

### Q3 2026 (Jul - Sep)
- [ ] Test coverage +200 tests (→ 90%)
- [ ] Chaos testing comprehensive
- [ ] All milestones complete
- [ ] **Production enterprise-ready**

---

## 📝 Notes

- All debt is tracked and has timelines
- No blocking issues prevent current deployment
- Focus is on evolving from MVP to enterprise
- Honest assessment drives quality
- Progress tracked weekly

**Next Review**: December 28, 2025


