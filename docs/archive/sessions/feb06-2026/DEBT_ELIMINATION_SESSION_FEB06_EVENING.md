# Deep Debt Elimination Session — Feb 06, 2026 (Evening)

**Start Time**: Feb 06, 2026, Evening  
**Focus**: Phase 1 - Test Infrastructure (135 errors → 0)  
**Principles**: Deep Debt Solutions + Modern Idiomatic Rust

---

## 🎯 Deep Debt Principles Applied

1. **Modern Idiomatic Rust** - Use Tensor method API, not free functions
2. **External Dependencies** - Already 100% Rust-native ✅
3. **Smart Refactoring** - Fix systematically by category
4. **Unsafe Evolution** - Already 100% safe ✅  
5. **Capability-Based** - Continue capability evolution after tests
6. **Primal Self-Knowledge** - ToadStool knows only itself ✅
7. **Mocks Isolated** - Will fix production mocks in Phase 2

---

## 📊 Starting State

**Test Compilation**:
- Main library: ✅ CLEAN (0 errors, 0 warnings)
- Test suite: ⚠️ 135 compilation errors

**Error Breakdown**:
- 74x E0282: Type annotations needed (cascading)
- 22x E0425: Cannot find function (API mismatch)
- 16x E0277: Not a future (incorrect async/await)
- 8x E0308: Mismatched types
- 6x E0061: Wrong number of arguments
- 3x E0432: Unresolved import
- 2x E0382: Ownership issues
- 1x E0599: No method found

---

## 🔍 Root Cause: API Mismatch

**The Issue**:
- Tests call operations as FREE FUNCTIONS: `grid_mask(&device, &queue, &data, ...)`
- Implementation uses TENSOR METHODS: `tensor.grid_mask(...)`

**Affected Operations** (E0425 errors):
- `grid_mask` (9 occurrences)
- `mosaic` (7 occurrences)
- `soft_nms` (4 occurrences)
- `random_affine` (1 occurrence)
- `random_perspective` (1 occurrence)

**Architectural Decision** (following Deep Debt Principles):
- ✅ **Option A**: Update tests to use Tensor method API (modern, idiomatic)
- ❌ Option B: Create free function wrappers (quick fix, not idiomatic)

**Rationale**: Modern idiomatic Rust uses method APIs on types. The Tensor API is correct; tests need to evolve.

---

## 🚀 Execution Plan

### **Wave 1: Fix API Mismatch (E0432, E0425)** - 25 errors

**Strategy**: Update tests to use Tensor method API

**Pattern**:
```rust
// OLD (wrong):
let result = operation(&device, &queue, &data, args...);

// NEW (correct, idiomatic):
let tensor = Tensor::from_vec(data, shape, device).await?;
let result = tensor.operation(args...)?;
let output = result.to_vec()?;
```

**Files to Fix**:
1. `grid_mask.rs` test module (9 errors)
2. `mosaic.rs` test module (7 errors)
3. `soft_nms.rs` test module (4 errors)
4. `random_affine.rs` test module (1 error)
5. `random_perspective.rs` test module (1 error)
6. E0432 unresolved imports (3 errors)

**Estimated**: 2-3 hours

---

### **Wave 2: Fix Async/Await (E0277)** - 16 errors

**Issue**: Methods like `to_vec()` became synchronous, but tests still use `.await`

**Pattern**:
```rust
// OLD (wrong):
let vec = tensor.to_vec().await?;

// NEW (correct):
let vec = tensor.to_vec()?;  // No .await
```

**Estimated**: 1 hour

---

### **Wave 3: Fix API Signatures (E0061, E0308)** - 14 errors

**Issues**:
- E0061: Wrong number of arguments (6 errors)
- E0308: Type mismatches (8 errors)

**Patterns**:
```rust
// Wrong arg count - check current API signature
// Type mismatch - usually u32 vs f32 or similar
```

**Estimated**: 1 hour

---

### **Wave 4: Fix Ownership (E0382)** - 2 errors

**Issue**: Value moved, use after move

**Pattern**:
```rust
// Add .clone() or restructure to avoid move
```

**Estimated**: 30 minutes

---

### **Wave 5: Type Annotations (E0282)** - Auto-resolve

**Issue**: Most are cascading from earlier errors

**Expected**: Auto-resolve as we fix root causes

**If not**: Add explicit type annotations

**Estimated**: 30 minutes

---

## ⏱️ Total Estimated Effort

| Wave | Errors | Effort |
|------|--------|--------|
| Wave 1 (API Mismatch) | 25 | 2-3h |
| Wave 2 (Async/Await) | 16 | 1h |
| Wave 3 (API Signatures) | 14 | 1h |
| Wave 4 (Ownership) | 2 | 30min |
| Wave 5 (Type Annotations) | 74 | 30min |

**Total**: 5-8 hours (as estimated)

---

## 📝 Session Progress

### Wave 1: API Mismatch Fixes

**Starting**: 135 errors

#### Fix 1: grid_mask.rs test
- [ ] Update test to use Tensor API
- [ ] Verify compilation

#### Fix 2: mosaic.rs test  
- [ ] Update test to use Tensor API
- [ ] Verify compilation

#### Fix 3: soft_nms.rs test
- [ ] Update test to use Tensor API
- [ ] Verify compilation

#### Fix 4: random_affine.rs test
- [ ] Update test to use Tensor API
- [ ] Verify compilation

#### Fix 5: random_perspective.rs test
- [ ] Update test to use Tensor API
- [ ] Verify compilation

#### Fix 6: E0432 unresolved imports
- [ ] Add missing imports
- [ ] Verify compilation

**Wave 1 Target**: 135 → ~110 errors (25 fixed)

---

## 🎯 Success Criteria

**Phase 1 Complete**:
- ✅ cargo test --workspace --no-run (0 compilation errors)
- ✅ cargo test --workspace (tests run and pass)
- ✅ All tests use idiomatic Tensor API
- ✅ No free function wrappers (clean, modern code)

**Then**:
- Phase 2: Production Mocks (7 mocks → 0)
- Phase 3: Large File Refactoring (9 files)
- Phase 4: Capability Evolution (295 ops)

---

**Session Status**: IN PROGRESS - Starting Wave 1
