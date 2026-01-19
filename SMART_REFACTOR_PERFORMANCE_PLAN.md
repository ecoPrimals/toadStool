# 🔧 Smart Refactoring Plan: performance_hardening.rs

**Date**: January 19, 2026  
**File**: `crates/core/toadstool/src/performance_hardening.rs`  
**Current Size**: 1,322 lines (OVER 1000-line limit)  
**Target**: 6 modules organized by resource domain  
**Principle**: Smart refactoring by logical domains, not arbitrary splitting

---

## 📊 Current Structure Analysis

**File Breakdown**:
- Lines 1-199: Configuration types (all 6 config structs + defaults)
- Lines 200-332: Resource monitoring (OptimizedResourceMonitor + metrics)
- Lines 333-477: Memory pools (MemoryPool + PooledObject)
- Lines 478-660: Intelligent caching (IntelligentCache + stats)
- Lines 661-799: Async batching (AsyncBatcher)
- Lines 800-1322: Manager (PerformanceHardeningManager + coordination)

**Total**: 1,322 lines

---

## 🎯 Refactoring Strategy

### **Module 1: `types.rs`** (~200 lines)
**Purpose**: All configuration and stats types

**Contains**:
- `PerformanceHardeningConfig`
- `OptimizedMonitoringConfig`
- `MemoryPoolConfig`
- `CachingConfig`
- `AsyncOptimizationConfig`
- `PerformanceConnectionPoolConfig`
- `AggregatedMetrics`
- `PoolStats`
- `CacheStats`

**Rationale**: Centralize all types for easy reference and reuse

---

### **Module 2: `monitoring.rs`** (~150 lines)
**Purpose**: Resource monitoring and metrics

**Contains**:
- `OptimizedResourceMonitor` struct + impl
- Adaptive sampling logic
- Metrics collection
- Aggregation

**Rationale**: Clear domain - monitoring is distinct from other operations

---

### **Module 3: `memory.rs`** (~200 lines)
**Purpose**: Memory pool management

**Contains**:
- `MemoryPool<T>` struct + impl
- `PooledObject<T>` struct + impl
- Pool allocation/deallocation
- Pool statistics

**Rationale**: Memory management is a distinct resource domain

---

### **Module 4: `caching.rs`** (~200 lines)
**Purpose**: Intelligent caching

**Contains**:
- `IntelligentCache<K, V>` struct + impl
- LRU eviction
- TTL management
- Cache statistics

**Rationale**: Caching is independent from other concerns

---

### **Module 5: `async_ops.rs`** (~150 lines)
**Purpose**: Async operation batching

**Contains**:
- `AsyncBatcher<T, R>` struct + impl
- Batch processing
- Queue management
- Concurrency control

**Rationale**: Async optimization is a separate concern

---

### **Module 6: `mod.rs`** (~450 lines)
**Purpose**: Main coordinator and public API

**Contains**:
- `PerformanceHardeningManager` struct + impl
- Module re-exports
- Integration logic
- Public API surface

**Rationale**: Orchestrates all domains, provides unified interface

---

## 📁 New Structure

```
crates/core/toadstool/src/performance_hardening/
├── mod.rs              (~450 lines) - Manager + public API
├── types.rs            (~200 lines) - All config/stats types
├── monitoring.rs       (~150 lines) - Resource monitoring
├── memory.rs           (~200 lines) - Memory pools
├── caching.rs          (~200 lines) - Intelligent caching
└── async_ops.rs        (~150 lines) - Async batching

Total: ~1,350 lines (slightly more due to module overhead)
```

**vs Old**:
```
crates/core/toadstool/src/performance_hardening.rs (1,322 lines)
```

---

## ✅ Benefits

1. **Clear Domain Boundaries**: Each module has single responsibility
2. **Improved Maintainability**: Easy to find relevant code
3. **Better Testability**: Can test each domain independently
4. **Logical Organization**: Matches mental model of system
5. **No Duplication**: Shared types in types.rs
6. **Preserves Functionality**: Zero behavior changes

---

## 🚀 Execution Plan

### **Step 1**: Create module directory ✅
```bash
mkdir -p crates/core/toadstool/src/performance_hardening
```

### **Step 2**: Create types.rs
- Extract all config structs (lines 23-199)
- Extract all stats structs (AggregatedMetrics, PoolStats, CacheStats)
- Keep all Default impls

### **Step 3**: Create monitoring.rs
- Extract OptimizedResourceMonitor (lines 202-332)
- Import types from types.rs

### **Step 4**: Create memory.rs
- Extract MemoryPool + PooledObject (lines 333-477)
- Import types from types.rs

### **Step 5**: Create caching.rs
- Extract IntelligentCache (lines 478-660)
- Import types from types.rs

### **Step 6**: Create async_ops.rs
- Extract AsyncBatcher (lines 661-799)
- Import types from types.rs

### **Step 7**: Create mod.rs
- Extract PerformanceHardeningManager (lines 800-1322)
- Add module declarations
- Re-export public types
- Import from submodules

### **Step 8**: Update parent lib.rs
```rust
// OLD:
pub mod performance_hardening;

// NEW: (same, but now it's a directory module)
pub mod performance_hardening;
```

### **Step 9**: Verify compilation
```bash
cargo build --package toadstool
```

### **Step 10**: Run tests
```bash
cargo test --package toadstool performance_hardening
```

---

## 🎯 Success Criteria

- [ ] All files under 500 lines (well under 1000 limit)
- [ ] Zero compilation errors
- [ ] Zero test failures
- [ ] Zero behavior changes
- [ ] Clear module boundaries
- [ ] Improved code organization

---

## 📝 Notes

### **Why This Is "Smart" Refactoring**:
1. ✅ Split by **resource domains** (monitoring, memory, cache, async)
2. ✅ NOT by arbitrary line counts
3. ✅ Each module has **clear responsibility**
4. ✅ Preserves **logical cohesion**
5. ✅ Follows **Deep Debt principles**

### **Why NOT Just Split Into 2 Files**:
- Would create 2 large files with mixed concerns
- Wouldn't improve organization
- Wouldn't follow domain boundaries

### **Rust Module Pattern**:
```rust
// mod.rs is the module root
// Other files are submodules
// All can access each other via use statements
```

---

**Status**: Ready for execution  
**Estimated Time**: 2-3 hours  
**Complexity**: Medium (clear structure, mechanical extraction)

🍄 **Smart Refactoring: Logical Domains Over Arbitrary Limits!** 🍄
