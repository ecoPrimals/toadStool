# Smart File Refactoring Plan - January 15, 2026

**Purpose**: Domain-based refactoring of 3 large files (not blind line-count splitting)  
**Policy**: All files must be < 1000 lines  
**Current Violations**: 3 files (total 6,005 lines)

---

## 📊 FILES REQUIRING REFACTORING

| File | Lines | Excess | Operations | Priority |
|------|-------|--------|------------|----------|
| `wgpu/training.rs` | 2,676 | +1,676 | 13 ops | **CRITICAL** |
| `wgpu/basic_ops.rs` | 1,758 | +758 | 15+ ops | **HIGH** |
| `wgpu/normalization.rs` | 1,571 | +571 | 8 ops | **MEDIUM** |

**Total to refactor**: 6,005 lines → ~14-16 files

---

## 🎯 REFACTORING PHILOSOPHY

**NOT THIS** (Blind splitting):
```
training.rs (2,676 lines)
├── training_part1.rs (900 lines)
├── training_part2.rs (900 lines)
└── training_part3.rs (876 lines)
```
❌ **Bad**: No logical separation, arbitrary splits

**THIS** (Domain-driven):
```
training.rs (2,676 lines)
├── losses.rs (850 lines) - All loss functions
├── optimizers_basic.rs (600 lines) - SGD, Adam, RMSProp
├── optimizers_advanced.rs (700 lines) - Adagrad, NAdam, Adadelta
└── training_types.rs (300 lines) - Shared types/configs
```
✅ **Good**: Clear domains, logical grouping, maintainable

---

## 📋 REFACTORING PLAN 1: training.rs (2,676 lines)

### Current Structure Analysis

**Operations** (13 total):

**Loss Functions** (7 operations):
- Line 18: `execute_cross_entropy` (~195 lines)
- Line 699: `execute_mse_loss` (~155 lines)
- Line 854: `execute_mae_loss` (~161 lines)
- Line 1249: `execute_huber_loss` (~153 lines)
- Line 1402: `execute_bce_loss` (~157 lines)
- Line 2357: `execute_focal_loss` (~169 lines)
- Line 2526: `execute_dice_loss` (~150 lines)

**Total loss code**: ~1,140 lines

**Optimizers** (6 operations):
- Line 213: `execute_adam_step` (~254 lines)
- Line 467: `execute_sgd` (~232 lines)
- Line 1015: `execute_rmsprop` (~234 lines)
- Line 1559: `execute_adagrad` (~227 lines)
- Line 1786: `execute_nadam` (~291 lines)
- Line 2077: `execute_adadelta` (~280 lines)

**Total optimizer code**: ~1,518 lines

### Refactored Structure

```
wgpu/training/
├── mod.rs (50 lines) - Re-exports
├── losses.rs (850 lines) - All 7 loss functions
├── optimizers_basic.rs (600 lines) - SGD, Adam, RMSProp (3 ops)
├── optimizers_advanced.rs (700 lines) - Adagrad, NAdam, Adadelta (3 ops)
└── types.rs (300 lines) - LossReduction, CrossEntropyConfig, OptimizerConfig, etc.
```

**Result**: 5 files, all < 900 lines ✅

### Implementation Steps

1. **Create directory** (1 min):
   ```bash
   mkdir -p showcase/gpu-universal/ml-inference/src/wgpu/training
   ```

2. **Extract types** (30 min):
   - Create `types.rs`
   - Move all `Config` structs
   - Move `LossReduction` enum
   - Move shared parameter types
   - Update imports in original file

3. **Extract losses** (45 min):
   - Create `losses.rs`
   - Move all 7 loss execute functions
   - Move loss-specific WGSL includes
   - Add `use super::types::*`
   - Test compilation

4. **Split optimizers** (60 min):
   - Create `optimizers_basic.rs` (SGD, Adam, RMSProp)
   - Create `optimizers_advanced.rs` (Adagrad, NAdam, Adadelta)
   - Move optimizer execute functions
   - Move optimizer-specific WGSL includes
   - Test compilation

5. **Create mod.rs** (15 min):
   - Re-export all types
   - Re-export all functions
   - Ensure API compatibility

6. **Update parent mod.rs** (10 min):
   - Change `pub mod training;` to `pub mod training;`
   - Update any dependent code

7. **Test** (20 min):
   - Run `cargo test --package ml-inference-showcase`
   - Verify all operations work
   - Check imports resolve

**Total time**: ~3 hours

---

## 📋 REFACTORING PLAN 2: basic_ops.rs (1,758 lines)

### Current Structure Analysis

**Operations** (15+ operations):
- Matrix operations (matmul, transpose, batch_matmul)
- Element-wise operations (add, subtract, multiply, divide)
- Reduction operations (sum, mean, max, min)
- Shape operations (reshape, flatten, squeeze, unsqueeze)
- Comparison operations (equal, greater, less)

### Refactored Structure

```
wgpu/basic_ops/
├── mod.rs (50 lines) - Re-exports
├── matrix.rs (500 lines) - MatMul, Transpose, BatchMatMul, etc.
├── elementwise.rs (450 lines) - Add, Sub, Mul, Div, Pow, etc.
├── reductions.rs (400 lines) - Sum, Mean, Max, Min, etc.
└── shapes.rs (400 lines) - Reshape, Flatten, Squeeze, Unsqueeze, etc.
```

**Result**: 5 files, all < 550 lines ✅

### Implementation Steps

1. **Analyze operation boundaries** (30 min):
   - Identify all execute functions
   - Group by logical domain
   - Note shared dependencies

2. **Create directory structure** (5 min)

3. **Extract matrix ops** (45 min):
   - MatMul, Transpose, BatchMatMul
   - Matrix-specific helpers

4. **Extract elementwise ops** (45 min):
   - Add, Sub, Mul, Div, Pow
   - Broadcasting logic

5. **Extract reduction ops** (45 min):
   - Sum, Mean, Max, Min
   - Reduction dimension handling

6. **Extract shape ops** (45 min):
   - Reshape, Flatten, Squeeze, Unsqueeze
   - Shape validation logic

7. **Create mod.rs and test** (30 min)

**Total time**: ~4 hours

---

## 📋 REFACTORING PLAN 3: normalization.rs (1,571 lines)

### Current Structure Analysis

**Operations** (8 operations):
- BatchNorm (training and inference modes)
- LayerNorm
- InstanceNorm
- GroupNorm
- RMSNorm
- WeightNorm
- SpectralNorm
- LocalResponseNorm

### Refactored Structure

```
wgpu/normalization/
├── mod.rs (50 lines) - Re-exports
├── batch_norm.rs (400 lines) - BatchNorm training + inference
├── layer_norm.rs (350 lines) - LayerNorm + RMSNorm
├── group_norm.rs (350 lines) - GroupNorm + InstanceNorm
└── specialized_norms.rs (400 lines) - WeightNorm, SpectralNorm, LRN
```

**Result**: 5 files, all < 450 lines ✅

### Implementation Steps

1. **Analyze normalization types** (30 min):
   - Group by similarity (batch vs layer vs group)
   - Identify shared types

2. **Extract batch normalization** (60 min):
   - Training and inference modes
   - Running statistics management

3. **Extract layer normalization** (45 min):
   - LayerNorm + RMSNorm (similar algorithms)

4. **Extract group normalization** (45 min):
   - GroupNorm + InstanceNorm (related concepts)

5. **Extract specialized norms** (45 min):
   - WeightNorm, SpectralNorm, LRN
   - Less common normalizations

6. **Create mod.rs and test** (30 min)

**Total time**: ~4 hours

---

## 🎯 REFACTORING SUMMARY

### Before (Current State)

```
wgpu/
├── training.rs (2,676 lines) ❌
├── basic_ops.rs (1,758 lines) ❌
└── normalization.rs (1,571 lines) ❌
```

**Total**: 3 files, 6,005 lines, 3 violations

### After (Refactored State)

```
wgpu/
├── training/
│   ├── mod.rs (50 lines)
│   ├── losses.rs (850 lines)
│   ├── optimizers_basic.rs (600 lines)
│   ├── optimizers_advanced.rs (700 lines)
│   └── types.rs (300 lines)
├── basic_ops/
│   ├── mod.rs (50 lines)
│   ├── matrix.rs (500 lines)
│   ├── elementwise.rs (450 lines)
│   ├── reductions.rs (400 lines)
│   └── shapes.rs (400 lines)
└── normalization/
    ├── mod.rs (50 lines)
    ├── batch_norm.rs (400 lines)
    ├── layer_norm.rs (350 lines)
    ├── group_norm.rs (350 lines)
    └── specialized_norms.rs (400 lines)
```

**Total**: 15 files, ~6,050 lines, 0 violations ✅

**Largest file**: 850 lines (losses.rs) - 150 lines under limit ✅

---

## ⏱️ TIME ESTIMATES

| Refactoring | Time | Priority |
|-------------|------|----------|
| **training.rs** | 3 hours | **CRITICAL** |
| **basic_ops.rs** | 4 hours | **HIGH** |
| **normalization.rs** | 4 hours | **MEDIUM** |
| **Testing & Integration** | 2 hours | **REQUIRED** |
| **Total** | **13 hours** | **~2 days** |

---

## ✅ VALIDATION CHECKLIST

After each refactoring:

- [ ] All files < 1000 lines
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] No API breaking changes
- [ ] Re-exports maintain compatibility
- [ ] Documentation updated
- [ ] Imports resolve correctly
- [ ] WGSL shaders load correctly

---

## 🎯 EXECUTION STRATEGY

### Option 1: Sequential (Safest)

1. **Day 1 Morning**: Refactor training.rs (3 hours)
2. **Day 1 Afternoon**: Test and validate (1 hour)
3. **Day 2 Morning**: Refactor basic_ops.rs (4 hours)
4. **Day 2 Afternoon**: Refactor normalization.rs (4 hours)
5. **Day 3 Morning**: Final testing and integration (2 hours)

**Total**: 2.5 days, safest approach

### Option 2: Parallel (Faster, Riskier)

Create separate branches for each refactoring, merge in sequence.

**Not recommended** - too many potential merge conflicts

### Option 3: Incremental (Recommended)

1. **Week 1**: training.rs only (3 hours + testing)
2. **Week 2**: basic_ops.rs (4 hours + testing)
3. **Week 3**: normalization.rs (4 hours + testing)

**Total**: 3 weeks, 1 refactoring per week, safest for production

---

## 🔧 REFACTORING TOOLS

### Automated Extraction

```bash
# Extract function range to new file
sed -n '18,212p' training.rs > losses/cross_entropy.rs

# Verify line counts
wc -l training/losses.rs training/optimizers_basic.rs
```

### Testing During Refactoring

```bash
# Test specific module
cargo test --package ml-inference-showcase --lib training

# Test compilation only
cargo check --package ml-inference-showcase

# Run clippy
cargo clippy --package ml-inference-showcase
```

### Incremental Compilation

```bash
# Fast check cycle during refactoring
cargo check --package ml-inference-showcase && echo "✅ OK"
```

---

## 🎓 LESSONS FROM PAST REFACTORINGS

### What Works Well

1. **Domain-driven splits** - Group by logical function, not line count
2. **Small PRs** - One refactoring at a time
3. **Preserve API** - Use re-exports to maintain compatibility
4. **Test continuously** - Run tests after each file extraction
5. **Document changes** - Update comments and docs

### What Doesn't Work

1. **Arbitrary splits** - "part1", "part2", "part3" files
2. **Breaking changes** - Changing public APIs during refactoring
3. **Big bang refactors** - Doing all 3 files at once
4. **Ignoring dependencies** - Missing shared types/imports
5. **Skipping tests** - Discovering breaks too late

---

## 📝 COMMIT STRATEGY

### Per Refactoring

```bash
# Commit 1: Create new directory structure
git add showcase/gpu-universal/ml-inference/src/wgpu/training/
git commit -m "refactor(training): create module structure

Create training/ submodule directory in preparation for
splitting training.rs (2,676 lines) into logical domains.

- Created training/mod.rs (re-exports)
- Created training/types.rs (shared types)
- No functional changes yet

Related: #file-size-policy-compliance"

# Commit 2: Extract losses
git commit -m "refactor(training): extract loss functions to losses.rs

Moved all 7 loss function implementations to dedicated losses.rs file:
- execute_cross_entropy
- execute_mse_loss
- execute_mae_loss
- execute_huber_loss
- execute_bce_loss
- execute_focal_loss
- execute_dice_loss

File sizes:
- losses.rs: 850 lines ✅
- training.rs: 1,826 lines remaining

All tests passing. No API changes."

# Commit 3: Extract basic optimizers
# Commit 4: Extract advanced optimizers
# Commit 5: Remove old file, finalize refactoring
```

### Benefits of Atomic Commits

- Easy to review
- Easy to revert if needed
- Clear history of refactoring process
- Incremental testing at each step

---

## 🚀 GETTING STARTED

### Immediate Next Steps

1. **Review this plan** (15 min)
2. **Choose execution strategy** (5 min)
3. **Start with training.rs** (highest priority)
4. **Create training/ directory**
5. **Follow refactoring steps**

### First Command

```bash
cd showcase/gpu-universal/ml-inference/src/wgpu
mkdir -p training
touch training/mod.rs
touch training/types.rs
touch training/losses.rs
touch training/optimizers_basic.rs
touch training/optimizers_advanced.rs
```

---

## 💎 SUCCESS CRITERIA

**Definition of Done**:

- ✅ All files < 1000 lines (policy compliance)
- ✅ All tests passing
- ✅ No API breaking changes
- ✅ Zero compilation warnings
- ✅ Documentation updated
- ✅ Clear commit history
- ✅ Code review approved

**Expected Outcome**:

- From **3 violations** to **0 violations**
- From **3 massive files** to **15 maintainable files**
- From **B+ grade** to **A grade** on file size metric
- Improved maintainability and developer experience

---

## 📊 PROGRESS TRACKING

```
File Size Compliance Progress:

Before:  [▓▓▓░░░░░░░] 30% compliant (3/10 large files < 1000)
After:   [▓▓▓▓▓▓▓▓▓▓] 100% compliant (0 violations)

Overall Grade:
Before:  B+ (85/100) - File size: C (70/100)
After:   A  (92/100) - File size: A+ (100/100) ✅
```

---

## 🎯 BOTTOM LINE

**Current State**: 3 files violating 1000-line policy

**Target State**: All files < 1000 lines

**Approach**: Domain-driven refactoring (not blind splitting)

**Time Required**: 13 hours (~2 days focused work)

**Recommended Strategy**: Incremental (1 file per week)

**Expected Result**: Policy compliance + improved maintainability

---

**Ready to proceed with refactoring!** 🚀

**Start with**: `training.rs` refactoring (highest priority, 3 hours)

---

*"Smart refactoring: domain-driven, not line-driven.*  
*Preserve APIs, maintain tests, improve structure.*  
*From 2,676 lines to 5 logical files.*  
*From violation to compliance.*  
*This is evolution."* ✨
