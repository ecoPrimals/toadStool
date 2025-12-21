# 🚀 Deep Evolution Execution Plan
## Modern Idiomatic Rust - December 5, 2025 Evening

**Status**: 🔄 IN PROGRESS  
**Goal**: Evolve to modern, idiomatic, capability-based, safe Rust  
**Approach**: Systematic, high-impact improvements  

---

## 📋 EXECUTION PRIORITIES

### **Phase 1: Hardcoding Evolution** (HIGH IMPACT) 🔥
**Timeline**: 2-3 hours  
**Impact**: Architecture modernization

#### 1.1 Templates & Dependencies ✅ STARTED
- [x] `specialized_templates.rs` line 97: ✅ Evolved to capability-based dependencies
- [ ] `network_config/configurator/core.rs`: Remove "primal.local" domain hardcoding
- [ ] All template files: Convert primal names to capability references

**Pattern**:
```rust
// ❌ OLD: Hardcoded primal names
dependencies: vec!["beardog".to_string(), "nestgate".to_string()]

// ✅ NEW: Capability-based
dependencies: vec![
    "capability:pki".to_string(),
    "capability:storage".to_string(),
]
```

#### 1.2 Discovery System Evolution
- [ ] `cli/src/zero_config/discovery.rs`: Verify all discovery is capability-based
- [ ] `cli/src/ecosystem/`: Remove any remaining primal name hardcoding
- [ ] Deprecate `primal_capabilities.rs` legacy functions (already deprecated, enforce usage)

#### 1.3 Configuration Evolution
- [ ] `config/src/defaults.rs`: Document all hardcoded ports as "fallback defaults only"
- [ ] Ensure all config loaders check environment first, then discovery, then defaults
- [ ] Add capability-based service resolution everywhere

**Files to Evolve**:
1. ✅ `crates/cli/src/templates/specialized_templates.rs`
2. `crates/cli/src/templates/basic_templates.rs`
3. `crates/cli/src/network_config/configurator/core.rs`
4. `crates/cli/src/ecosystem/integrator_impl.rs`
5. `crates/cli/src/zero_config/discovery.rs`

---

### **Phase 2: Unsafe Code Elimination** (HIGH IMPACT) 🔥
**Timeline**: 1-2 hours  
**Impact**: Safety + performance

#### 2.1 WASM Cache Migration
**Current**: `cache.rs` uses unsafe `Module::deserialize()` (3 instances)  
**Target**: Use `cache_zero_unsafe.rs` pattern everywhere

**Action Plan**:
1. [x] Verify `cache_zero_unsafe.rs` exists and is complete ✅
2. [ ] Benchmark both implementations (verify zero-unsafe is competitive)
3. [ ] Switch default to `ZeroUnsafeModuleCache`
4. [ ] Add feature flag `unsafe-fast-cache` for ultra-performance needs
5. [ ] Document tradeoffs clearly

**Code Changes**:
```rust
// In crates/runtime/wasm/src/lib.rs
#[cfg(not(feature = "unsafe-fast-cache"))]
pub use cache_zero_unsafe::ZeroUnsafeModuleCache as ModuleCache;

#[cfg(feature = "unsafe-fast-cache")]
pub use cache::ModuleCache;
```

#### 2.2 Document Remaining Unsafe
- All 17 unsafe instances should have:
  - `// SAFETY:` comment explaining why it's safe
  - Link to audit/decision document
  - Alternative safe approach (if exists)

**Goal**: 0 unsafe by default, unsafe only behind feature flag

---

### **Phase 3: Large File Refactoring** (MEDIUM IMPACT) 📝
**Timeline**: 4-6 hours  
**Impact**: Maintainability

#### 3.1 Smart Refactoring Strategy

**DON'T**: Just split files arbitrarily  
**DO**: Extract logical modules with clear responsibilities

**Files to Refactor** (8 files >1000 LOC):
1. `biomeos_integration_tests.rs` (1,424 lines) → Extract modules:
   - `auth_integration_tests.rs`
   - `storage_integration_tests.rs`
   - `agent_integration_tests.rs`
   - `common_test_fixtures.rs` (shared setup)

2. `comprehensive_policy_tests.rs` (1,397 lines) → Extract:
   - `policy_evaluation_tests.rs`
   - `policy_enforcement_tests.rs`
   - `policy_management_tests.rs`
   - Common test utilities

3. `comprehensive_sandbox_tests.rs` (1,188 lines) → Extract:
   - `sandbox_isolation_tests.rs`
   - `sandbox_security_tests.rs`
   - `sandbox_resource_tests.rs`

4. `runtime_comprehensive_tests.rs` (1,129 lines) → Extract by engine type
5. `executor_comprehensive_lifecycle_tests.rs` (1,119 lines) → Extract by lifecycle phase
6. `comprehensive_client_tests_expansion.rs` (1,093 lines) → Extract by client type
7. `integration_test.rs` (1,072 lines) → Extract by integration type
8. `network_config_tests.rs` (1,032 lines) → Extract by config type

**Pattern**:
```
tests/
├── module_name/
│   ├── mod.rs              # Re-exports + shared fixtures
│   ├── feature_a_tests.rs  # Focused test suite A
│   ├── feature_b_tests.rs  # Focused test suite B
│   └── fixtures.rs         # Common test setup
```

---

### **Phase 4: Zero-Copy Optimization** (HIGH IMPACT) ⚡
**Timeline**: 6-8 hours  
**Impact**: Performance (10-20% gains)

#### 4.1 String Clone Elimination (High Impact)

**Hot Paths to Optimize**:
1. Configuration loading/access
2. Service discovery results
3. Capability matching
4. Error message construction

**Strategies**:
```rust
// ❌ OLD: Unnecessary clone
fn process_config(config: Config) {
    let name = config.name.clone(); // Unnecessary!
    use_name(&name);
}

// ✅ NEW: Borrow
fn process_config(config: &Config) {
    use_name(&config.name); // Direct reference
}

// ❌ OLD: String allocation
format!("Error: {}", msg.clone())

// ✅ NEW: Use Cow or direct reference
Cow::Borrowed(msg)
```

#### 4.2 Collection Clone Elimination

**Target**: 2,812 clone() calls → ~1,500 (46% reduction target)

**Audit Process**:
1. Profile to find hot paths (use `cargo flamegraph`)
2. Focus on top 20% of hot clones (80% of impact)
3. Replace with:
   - References (`&T`)
   - `Arc<T>` for shared ownership
   - `Cow<'a, T>` for clone-on-write
   - Iterators instead of collected vectors

---

### **Phase 5: Test Coverage Expansion** (CRITICAL) 🎯
**Timeline**: 8-12 hours  
**Impact**: Quality assurance (52% → 90%)

#### 5.1 Critical Modules (Week 1 Priority)

**Target Modules** (0% coverage):
1. `auto_config` (11 files, 0%)
2. `client/core` (0%)
3. `config_utils` (1.87%)

**Strategy**:
- Write property-based tests (use `proptest`)
- Focus on error paths (often untested)
- Use table-driven tests for variations
- Mock external dependencies properly

**Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Property-based test
    proptest! {
        #[test]
        fn parse_always_succeeds_or_errors(input in ".*") {
            let result = parse_config(&input);
            // Should never panic, only Ok or Err
            assert!(result.is_ok() || result.is_err());
        }
    }
    
    // Table-driven test
    #[test]
    fn config_validation() {
        let cases = vec![
            ("valid", Ok(())),
            ("invalid", Err(ConfigError::Invalid)),
        ];
        
        for (input, expected) in cases {
            assert_eq!(validate(input), expected);
        }
    }
}
```

---

### **Phase 6: Production Unwrap Elimination** (MEDIUM IMPACT) ⚠️
**Timeline**: 4-6 hours  
**Impact**: Robustness

#### 6.1 Unwrap Audit & Replacement

**Current**: ~80-150 production unwraps (3,635 total, mostly tests)

**Strategy**:
1. Find all production unwraps: `grep -r "unwrap()" crates/*/src/ --include="*.rs"`
2. Categorize:
   - **Critical path**: Replace immediately with `?` or `unwrap_or`
   - **Initialization**: Add proper error handling
   - **Test code**: Keep (tests should panic on unexpected errors)

**Patterns**:
```rust
// ❌ OLD: Unwrap (panics on error)
let config = load_config().unwrap();

// ✅ NEW: Propagate error
let config = load_config()?;

// ✅ NEW: Provide fallback
let config = load_config().unwrap_or_default();

// ✅ NEW: Context
let config = load_config()
    .map_err(|e| format!("Failed to load config: {}", e))?;
```

---

### **Phase 7: Mock Completion** (LOW IMPACT) ✅
**Timeline**: 2-3 hours  
**Impact**: Completeness

#### 7.1 Verify No Production Mocks

**Audit Result**: ✅ Already verified - zero production mocks

**Action**: Document completion, no work needed

---

### **Phase 8: Pedantic Lint Enablement** (MEDIUM IMPACT) 📐
**Timeline**: 4-6 hours  
**Impact**: Code quality

#### 8.1 Workspace-Wide Pedantic Lints

**Current**: `toadstool-common` is 100% pedantic-clean  
**Target**: All crates pedantic-clean

**Remaining**: ~150 issues across other crates

**Strategy**:
1. Enable pedantic per crate (not all at once)
2. Fix issues by category:
   - `doc_markdown`: 2-3 hours (add backticks)
   - `must_use_candidate`: 1-2 hours (add attributes)
   - `missing_errors_doc`: 1 hour (document errors)
   - Others: 1-2 hours

**Order**:
1. ✅ `toadstool-common` (done)
2. `toadstool-config` (next, ~30 issues)
3. `toadstool-core` (then, ~40 issues)
4. Remaining crates (lower priority)

---

## 📊 IMPACT ASSESSMENT

| Phase | Time | Impact | Priority | Status |
|-------|------|--------|----------|--------|
| **Hardcoding Evolution** | 2-3h | 🔥 HIGH | P0 | 🔄 Started |
| **Unsafe Elimination** | 1-2h | 🔥 HIGH | P0 | Pending |
| **Large File Refactor** | 4-6h | 📝 MEDIUM | P1 | Pending |
| **Zero-Copy Optimization** | 6-8h | ⚡ HIGH | P1 | Pending |
| **Test Coverage** | 8-12h | 🎯 CRITICAL | P0 | Pending |
| **Unwrap Elimination** | 4-6h | ⚠️ MEDIUM | P1 | Pending |
| **Mock Completion** | 0h | ✅ DONE | - | ✅ Complete |
| **Pedantic Lints** | 4-6h | 📐 MEDIUM | P2 | Partial |

**Total Estimated Time**: 29-43 hours  
**Phases this session**: 1, 2 (4-5 hours)  
**Carry forward**: Phases 3-8 (25-38 hours)

---

## 🎯 SUCCESS CRITERIA

### Phase 1: Hardcoding Evolution
- [ ] Zero hardcoded primal names in production code
- [ ] All dependencies use `capability:*` prefix
- [ ] All discovery is capability-based
- [ ] Legacy functions clearly deprecated

### Phase 2: Unsafe Elimination
- [ ] Zero unsafe in default build
- [ ] Unsafe behind `unsafe-fast-cache` feature flag
- [ ] All unsafe has SAFETY comments
- [ ] Benchmarks show <5% perf difference

### Phase 3: Large Files
- [ ] All files <1000 LOC
- [ ] Logical module organization
- [ ] Improved test discoverability
- [ ] Shared fixtures extracted

### Phase 4: Zero-Copy
- [ ] 2,812 → ~1,500 clones (46% reduction)
- [ ] Hot paths use references/Arc/Cow
- [ ] 10-20% performance improvement (measured)
- [ ] Zero functional changes

### Phase 5: Test Coverage
- [ ] Overall: 52% → 90%
- [ ] auto_config: 0% → 70%
- [ ] client/core: 0% → 70%
- [ ] config_utils: 1.87% → 70%

### Phase 6: Unwraps
- [ ] Production unwraps: ~80-150 → 0
- [ ] All errors properly propagated
- [ ] Graceful degradation where appropriate

### Phase 7: Mocks
- [x] ✅ Zero production mocks (already verified)

### Phase 8: Pedantic
- [ ] All crates pass pedantic lints
- [ ] ~150 issues → 0
- [ ] Quality-of-life improvements throughout

---

## 🔄 PROGRESS TRACKING

### Session 1 (Dec 5, 2025 Evening) - Current
**Time**: 2-3 hours  
**Focus**: Phases 1 & 2 (Hardcoding + Unsafe)

**Completed**:
- [x] Comprehensive audit report generated
- [x] Execution plan created
- [x] Started hardcoding evolution (1/5 files)

**Next**:
- [ ] Complete hardcoding evolution (4/5 files)
- [ ] Unsafe code elimination
- [ ] Benchmark safe vs unsafe cache

### Session 2 (Future)
**Focus**: Phases 3 & 4 (Files + Zero-Copy)

### Session 3 (Future)
**Focus**: Phases 5 & 6 (Tests + Unwraps)

### Session 4 (Future)
**Focus**: Phase 8 (Pedantic)

---

## 📚 PRINCIPLES

### Self-Knowledge Pattern
**Each primal knows only itself**
- No hardcoded knowledge of other primals
- Discovery through capabilities
- Runtime service resolution
- Adapter pattern for integration

### Modern Idiomatic Rust
- Zero-copy where possible
- Proper error propagation (`?` not `unwrap`)
- Safe by default (unsafe behind feature flags)
- Pedantic lints enabled
- Well-documented code

### Smart Refactoring
- Extract logical modules, not arbitrary splits
- Preserve test coverage
- Improve discoverability
- Shared fixtures properly organized

### Performance + Safety
- Fast AND safe (not fast OR safe)
- Profile before optimizing
- Benchmark all optimizations
- Zero-copy without zero-safety

---

## 🚀 LET'S EXECUTE!

**Current Status**: Phase 1.1 in progress  
**Next Action**: Complete template evolution  
**Goal**: Modern, capability-based, safe, fast Rust

---

*Generated: December 5, 2025 Evening*  
*Status: Living document, updated as we progress*

