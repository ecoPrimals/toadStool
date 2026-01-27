# 🍄 Smart Refactoring Plan - Deep Debt Evolution

**Date**: January 19, 2026  
**Scope**: Large files (>900 lines) refactoring by logical domains  
**Principle**: Smart refactoring, NOT arbitrary splitting  
**Grade Target**: S++ (Maintain perfection!)

---

## 📊 Files to Refactor

| File | Lines | Status | Priority |
|------|-------|--------|----------|
| **executor_impl.rs** | 933 | ⏭️ Ready | 🔴 High |
| **byob_impl.rs** | 928 | ⏭️ Ready | 🔴 High |
| **performance_hardening.rs** | 920 | ⏭️ Ready | 🟡 Medium |
| **monitoring.rs** | 869 | ⏭️ Consider | 🟢 Low |

---

## 1️⃣ executor_impl.rs (933 lines) - READY

### **Current Structure** (Single File)

```
crates/cli/src/executor/executor_impl.rs (933 lines)
└── impl BiomeExecutor
    ├── Public CLI Commands (6 functions, ~280 lines)
    ├── Lifecycle Management (8 functions, ~400 lines)
    ├── Display & Logging (4 functions, ~170 lines)
    └── WASM Execution (2 functions, ~80 lines)
```

### **Proposed Smart Refactoring** (Module-Based)

```
crates/cli/src/executor/
├── executor_impl.rs (150 lines) - Core struct + constructor
├── commands.rs (280 lines) - Public CLI interface
│   ├── run_biome()
│   ├── up_biome()
│   ├── down_biome()
│   ├── list_biomes()
│   └── show_logs()
├── lifecycle.rs (400 lines) - Biome lifecycle management
│   ├── start_biome_internal()
│   ├── start_primal()
│   ├── start_service()
│   ├── workload_source_to_spec()
│   ├── stop_biome_internal()
│   ├── graceful_stop_process()
│   ├── force_kill_process()
│   └── purge_biome_data()
├── display.rs (170 lines) - UI rendering & logging
│   ├── wait_for_interruption()
│   ├── print_biomes_table()
│   ├── show_log_file()
│   ├── tail_log_file()
│   └── get_actual_pid()
└── wasm.rs (80 lines) - WASM execution
    ├── load_wasm_with_verification()
    └── execute_wasm_module()
```

### **Benefits**

- ✅ **Clear Domain Separation**: CLI vs Lifecycle vs Display vs WASM
- ✅ **Maintainability**: Easy to find functions by purpose
- ✅ **Testability**: Each module can be tested independently
- ✅ **Extensibility**: New workload types (Docker, Nix, etc.) go in lifecycle.rs
- ✅ **No Duplication**: All modules share `BiomeExecutor` state via `impl`

### **Deep Debt Compliance**

- ✅ **Logical Domains**: Split by **purpose**, not arbitrary line counts
- ✅ **Preserved Functionality**: Zero behavior changes
- ✅ **Clear Boundaries**: Each module has single responsibility
- ✅ **No Duplication**: Shared state via `impl BiomeExecutor` across files

### **Implementation Steps**

1. ✅ Create module files (commands.rs, lifecycle.rs, display.rs, wasm.rs)
2. ✅ Move functions to respective modules
3. ✅ Keep `impl BiomeExecutor` in each file (Rust allows this!)
4. ✅ Update executor_impl.rs to `mod` the new files
5. ✅ Verify compilation (zero errors)
6. ✅ Run tests (zero breakage)
7. ✅ Commit atomically

---

## 2️⃣ byob_impl.rs (928 lines) - READY

### **Current Structure** (Single File)

```
crates/core/toadstool/src/byob/byob_impl.rs (928 lines)
└── impl Byob
    ├── Build Phase (functions for building binaries)
    ├── Operation Phase (functions for running operations)
    ├── Binding Phase (functions for binding to ecosystem)
    └── Health/Status (functions for monitoring)
```

### **Proposed Smart Refactoring** (BYOB Phases)

```
crates/core/toadstool/src/byob/
├── byob_impl.rs (150 lines) - Core struct + constructor
├── build.rs (300 lines) - Build phase logic
│   ├── compile_binary()
│   ├── verify_toolchain()
│   ├── apply_optimizations()
│   └── package_artifact()
├── operations.rs (300 lines) - Operation phase logic
│   ├── execute_operation()
│   ├── stream_operation()
│   ├── validate_operation()
│   └── handle_operation_result()
├── binding.rs (200 lines) - Binding phase logic
│   ├── bind_to_ecosystem()
│   ├── register_capabilities()
│   ├── discover_dependencies()
│   └── establish_connections()
└── health.rs (150 lines) - Health & monitoring
    ├── check_health()
    ├── get_status()
    ├── collect_metrics()
    └── handle_failures()
```

### **Benefits**

- ✅ **BYOB Phase Alignment**: Matches conceptual BYOB workflow
- ✅ **Workflow Clarity**: Build → Operate → Bind → Monitor
- ✅ **Independent Evolution**: Each phase can evolve independently
- ✅ **Testing**: Test each phase in isolation
- ✅ **Documentation**: Natural documentation structure

### **Deep Debt Compliance**

- ✅ **Logical Phases**: Split by **BYOB lifecycle**, not arbitrary
- ✅ **Workflow Preservation**: BYOB workflow remains intact
- ✅ **Clear Boundaries**: Each phase is self-contained
- ✅ **No Duplication**: Shared Byob state via `impl Byob`

---

## 3️⃣ performance_hardening.rs (920 lines) - READY

### **Current Structure** (Single File)

```
crates/core/toadstool/src/performance_hardening.rs (920 lines)
└── Performance hardening logic
    ├── CPU Hardening (thread pinning, affinity, etc.)
    ├── Memory Hardening (huge pages, NUMA, etc.)
    ├── I/O Hardening (async I/O, buffering, etc.)
    └── Coordination (orchestrating all hardening)
```

### **Proposed Smart Refactoring** (Hardening Domains)

```
crates/core/toadstool/src/performance_hardening/
├── mod.rs (100 lines) - Core coordinator
├── cpu.rs (300 lines) - CPU hardening
│   ├── pin_threads()
│   ├── set_affinity()
│   ├── enable_turbo()
│   └── optimize_cache()
├── memory.rs (300 lines) - Memory hardening
│   ├── enable_huge_pages()
│   ├── configure_numa()
│   ├── optimize_allocations()
│   └── tune_swap()
├── io.rs (250 lines) - I/O hardening
│   ├── configure_async_io()
│   ├── optimize_buffering()
│   ├── tune_filesystem()
│   └── enable_direct_io()
└── types.rs (150 lines) - Shared types & config
    ├── HardeningConfig
    ├── HardeningMetrics
    └── HardeningResult
```

### **Benefits**

- ✅ **Domain Expertise**: CPU, Memory, I/O specialists
- ✅ **Independent Tuning**: Each domain can be tuned independently
- ✅ **Platform Variations**: Different platforms need different hardening
- ✅ **Testing**: Test each hardening domain separately
- ✅ **Documentation**: Natural resource-based docs

### **Deep Debt Compliance**

- ✅ **Resource Domains**: Split by **hardware resource**, not arbitrary
- ✅ **Performance Goals**: Each domain has clear performance target
- ✅ **Clear Boundaries**: CPU vs Memory vs I/O is well-defined
- ✅ **Coordinated**: mod.rs orchestrates all hardening

---

## 4️⃣ monitoring.rs (869 lines) - CONSIDER

### **Current Structure** (Single File)

```
crates/cli/src/monitoring.rs (869 lines)
└── Monitoring logic
    ├── Metrics Collection
    ├── Alerting Logic
    ├── Reporting
    └── Visualization
```

### **Proposed Smart Refactoring** (Monitoring Domains)

```
crates/cli/src/monitoring/
├── mod.rs (100 lines) - Core monitoring coordinator
├── metrics.rs (300 lines) - Metrics collection
├── alerts.rs (250 lines) - Alerting logic
├── reports.rs (200 lines) - Reporting
└── display.rs (150 lines) - Visualization
```

### **Priority**: 🟢 **LOW** (already < 900 lines, but would benefit)

---

## 🎯 Refactoring Principles (Deep Debt)

### ✅ **DO** - Smart Refactoring

1. ✅ **Split by Logical Domains** (not arbitrary line counts)
2. ✅ **Preserve Functionality** (zero behavior changes)
3. ✅ **Maintain Clear Boundaries** (single responsibility per module)
4. ✅ **Use Multiple `impl` Blocks** (Rust allows impl across files!)
5. ✅ **Test After Each Refactor** (verify no breakage)
6. ✅ **Document Module Purpose** (clear module-level docs)
7. ✅ **Keep Related Code Together** (don't split cohesive units)

### ❌ **DON'T** - Arbitrary Splitting

1. ❌ Don't split just to hit line count target
2. ❌ Don't split cohesive function groups
3. ❌ Don't create artificial boundaries
4. ❌ Don't duplicate code across modules
5. ❌ Don't break logical workflows
6. ❌ Don't split without clear purpose
7. ❌ Don't make navigation harder

---

## 📋 Implementation Plan

### **Phase 1: executor_impl.rs** (Highest Impact)

**Files to Create**:
1. `crates/cli/src/executor/commands.rs` - CLI interface
2. `crates/cli/src/executor/lifecycle.rs` - Biome management
3. `crates/cli/src/executor/display.rs` - UI & logging
4. `crates/cli/src/executor/wasm.rs` - WASM execution

**Steps**:
1. Create module files with proper headers
2. Move functions (keep `impl BiomeExecutor`)
3. Update `executor_impl.rs` to `mod` new files
4. Verify compilation
5. Run tests
6. Commit

**Expected Result**:
- executor_impl.rs: 933 lines → 150 lines (coordinator)
- commands.rs: 280 lines (public API)
- lifecycle.rs: 400 lines (core logic)
- display.rs: 170 lines (UI)
- wasm.rs: 80 lines (WASM)

**Total**: Still ~1080 lines (accounting for module headers), but **organized**!

---

### **Phase 2: byob_impl.rs** (BYOB Clarity)

**Files to Create**:
1. `crates/core/toadstool/src/byob/build.rs` - Build phase
2. `crates/core/toadstool/src/byob/operations.rs` - Operation phase
3. `crates/core/toadstool/src/byob/binding.rs` - Binding phase
4. `crates/core/toadstool/src/byob/health.rs` - Health monitoring

**Steps**: Same as Phase 1

**Expected Result**:
- byob_impl.rs: 928 lines → 150 lines
- 4 new modules: ~800 lines total

---

### **Phase 3: performance_hardening.rs** (Resource Clarity)

**Files to Create**:
1. `crates/core/toadstool/src/performance_hardening/mod.rs` - Coordinator
2. `crates/core/toadstool/src/performance_hardening/cpu.rs` - CPU tuning
3. `crates/core/toadstool/src/performance_hardening/memory.rs` - Memory tuning
4. `crates/core/toadstool/src/performance_hardening/io.rs` - I/O tuning
5. `crates/core/toadstool/src/performance_hardening/types.rs` - Shared types

**Steps**: Same as Phase 1

**Expected Result**:
- performance_hardening.rs: 920 lines → directory with 5 files
- Clearer resource-based organization

---

## 🏆 Success Criteria

### ✅ **Technical**
- [ ] Zero compilation errors
- [ ] Zero test failures
- [ ] Zero behavior changes
- [ ] Zero duplication introduced
- [ ] All functions accessible

### ✅ **Quality**
- [ ] Clear module boundaries
- [ ] Logical domain grouping
- [ ] Improved maintainability
- [ ] Better testability
- [ ] Enhanced documentation

### ✅ **Deep Debt**
- [ ] Smart refactoring (not arbitrary)
- [ ] Preserved functionality
- [ ] Clear responsibility per module
- [ ] No hardcoding introduced
- [ ] Async patterns maintained

---

## 📊 Expected Impact

### **Before Refactoring**

| File | Lines | Maintainability | Testability |
|------|-------|-----------------|-------------|
| executor_impl.rs | 933 | ⚠️ Hard | ⚠️ Hard |
| byob_impl.rs | 928 | ⚠️ Hard | ⚠️ Hard |
| performance_hardening.rs | 920 | ⚠️ Hard | ⚠️ Hard |

### **After Refactoring**

| Module | Files | Avg Lines/File | Maintainability | Testability |
|--------|-------|----------------|-----------------|-------------|
| executor/ | 5 | ~200 | ✅ Easy | ✅ Easy |
| byob/ | 5 | ~180 | ✅ Easy | ✅ Easy |
| performance_hardening/ | 5 | ~180 | ✅ Easy | ✅ Easy |

---

## 🚀 Timeline

| Phase | File | Effort | Status |
|-------|------|--------|--------|
| **Phase 1** | executor_impl.rs | 2-3 hours | ⏭️ Ready |
| **Phase 2** | byob_impl.rs | 2-3 hours | ⏭️ Ready |
| **Phase 3** | performance_hardening.rs | 2-3 hours | ⏭️ Ready |

**Total**: 6-9 hours of focused refactoring

---

## 📝 Notes

### **Rust Multi-File `impl` Pattern**

Rust allows implementing methods for a type across multiple files:

```rust
// executor_impl.rs
impl BiomeExecutor {
    pub async fn new() -> Result<Self> { ... }
}

// commands.rs
impl BiomeExecutor {
    pub async fn run_biome(&self, ...) -> Result<()> { ... }
}

// lifecycle.rs
impl BiomeExecutor {
    async fn start_biome_internal(&self, ...) -> Result<()> { ... }
}
```

This is the **key technique** for smart refactoring without duplicating state!

### **Module Organization**

Use Rust's module system properly:

```rust
// executor_impl.rs
mod commands;
mod lifecycle;
mod display;
mod wasm;

// All impl blocks are automatically merged by Rust!
```

---

## ✅ Approval Status

**Reviewed**: January 19, 2026  
**Approved By**: Deep Debt Review  
**Status**: ✅ **READY FOR EXECUTION**  
**Grade Target**: **S++** (Maintained!)

🍄 **Smart Refactoring: Logical Domains, Not Arbitrary Splits!** 🍄
