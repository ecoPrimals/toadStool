# 📦 File Refactoring Plan

**Date**: November 12, 2025  
**Purpose**: Break down oversized files into manageable modules  
**Target**: Files exceeding 1000 lines (24 files identified)  
**Status**: Analysis complete, ready for systematic refactoring

---

## 📊 PROBLEM STATEMENT

### **Current State**:
- **24 files** exceed the 1000-line limit
- **Largest file**: 2,497 lines
- **Total excess**: ~15,000 lines over limit
- **Impact**: Reduced maintainability, longer compile times, harder code navigation

### **Target State**:
- **All files** ≤ 1000 lines
- **Clear module boundaries**
- **Logical code organization**
- **Improved discoverability**
- **Faster compile times**

---

## 🎯 TOP PRIORITY FILES (>1400 Lines)

### **Priority 1: Distributed Coordinator** (2,497 lines)
**File**: `crates/distributed/src/core/coordinator.rs`

**Current Structure Analysis**:
- Main coordinator logic: ~500 lines
- Job scheduling: ~400 lines
- Health monitoring: ~300 lines
- Failure recovery: ~350 lines
- Network coordination: ~400 lines
- Metrics/logging: ~300 lines
- Tests (internal): ~247 lines

**Refactoring Strategy**:
```
crates/distributed/src/core/
├── coordinator.rs (300 lines) - Main coordinator entry point
├── coordinator/
│   ├── mod.rs (50 lines) - Re-exports
│   ├── scheduling.rs (400 lines) - Job scheduling logic
│   ├── health.rs (300 lines) - Health monitoring
│   ├── recovery.rs (350 lines) - Failure recovery
│   ├── networking.rs (400 lines) - Network coordination
│   └── metrics.rs (300 lines) - Metrics and observability
└── tests/
    └── coordinator_tests.rs (247 lines) - Move tests here
```

**Benefits**:
- Clear separation of concerns
- Testable submodules
- Easier to navigate
- Faster incremental compilation

**Effort**: 6-8 hours

---

### **Priority 2: Universal Adapter** (1,823 lines)
**File**: `crates/core/toadstool/src/universal.rs`

**Current Structure Analysis**:
- Adapter trait definitions: ~200 lines
- Runtime adapters: ~500 lines
- Platform-specific logic: ~400 lines
- Conversion utilities: ~300 lines
- Error handling: ~200 lines
- Tests (internal): ~223 lines

**Refactoring Strategy**:
```
crates/core/toadstool/src/
├── universal.rs (200 lines) - Main trait definitions
├── universal/
│   ├── mod.rs (50 lines) - Re-exports
│   ├── adapters.rs (500 lines) - Runtime adapters
│   ├── platforms.rs (400 lines) - Platform-specific logic
│   ├── conversions.rs (300 lines) - Type conversions
│   └── errors.rs (200 lines) - Error types
└── tests/
    └── universal_tests/ (split existing tests)
```

**Benefits**:
- Cleaner trait hierarchy
- Easier to add new runtimes
- Better error documentation
- Improved testing structure

**Effort**: 5-7 hours

---

### **Priority 3: Ecosystem Integration** (1,642 lines)
**File**: `crates/core/toadstool/src/ecosystem.rs`

**Current Structure Analysis**:
- Ecosystem coordinator: ~300 lines
- Service discovery: ~350 lines
- Node management: ~350 lines
- Communication protocols: ~300 lines
- State management: ~250 lines
- Tests (internal): ~92 lines

**Refactoring Strategy**:
```
crates/core/toadstool/src/
├── ecosystem.rs (250 lines) - Main coordinator
├── ecosystem/
│   ├── mod.rs (50 lines) - Re-exports
│   ├── discovery.rs (350 lines) - Service discovery
│   ├── nodes.rs (350 lines) - Node management
│   ├── protocols.rs (300 lines) - Communication
│   └── state.rs (250 lines) - State management
└── tests/
    └── ecosystem_tests/ (split existing tests)
```

**Benefits**:
- Modular ecosystem components
- Easier to test discovery
- Clear protocol boundaries
- Better state isolation

**Effort**: 5-7 hours

---

### **Priority 4: Server Library** (1,556 lines)
**File**: `crates/server/src/lib.rs`

**Current Structure Analysis**:
- Server configuration: ~250 lines
- Request handlers: ~400 lines
- WebSocket logic: ~300 lines
- State management: ~250 lines
- Background tasks: ~200 lines
- Error handling: ~156 lines

**Refactoring Strategy**:
```
crates/server/src/
├── lib.rs (200 lines) - Public API
├── config.rs (250 lines) - Configuration (already exists, merge)
├── handlers/ (create)
│   ├── mod.rs (50 lines)
│   ├── http.rs (200 lines) - HTTP handlers
│   └── websocket.rs (300 lines) - WebSocket handlers
├── state.rs (250 lines) - State management
├── background.rs (200 lines) - Background tasks
└── errors.rs (156 lines) - Error types
```

**Benefits**:
- Clear handler organization
- Testable state management
- Isolated WebSocket logic
- Better error propagation

**Effort**: 4-6 hours

---

### **Priority 5: Security Sandbox Manager** (1,478 lines)
**File**: `crates/security/sandbox/src/manager.rs`

**Current Structure Analysis**:
- Sandbox manager: ~300 lines
- Process isolation: ~350 lines
- Resource limits: ~300 lines
- Security policies: ~250 lines
- Monitoring: ~200 lines
- Tests (internal): ~78 lines

**Refactoring Strategy**:
```
crates/security/sandbox/src/
├── manager.rs (300 lines) - Main manager
├── manager/
│   ├── mod.rs (50 lines) - Re-exports
│   ├── isolation.rs (350 lines) - Process isolation
│   ├── limits.rs (300 lines) - Resource limits
│   ├── policies.rs (250 lines) - Security policies
│   └── monitoring.rs (200 lines) - Security monitoring
└── tests/
    └── manager_tests.rs (78 lines) - Move tests here
```

**Benefits**:
- Security concerns separated
- Easier to audit
- Clear policy boundaries
- Testable isolation

**Effort**: 4-6 hours

---

## 📋 MEDIUM PRIORITY FILES (1100-1400 Lines)

### **Files to Refactor** (19 files):
1. `crates/cli/src/executor/executor_impl.rs` (1,389 lines)
2. `crates/distributed/src/hosting/recursive.rs` (1,367 lines)
3. `crates/integration/protocols/src/lib.rs` (1,342 lines)
4. `crates/runtime/edge/src/lib.rs` (1,298 lines)
5. `crates/runtime/specialty/src/lib.rs` (1,287 lines)
6. `crates/core/toadstool/src/byob/byob_impl.rs` (1,256 lines)
7. `crates/client/src/lib.rs` (1,234 lines)
8. `crates/distributed/src/universal/scheduler.rs` (1,212 lines)
9. `crates/cli/src/ecosystem/integrator_impl.rs` (1,198 lines)
10. `crates/runtime/container/src/lib.rs` (1,187 lines)
... (9 more files in this range)

**Strategy**: Similar modular breakdown for each
**Effort Per File**: 3-5 hours
**Total Effort**: 57-95 hours (7-12 days)

---

## 🔧 REFACTORING METHODOLOGY

### **Step-by-Step Process**:

#### **Phase 1: Analysis** (30 min per file)
1. Read entire file
2. Identify logical sections
3. Map dependencies between sections
4. Identify public vs. private APIs
5. Plan module structure

#### **Phase 2: Setup** (30 min per file)
1. Create new module directory
2. Create `mod.rs` with re-exports
3. Create stub files for each submodule
4. Update parent module imports

#### **Phase 3: Migration** (2-4 hours per file)
1. Move sections one at a time
2. Update imports in moved code
3. Test after each section move
4. Maintain API compatibility
5. Update documentation

#### **Phase 4: Cleanup** (30 min per file)
1. Remove duplicate imports
2. Optimize module structure
3. Update visibility (pub/pub(crate))
4. Run `cargo fmt`
5. Run `cargo clippy`

#### **Phase 5: Validation** (30 min per file)
1. Run full test suite
2. Check compile times
3. Verify no API breakage
4. Update module docs
5. Commit changes

---

## 📐 MODULE STRUCTURE PATTERNS

### **Pattern 1: Feature Modules**
Used for: Coordinators, managers, complex systems

```
parent/
├── main.rs (public API, core logic)
└── main/
    ├── mod.rs (re-exports)
    ├── feature1.rs
    ├── feature2.rs
    └── feature3.rs
```

### **Pattern 2: Layer Modules**
Used for: Handlers, adapters, protocols

```
parent/
├── lib.rs (trait definitions)
└── implementations/
    ├── mod.rs
    ├── impl1.rs
    ├── impl2.rs
    └── common.rs
```

### **Pattern 3: Domain Modules**
Used for: Business logic, domain models

```
parent/
├── types.rs (data structures)
├── logic.rs (business logic)
├── errors.rs (error types)
└── utils.rs (utilities)
```

---

## ✅ REFACTORING CHECKLIST

For each file refactoring:

### **Before Starting**:
- [ ] Read entire file
- [ ] Document current structure
- [ ] Plan module breakdown
- [ ] Create branch (`git checkout -b refactor-<module>`)
- [ ] Run tests to establish baseline

### **During Refactoring**:
- [ ] Create module structure
- [ ] Move code incrementally
- [ ] Update imports after each move
- [ ] Maintain API compatibility
- [ ] Run tests after each section
- [ ] Keep commits small and focused

### **After Refactoring**:
- [ ] All tests passing
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Module docs written
- [ ] Exports clearly documented
- [ ] Compile time measured (should improve)
- [ ] PR created with clear description

---

## 📊 EFFORT ESTIMATION

### **Priority 1-5 Files** (Top 5 largest):
- **Analysis**: 2.5 hours
- **Setup**: 2.5 hours
- **Migration**: 15-20 hours
- **Cleanup**: 2.5 hours
- **Validation**: 2.5 hours
- **Total**: 25-30 hours (3-4 days)

### **Medium Priority Files** (19 files, 1100-1400 lines):
- **Per File**: 3-5 hours
- **Total**: 57-95 hours (7-12 days)

### **Overall Project**:
- **Total Files**: 24
- **Total Effort**: 82-125 hours (10-16 days)
- **Recommended Timeline**: 3-4 weeks (part-time)

---

## 🎯 RECOMMENDED APPROACH

### **Option A: Systematic (Recommended)**
Tackle files in priority order, one at a time:
- **Week 1**: Priority 1-2 (coordinator, universal)
- **Week 2**: Priority 3-5 (ecosystem, server, sandbox)
- **Week 3**: Medium priority batch 1 (5 files)
- **Week 4**: Medium priority batch 2 (remaining files)

**Pros**: Steady progress, lower risk, easier to review  
**Cons**: Takes full 4 weeks

### **Option B: Parallel**
Split work across multiple developers:
- **Dev 1**: Priority 1-2
- **Dev 2**: Priority 3-4
- **Dev 3**: Priority 5 + medium priority

**Pros**: Faster completion (1-2 weeks)  
**Cons**: Merge conflicts, coordination overhead

### **Option C: Critical Path**
Focus only on Priority 1-5 files, defer medium priority:
- **Immediate**: Refactor top 5 files
- **Later**: Address medium priority as needed

**Pros**: Quick impact, addresses worst offenders  
**Cons**: Leaves 19 files over limit

---

## 🛡️ RISK MITIGATION

### **Risk 1: Breaking API Changes**
**Mitigation**:
- Keep public APIs in main file
- Use `pub use` re-exports
- Run integration tests frequently
- Document any changes

### **Risk 2: Import Cycles**
**Mitigation**:
- Plan dependency graph first
- Use trait objects where needed
- Consider `pub(crate)` visibility
- Extract shared types to separate module

### **Risk 3: Test Breakage**
**Mitigation**:
- Run tests after each move
- Update test imports immediately
- Keep test structure parallel to code
- Use `#[cfg(test)] mod tests`

### **Risk 4: Merge Conflicts**
**Mitigation**:
- Small, focused PRs
- Regular rebasing
- Clear communication
- Lock files during active refactoring

---

## 📚 EXAMPLE REFACTORING

### **Before**: `coordinator.rs` (2,497 lines)

```rust
// crates/distributed/src/core/coordinator.rs

pub struct Coordinator { /* ... */ }

impl Coordinator {
    // Main coordination logic (500 lines)
    pub fn new() -> Self { /* ... */ }
    pub fn coordinate() { /* ... */ }
    
    // Job scheduling (400 lines)
    fn schedule_job() { /* ... */ }
    fn assign_worker() { /* ... */ }
    
    // Health monitoring (300 lines)
    fn check_health() { /* ... */ }
    fn handle_failure() { /* ... */ }
    
    // ... (1,297 more lines)
}
```

### **After**: Modular structure

```rust
// crates/distributed/src/core/coordinator.rs (300 lines)

mod scheduling;
mod health;
mod recovery;
mod networking;
mod metrics;

pub use self::scheduling::Scheduler;
pub use self::health::HealthMonitor;
pub use self::recovery::RecoveryManager;

pub struct Coordinator {
    scheduler: Scheduler,
    health: HealthMonitor,
    recovery: RecoveryManager,
    // ... other components
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            health: HealthMonitor::new(),
            recovery: RecoveryManager::new(),
        }
    }
    
    pub fn coordinate(&self) -> Result<()> {
        self.scheduler.schedule()?;
        self.health.check()?;
        Ok(())
    }
}

// crates/distributed/src/core/coordinator/scheduling.rs (400 lines)
pub struct Scheduler { /* ... */ }
impl Scheduler {
    pub fn new() -> Self { /* ... */ }
    pub fn schedule(&self) -> Result<()> { /* ... */ }
}

// crates/distributed/src/core/coordinator/health.rs (300 lines)
pub struct HealthMonitor { /* ... */ }
// ... and so on
```

**Result**:
- ✅ Clear module boundaries
- ✅ Each file < 400 lines
- ✅ Testable components
- ✅ Faster compilation
- ✅ Better discoverability

---

## 🚀 QUICK START

### **To Begin Refactoring**:

```bash
# 1. Choose a file (start with Priority 1)
FILE="crates/distributed/src/core/coordinator.rs"

# 2. Create branch
git checkout -b refactor-coordinator

# 3. Create module directory
mkdir -p crates/distributed/src/core/coordinator

# 4. Create mod.rs
touch crates/distributed/src/core/coordinator/mod.rs

# 5. Create submodule files
touch crates/distributed/src/core/coordinator/scheduling.rs
touch crates/distributed/src/core/coordinator/health.rs
touch crates/distributed/src/core/coordinator/recovery.rs

# 6. Start moving code (one section at a time)
# Edit files...

# 7. Test after each move
cargo test --package distributed

# 8. Commit incrementally
git add .
git commit -m "refactor(distributed): extract scheduling logic"

# 9. Continue until complete
# ...

# 10. Final validation
cargo test --workspace
cargo clippy --all-targets
cargo fmt --all

# 11. Create PR
git push origin refactor-coordinator
```

---

## 📞 NEXT STEPS

1. **Review this plan** with team
2. **Prioritize files** (confirm Priority 1-5)
3. **Assign ownership** (who refactors what)
4. **Set timeline** (3-4 weeks recommended)
5. **Begin with Priority 1** (coordinator.rs)
6. **Track progress** (update STATUS.md)

---

## 📈 SUCCESS METRICS

### **After Refactoring**:
- ✅ All files ≤ 1000 lines
- ✅ All tests passing
- ✅ Zero clippy warnings
- ✅ Improved compile times (measure!)
- ✅ Clear module documentation
- ✅ No API breakage
- ✅ Code more navigable
- ✅ Team satisfaction improved

---

**Status**: ✅ ANALYSIS COMPLETE - READY FOR EXECUTION  
**Priority**: HIGH (code maintainability)  
**Effort**: 82-125 hours (10-16 days)  
**Risk**: LOW (clear plan, incremental approach)  
**Impact**: HIGH (code quality, team velocity)

---

*End of File Refactoring Plan*

