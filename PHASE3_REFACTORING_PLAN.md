# Phase 3: Large File Refactoring Plan

**Date**: February 6, 2026  
**Objective**: Smart semantic refactoring of files >500 lines  
**Principle**: Meaningful boundaries, NOT mechanical splitting  

---

## 📊 Files Requiring Refactoring (19 files >500 lines)

| File | Lines | Priority | Complexity |
|------|-------|----------|------------|
| `ops/mha.rs` | 845 | P0 | High |
| `esn_v2.rs` | 795 | P1 | Medium |
| `ops/cross_attn.rs` | 768 | P0 | High |
| `ops/nonzero.rs` | 735 | P2 | Low |
| `ops/local_attention.rs` | 728 | P1 | High |
| `tensor.rs` | 723 | P0 | Critical |
| `genomics.rs` | 681 | P2 | Medium |
| `ops/adamw.rs` | 665 | P2 | Low |
| `ops/nms.rs` | 648 | P2 | Medium |
| `ops/sparse_attn.rs` | 635 | P1 | High |
| `ops/masked_select.rs` | 628 | P2 | Low |
| `ops/nadam.rs` | 626 | P2 | Low |
| `ops/adadelta.rs` | 623 | P2 | Low |
| `timeseries.rs` | 618 | P2 | Medium |
| `ops/causal_attn.rs` | 616 | P1 | High |
| `ops/triplet_loss.rs` | 609 | P2 | Medium |
| `ops/fhe_intt.rs` | 598 | P3 | Low |
| `ops/expand.rs` | 596 | P2 | Low |
| `ops/adam.rs` | 596 | P2 | Low |

**Total**: 12,193 lines to refactor

---

## 🎯 Priority 0: Critical Files (3 files, ~2,336 lines)

### 1. `ops/mha.rs` (845 lines) - Multi-Head Attention

**Current Structure**:
- Module docs (1-43): 43 lines
- MhaParams struct (47-57): 11 lines
- MultiHeadAttention struct (62-72): 11 lines
- `impl MultiHeadAttention` (73-541): **469 lines** ⚠️
- `impl Tensor` (542-596): 55 lines
- Tests (597-845): 248 lines

**Refactoring Strategy** (Smart Semantic Split):

```
ops/mha/
├── mod.rs              (150 lines) - Public API + core logic
├── projections.rs      (200 lines) - Q/K/V projection methods
├── shaders.rs          (150 lines) - WGSL shader definitions
└── tests.rs            (250 lines) - Test suite
```

**Semantic Boundaries**:
1. **Core Logic** (`mod.rs`):
   - Public API (`impl Tensor`)
   - Struct definitions
   - Main `new()` and `execute()` methods
   - High-level orchestration

2. **Projections** (`projections.rs`):
   - `project_qkv()` - Query/Key/Value projections
   - `project_output()` - Output projection
   - Head splitting/concatenation logic
   - Buffer management for projections

3. **Shaders** (`shaders.rs`):
   - WGSL shader source code
   - Shader compilation helpers
   - Pipeline creation
   - Bind group layouts

4. **Tests** (`tests.rs`):
   - All test cases
   - Test helpers
   - Validation utilities

**Benefits**:
- Clear separation of concerns
- Easier to understand each component
- Shader changes don't affect core logic
- Testable in isolation
- Better IDE navigation

---

### 2. `tensor.rs` (723 lines) - Core Tensor Implementation

**Current Structure**: Single monolithic file

**Refactoring Strategy**:

```
tensor/
├── mod.rs              (200 lines) - Public API + core struct
├── creation.rs         (150 lines) - Creation methods (zeros, ones, randn, etc.)
├── operations.rs       (150 lines) - Basic ops (add, mul, reshape, etc.)
├── conversion.rs       (100 lines) - to_vec, from_vec, serialization
└── tests.rs            (150 lines) - Test suite
```

**Semantic Boundaries**:
1. **Core** (`mod.rs`): Tensor struct, device management, basic methods
2. **Creation**: Factory methods and constructors
3. **Operations**: Arithmetic and shape operations
4. **Conversion**: Data format conversions

---

### 3. `ops/cross_attn.rs` (768 lines) - Cross Attention

Similar pattern to MHA, extract:
- Core logic
- Projections
- Shaders
- Tests

---

## 🎯 Priority 1: Attention Modules (4 files, ~2,684 lines)

### Pattern for Attention Modules

All attention modules follow similar structure:
- `local_attention.rs` (728 lines)
- `sparse_attn.rs` (635 lines)
- `causal_attn.rs` (616 lines)
- `esn_v2.rs` (795 lines) - Different pattern (stateful network)

**Common Refactoring Strategy**:

```
ops/<attention_type>/
├── mod.rs       - Public API + orchestration
├── compute.rs   - Core computation logic
├── shaders.rs   - WGSL shaders
└── tests.rs     - Test suite
```

---

## 🎯 Priority 2: Optimizer Modules (6 files, ~3,835 lines)

### Pattern for Optimizers

All optimizer modules share structure:
- `adamw.rs` (665 lines)
- `nadam.rs` (626 lines)
- `adadelta.rs` (623 lines)
- `adam.rs` (596 lines)
- `nms.rs` (648 lines)
- `triplet_loss.rs` (609 lines)

**Common Structure**:
- Optimizer state management
- Step computation
- Momentum/velocity updates
- Parameter updates
- Tests

**Refactoring Strategy**:

```
ops/<optimizer>/
├── mod.rs       - Public API + state struct
├── step.rs      - Step computation logic
├── shaders.rs   - WGSL shaders
└── tests.rs     - Tests
```

---

## 🎯 Priority 3: Specialized Modules (6 files, ~3,338 lines)

Lower priority but still need refactoring:
- `genomics.rs` (681 lines) - Extract k-mer, alignment, scoring
- `timeseries.rs` (618 lines) - Extract forecasting, analysis, decomposition
- `nonzero.rs` (735 lines) - Extract selection, masking, indexing
- `masked_select.rs` (628 lines) - Similar to nonzero
- `expand.rs` (596 lines) - Extract broadcasting, tiling
- `fhe_intt.rs` (598 lines) - Extract FHE primitives, NTT operations

---

## 📋 Refactoring Execution Plan

### Wave 1: Attention Modules (Priority 0-1)
**Target**: 7 files → ~28 modules  
**Lines**: ~5,020 → distributed  
**Effort**: 15-20 hours  

Files:
1. `ops/mha.rs` → 4 modules
2. `tensor.rs` → 5 modules
3. `ops/cross_attn.rs` → 4 modules
4. `ops/local_attention.rs` → 4 modules
5. `ops/sparse_attn.rs` → 4 modules
6. `ops/causal_attn.rs` → 4 modules
7. `esn_v2.rs` → 3 modules

### Wave 2: Optimizers (Priority 2)
**Target**: 6 files → ~24 modules  
**Lines**: ~3,835 → distributed  
**Effort**: 8-12 hours  

### Wave 3: Specialized (Priority 3)
**Target**: 6 files → ~18 modules  
**Lines**: ~3,338 → distributed  
**Effort**: 8-10 hours  

---

## ✅ Success Criteria

### Per-File
- ✅ All files <500 lines
- ✅ Clear semantic boundaries
- ✅ Single responsibility per module
- ✅ Tests still pass
- ✅ Public API unchanged
- ✅ No performance regression

### Overall
- ✅ 19 files → ~70 well-organized modules
- ✅ Better IDE navigation
- ✅ Easier code review
- ✅ Improved maintainability
- ✅ Clear module hierarchy

---

## 🎓 Deep Debt Principles for Refactoring

### ✅ Smart, Not Mechanical
- Respect semantic boundaries
- Group related functionality
- Clear interfaces between modules
- Logical organization

### ✅ Maintainability First
- Single responsibility
- Clear naming
- Good documentation
- Easy to understand

### ✅ No Breaking Changes
- Public API unchanged
- Tests pass unchanged
- Performance maintained
- Backward compatible

### ✅ Module Hierarchy
```
ops/
├── attention/
│   ├── multi_head/
│   ├── cross/
│   ├── local/
│   ├── sparse/
│   └── causal/
├── optimizer/
│   ├── adam/
│   ├── adamw/
│   ├── nadam/
│   └── adadelta/
└── loss/
    ├── triplet/
    └── nms/
```

---

## 📊 Impact Analysis

### Before Refactoring
- 19 files >500 lines
- Average file: 641 lines
- Largest file: 845 lines
- Hard to navigate
- Mixed concerns
- Test discovery difficult

### After Refactoring
- 0 files >500 lines
- ~70 focused modules
- Average module: 175 lines
- Easy to navigate
- Clear separation
- Tests well-organized

---

## 🚀 Next Steps

1. **Start with MHA** (`ops/mha.rs`) - Most complex, sets pattern
2. **Verify tests pass** after each refactoring
3. **Document module structure** in each `mod.rs`
4. **Apply pattern** to similar files
5. **Review and iterate** based on learnings

---

**Status**: Ready for Execution  
**Estimated Total Effort**: 31-42 hours  
**Expected Result**: Clean, maintainable, well-organized codebase  

---

*This plan follows deep debt principles: smart refactoring based on semantics, not mechanics. Each split creates meaningful, cohesive modules with clear responsibilities.*
