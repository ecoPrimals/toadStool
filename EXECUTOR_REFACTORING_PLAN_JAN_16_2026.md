# Executor Refactoring Plan - executor_impl.rs

**File**: `crates/cli/src/executor/executor_impl.rs`  
**Current Size**: 933 lines  
**Target Size**: <500 lines (main impl), rest in sub-modules  
**Date**: January 16, 2026

---

## 📊 **CURRENT STRUCTURE ANALYSIS**

### **File Composition**

**Total Lines**: 933  
**Impl Blocks**: 1 (single massive impl BiomeExecutor)  
**Public Methods**: 6 (API surface)  
**Private Methods**: 16 (internal helpers)

### **Method Categories** (22 total methods)

#### **1. Public API** (6 methods - KEEP IN MAIN FILE)
- `new()` - Constructor
- `run_biome()` - Foreground execution
- `up_biome()` - Background execution
- `down_biome()` - Stop biome
- `list_biomes()` - List running biomes
- `show_logs()` - View logs

#### **2. Lifecycle Management** (4 methods - EXTRACT TO MODULE)
- `start_biome_internal()` - Core startup logic (147 lines!)
- `stop_biome_internal()` - Core shutdown logic (54 lines)
- `graceful_stop_process()` - Graceful shutdown (22 lines)
- `force_kill_process()` - Force kill (17 lines)

#### **3. Process Management** (3 methods - EXTRACT TO MODULE)
- `start_primal()` - Start primal process (38 lines)
- `start_service()` - Start service process (41 lines)
- `workload_source_to_spec()` - Convert source to spec (46 lines)

#### **4. Resource Management** (2 methods - EXTRACT TO MODULE)
- `purge_biome_data()` - Clean up data (15 lines)
- `get_actual_pid()` - Get process PID (35 lines)

#### **5. UI/Display** (3 methods - EXTRACT TO MODULE)
- `print_biomes_table()` - Pretty print table (51 lines)
- `show_log_file()` - Display log file (11 lines)
- `tail_log_file()` - Tail log file (13 lines)

#### **6. Signal Handling** (2 methods - EXTRACT TO MODULE)
- `wait_for_interruption()` - Wait for signal (28 lines)
- `send_signal_to_process()` - Send Unix signal (14 lines)

#### **7. WASM-Specific** (2 methods - EXTRACT TO MODULE)
- `load_wasm_with_verification()` - Load WASM (30 lines)
- `execute_wasm_module()` - Execute WASM (13 lines)

---

## 🎯 **REFACTORING STRATEGY**

### **Goal**: Extract 16 private methods into 5 domain modules

**Result**:
- Main file: ~500 lines (6 public methods + glue code)
- 5 sub-modules: ~400 lines total
- Total: ~900 lines (similar total, but modular!)

---

## 📋 **MODULE EXTRACTION PLAN**

### **Module 1: `lifecycle.rs`** (Biome Lifecycle Management)

**Purpose**: Start/stop biome logic  
**Size**: ~240 lines

**Contains**:
- `start_biome_internal()` (147 lines)
- `stop_biome_internal()` (54 lines)
- `graceful_stop_process()` (22 lines)
- `force_kill_process()` (17 lines)

**Public API**:
```rust
pub(super) struct BiomeLifecycle<'a> {
    executor: &'a BiomeExecutor,
}

impl<'a> BiomeLifecycle<'a> {
    pub async fn start_biome(...) -> Result<BiomeInfo> { ... }
    pub async fn stop_biome(...) -> Result<()> { ... }
    async fn graceful_stop_process(...) -> Result<()> { ... }
    async fn force_kill_process(...) -> Result<()> { ... }
}
```

**Benefits**:
- Isolates complex startup/shutdown logic
- Testable in isolation
- Clear lifecycle semantics

---

### **Module 2: `process.rs`** (Process Management)

**Purpose**: Start and manage child processes  
**Size**: ~125 lines

**Contains**:
- `start_primal()` (38 lines)
- `start_service()` (41 lines)
- `workload_source_to_spec()` (46 lines)

**Public API**:
```rust
pub(super) struct ProcessManager<'a> {
    executor: &'a BiomeExecutor,
}

impl<'a> ProcessManager<'a> {
    pub async fn start_primal(...) -> Result<ProcessHandle> { ... }
    pub async fn start_service(...) -> Result<ProcessHandle> { ... }
    pub async fn workload_to_spec(...) -> Result<WorkloadSpec> { ... }
}
```

**Benefits**:
- Centralizes process spawning logic
- Easier to test process creation
- Clear separation from lifecycle

---

### **Module 3: `resources.rs`** (Resource Management)

**Purpose**: Manage biome resources and cleanup  
**Size**: ~50 lines

**Contains**:
- `purge_biome_data()` (15 lines)
- `get_actual_pid()` (35 lines)

**Public API**:
```rust
pub(super) struct ResourceManager<'a> {
    executor: &'a BiomeExecutor,
}

impl<'a> ResourceManager<'a> {
    pub async fn purge_data(biome_name: &str) -> Result<()> { ... }
    pub async fn get_pid(biome_name: &str) -> Result<u32> { ... }
}
```

**Benefits**:
- Isolates resource cleanup logic
- Clear ownership of resource operations
- Easier to add new resource types

---

### **Module 4: `display.rs`** (UI and Display)

**Purpose**: Pretty printing and log display  
**Size**: ~75 lines

**Contains**:
- `print_biomes_table()` (51 lines)
- `show_log_file()` (11 lines)
- `tail_log_file()` (13 lines)

**Public API**:
```rust
pub(super) struct DisplayManager;

impl DisplayManager {
    pub async fn print_table(biomes: &HashMap<...>) -> Result<()> { ... }
    pub async fn show_logs(path: &Path) -> Result<()> { ... }
    pub async fn tail_logs(path: &Path, lines: usize) -> Result<()> { ... }
}
```

**Benefits**:
- Separates display from business logic
- Testable formatting logic
- Easy to change UI without touching core

---

### **Module 5: `signals.rs`** (Signal Handling)

**Purpose**: Unix signal handling and interruption  
**Size**: ~42 lines

**Contains**:
- `wait_for_interruption()` (28 lines)
- `send_signal_to_process()` (14 lines)

**Public API**:
```rust
pub(super) struct SignalManager;

impl SignalManager {
    pub async fn wait_for_interrupt() -> Result<()> { ... }
    pub fn send_signal(pid: u32, signal: &str) -> Result<()> { ... }
}
```

**Benefits**:
- Isolates platform-specific signal code
- Easier to test signal handling
- Clear signal management semantics

---

### **Module 6: `wasm.rs`** (WASM-Specific, OPTIONAL)

**Purpose**: WASM verification and execution  
**Size**: ~43 lines

**Contains**:
- `load_wasm_with_verification()` (30 lines)
- `execute_wasm_module()` (13 lines)

**Public API**:
```rust
#[cfg(feature = "wasm")]
pub(super) struct WasmManager<'a> {
    executor: &'a BiomeExecutor,
}

#[cfg(feature = "wasm")]
impl<'a> WasmManager<'a> {
    pub async fn load_verified(path: &Path) -> Result<Vec<u8>> { ... }
    pub async fn execute_module(bytes: &[u8]) -> Result<()> { ... }
}
```

**Benefits**:
- Feature-gated (WASM optional)
- Isolates WASM complexity
- Testable independently

---

## 🏗️ **IMPLEMENTATION STEPS**

### **Phase 1: Create Module Files** (1 hour)

1. Create `crates/cli/src/executor/lifecycle.rs`
2. Create `crates/cli/src/executor/process.rs`
3. Create `crates/cli/src/executor/resources.rs`
4. Create `crates/cli/src/executor/display.rs`
5. Create `crates/cli/src/executor/signals.rs`
6. Create `crates/cli/src/executor/wasm.rs` (optional)

### **Phase 2: Extract Methods** (3 hours)

For each module:
1. Copy methods from executor_impl.rs
2. Create struct with `&BiomeExecutor` reference
3. Adapt methods to work in new context
4. Add `pub(super)` visibility
5. Update imports

### **Phase 3: Update Main File** (2 hours)

1. Update `executor_impl.rs` imports
2. Replace method calls with module calls
3. Remove extracted methods
4. Keep only 6 public API methods
5. Add module declarations

### **Phase 4: Update `mod.rs`** (30 min)

```rust
// crates/cli/src/executor/mod.rs
mod executor_impl;
mod lifecycle;
mod process;
mod resources;
mod display;
mod signals;
#[cfg(feature = "wasm")]
mod wasm;

pub use executor_impl::BiomeExecutor;
```

### **Phase 5: Test** (1 hour)

1. Run existing tests
2. Verify all public API works
3. Check compile times (should improve!)
4. No behavior changes (pure refactor)

**Total Time**: ~7.5 hours (1 day)

---

## ✅ **EXPECTED BENEFITS**

### **Before Refactoring**

**Structure**:
- 1 file: 933 lines
- 1 impl block: 22 methods
- Hard to navigate
- Hard to test individual concerns
- Long compile times

**Readability**: ⚠️ Difficult
**Testability**: ⚠️ Limited  
**Maintainability**: ⚠️ Hard

### **After Refactoring**

**Structure**:
- 7 files: ~900 lines total
- Main file: ~500 lines (6 methods)
- 5 modules: ~400 lines (16 methods)
- Easy to navigate
- Easy to test each domain
- Faster compile times (parallel compilation)

**Readability**: ✅ Excellent  
**Testability**: ✅ Excellent  
**Maintainability**: ✅ Excellent

---

## 🎯 **SUCCESS CRITERIA**

**Quantitative**:
- ✅ Main file <500 lines
- ✅ Each module <150 lines
- ✅ Zero behavioral changes
- ✅ All tests pass
- ✅ Compile time improved

**Qualitative**:
- ✅ Clear domain boundaries
- ✅ Single responsibility per module
- ✅ Easy to find code
- ✅ Easy to test in isolation
- ✅ Future features easier to add

---

## 📝 **EXAMPLE: Before/After**

### **Before** (executor_impl.rs)

```rust
impl BiomeExecutor {
    pub async fn run_biome(...) -> Result<()> {
        // 50 lines of logic
        self.start_biome_internal(...).await?;
        self.wait_for_interruption().await?;
        self.stop_biome_internal(...).await?;
        // ...
    }

    async fn start_biome_internal(...) -> Result<BiomeInfo> {
        // 147 lines of complex startup logic!
    }

    async fn wait_for_interruption(&self) -> Result<()> {
        // 28 lines of signal handling
    }

    // ... 19 more methods ...
}
```

**Issues**: Everything in one place, hard to navigate!

### **After** (executor_impl.rs + modules)

```rust
// executor_impl.rs (main file - clean!)
impl BiomeExecutor {
    pub async fn run_biome(...) -> Result<()> {
        // 50 lines of orchestration
        let lifecycle = BiomeLifecycle::new(self);
        let biome = lifecycle.start_biome(...).await?;
        
        let signals = SignalManager;
        signals.wait_for_interrupt().await?;
        
        lifecycle.stop_biome(&biome).await?;
        // ...
    }
}

// lifecycle.rs (domain module - focused!)
impl<'a> BiomeLifecycle<'a> {
    pub async fn start_biome(...) -> Result<BiomeInfo> {
        // 147 lines of startup logic (but in its own file!)
    }
}

// signals.rs (domain module - focused!)
impl SignalManager {
    pub async fn wait_for_interrupt() -> Result<()> {
        // 28 lines of signal handling (but in its own file!)
    }
}
```

**Benefits**: Clear separation, easy to find, easy to test!

---

## 🚀 **READY TO EXECUTE**

**Status**: Plan complete, ready to implement!

**Next Steps**:
1. Create module files
2. Extract methods
3. Update main file
4. Test
5. Commit

**Timeline**: 1 day (7.5 hours)

---

**Created**: January 16, 2026  
**Purpose**: Systematic executor refactoring  
**Goal**: 933 lines → <500 lines main + 5 modules  
**Status**: Ready to implement!

🦀🧬✨ **Modern Idiomatic Modular Rust!** ✨🧬🦀
