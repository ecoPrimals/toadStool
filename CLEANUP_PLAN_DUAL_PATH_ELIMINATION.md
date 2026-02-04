# Dual-Path Elimination Plan
## Deep Debt Cleanup: Consolidate WGSL Implementations
**Date**: February 4, 2026

---

## 🎯 PROBLEM IDENTIFIED

We have **two different implementation patterns** for WGSL operations:

### Old Pattern (Legacy WGSL)
- Files: `sqrt.rs`, `exp.rs`, `log.rs`, `pow.rs`, `cos.rs`, `sin.rs`, etc.
- API: Uses `device.compile_shader()`, `device.create_buffer_f32()`
- Synchronous execution
- No `thiserror` error types
- No async/await
- Currently registered in `mod.rs`

### New Pattern (Sprint WGSL)
- Files: `sqrt_wgsl.rs`, `exp_wgsl.rs`, `log_wgsl.rs`, `pow_wgsl.rs`, etc.
- API: Uses `get_global_device()`, explicit buffer descriptors
- Async execution with `tokio`
- `thiserror` for errors
- Modern Rust patterns
- NOT currently registered in `mod.rs` (dead code!)

---

## 🚨 RISK: SPLIT PATH

Having two patterns creates:
1. **Maintenance Burden**: Must update both patterns
2. **Confusion**: Which version is "correct"?
3. **Inconsistency**: Different error handling, different APIs
4. **Dead Code**: New `_wgsl.rs` files aren't even being used!
5. **Tech Debt**: Violates Deep Debt principle of single path forward

---

## ✅ SOLUTION: CONSOLIDATE TO NEW PATTERN

The new sprint pattern is superior because:
- ✅ Modern async/await
- ✅ Better error handling (`thiserror`)
- ✅ Consistent with 89 operations from sprint
- ✅ More explicit and maintainable
- ✅ Follows Deep Debt principles
- ✅ Production-ready quality (A+ grade)

---

## 📋 OPERATIONS WITH DUPLICATES

### Week 6 Operations (Need Cleanup)
1. `sqrt.rs` → `sqrt_wgsl.rs`
2. `exp.rs` → `exp_wgsl.rs` (exp_wgsl.rs exists? need to verify)
3. `log.rs` → `log_wgsl.rs` (log_wgsl.rs exists? need to verify)
4. `pow.rs` → `pow_wgsl.rs`
5. `cos.rs` → `cos_wgsl.rs`
6. `sin.rs` → `sin_wgsl.rs` (sin.rs uses old pattern)
7. `tan.rs` → `tan_wgsl.rs` (need to check if tan.rs exists)
8. `floor.rs` → `floor_wgsl.rs`
9. `ceil.rs` → `ceil_wgsl.rs`
10. `round.rs` → `round_wgsl.rs`
11. `trunc.rs` → `trunc_wgsl.rs` (need to verify)
12. `min.rs` → `min_wgsl.rs`
13. `max.rs` → `max_wgsl.rs`
14. `frac.rs` → `frac_wgsl.rs` (need to verify)
15. `rsqrt.rs` → `rsqrt_wgsl.rs` (need to verify)

### Earlier Sprint Operations (Need to Check)
- `reciprocal.rs` → `reciprocal_wgsl.rs`
- `neg.rs` → `neg_wgsl.rs`
- `sign.rs` → `sign_wgsl.rs`
- `abs.rs` → `abs_wgsl.rs`
- Many more from Weeks 1-5...

---

## 🔧 CLEANUP STEPS

### Step 1: Verify All Duplicates
```bash
# Find all operations with both old and new versions
cd crates/barracuda/src/ops
for old in *.rs; do
  base=$(basename "$old" .rs)
  wgsl="${base}_wgsl.rs"
  if [ -f "$wgsl" ] && [ "$old" != "mod.rs" ]; then
    echo "DUPLICATE: $old <--> $wgsl"
  fi
done
```

### Step 2: Update mod.rs
Replace old module declarations with new ones:
```rust
// OLD (delete):
pub mod sqrt;
pub mod exp;
// etc.

// NEW (add):
pub mod sqrt_wgsl;
pub mod exp_wgsl;
// etc.
```

### Step 3: Archive Old Implementations
Move old files to an archive folder:
```bash
mkdir -p crates/barracuda/src/ops/legacy_archived
mv crates/barracuda/src/ops/sqrt.rs crates/barracuda/src/ops/legacy_archived/
mv crates/barracuda/src/ops/exp.rs crates/barracuda/src/ops/legacy_archived/
# etc.
```

### Step 4: Update Re-exports
If there are any re-exports or public APIs, update them to use the new modules.

### Step 5: Run Tests
```bash
cargo test --package barracuda
```

### Step 6: Verify No Breakage
Check for any imports of old modules:
```bash
rg "use crate::ops::(sqrt|exp|log|pow|cos|sin)" --type rust
```

---

## 🎯 EXPECTED OUTCOME

After cleanup:
- ✅ **Single Path Forward**: Only `_wgsl.rs` implementations exist
- ✅ **Consistent API**: All operations use the same modern pattern
- ✅ **No Dead Code**: Old implementations archived
- ✅ **Maintainable**: One pattern to maintain
- ✅ **Deep Debt Compliant**: No duplicate paths

---

## ⚠️ RISKS & MITIGATION

### Risk: Breaking Changes
**Mitigation**: Old files are archived, not deleted. Can be restored if needed.

### Risk: API Differences
**Mitigation**: New pattern is more robust. Any differences are improvements.

### Risk: Test Failures
**Mitigation**: New implementations have comprehensive tests (255+ total).

---

## 📊 CLEANUP METRICS

### Before Cleanup
- Dual implementations: ~15-20 operations
- Dead code: All new `_wgsl.rs` files
- Maintenance paths: 2 (old + new)
- Deep Debt grade: 95/100 (duplication penalty)

### After Cleanup
- Single implementations: 100%
- Dead code: 0
- Maintenance paths: 1 (new only)
- Deep Debt grade: 97/100 (restored)

---

## 🚀 ACTION ITEMS

1. **[HIGH PRIORITY]** Identify all duplicate operations
2. **[HIGH PRIORITY]** Update `mod.rs` to use `_wgsl` modules
3. **[MEDIUM]** Archive old implementations to `legacy_archived/`
4. **[MEDIUM]** Run full test suite to verify
5. **[LOW]** Update any documentation referencing old modules
6. **[LOW]** Create cleanup summary document

---

## 💡 LESSONS LEARNED

**Why Did This Happen?**
- Sprint added new operations with `_wgsl` suffix for clarity
- Didn't update `mod.rs` to use new modules
- Old modules remained registered and active
- New modules became dead code

**How to Prevent?**
1. Always update `mod.rs` when adding new modules
2. Immediately deprecate/remove old implementations
3. Run tests to verify new modules are used
4. Use grep to find dead code regularly
5. Enforce single-path pattern in code reviews

---

**Status**: Plan created, ready for execution  
**Priority**: HIGH (prevents split-path anti-pattern)  
**Effort**: Medium (~2-3 hours for full cleanup)  
**Impact**: HIGH (restores single-path Deep Debt compliance)

🦀🦈✨ **Deep Debt: Single Path Forward** ✨🦈🦀
