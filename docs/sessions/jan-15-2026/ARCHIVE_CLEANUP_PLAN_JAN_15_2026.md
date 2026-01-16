# Archive Cleanup Plan - January 15, 2026

**Date**: January 15, 2026  
**Purpose**: Clean outdated code while keeping docs as fossil record  
**Status**: Ready for review and execution

---

## 🎯 Summary

**Found**:
- 1,588 TODO/FIXME/DEPRECATED references across 284 files
- 262 `#[allow(dead_code)]` attributes across 110 files
- 3 empty generated files in target/ (build artifacts)
- 0 backup files (.bak, .old, ~)

**Keep**: All documentation (fossil record)  
**Clean**: Outdated code TODOs, dead code attributes where safe

---

## 📊 Analysis

### TODOs Breakdown

**Total**: 1,588 references across 284 files

**By Location**:
- **Documentation** (~1,400): Keep as fossil record ✅
- **Code files** (~188): Review for cleanup

**Code TODOs** (highest priority for cleanup):

1. **Test Files** (~100 TODOs):
   - Placeholder test implementations
   - `tests/error_paths_discovery_tests.rs` (4 TODOs - line 255-278)
   - Many marked as "TODO: Implementation" for error paths

2. **Examples** (~30 TODOs):
   - `examples/real_gpu_pool.rs`: "TODO: GPU resource auto-detection"
   - `examples/gpu_cpu_pool_demo.rs`: "TODO: Register GPU resource"
   - Future feature placeholders

3. **Core Code** (~58 TODOs):
   - Integration placeholders
   - Future feature markers
   - Some outdated (completed but not removed)

### Dead Code Attributes

**Total**: 262 `#[allow(dead_code)]` across 110 files

**Categories**:

1. **Intentional** (keep):
   - GPU backends for future hardware
   - Framework adapters not yet integrated
   - Test utilities and fixtures

2. **Can Review** (~50):
   - Fields marked dead in completed features
   - Old integration stubs
   - Placeholder structs

---

## 🧹 Cleanup Actions

### Action 1: Review Code TODOs (Priority: High)

**Target**: ~188 code TODOs

**Categories**:

#### 1.1 Completed TODOs (Remove)

**Search for**:
```bash
# TODOs for features that are now complete
grep -r "TODO.*implement" crates/ --include="*.rs" | \
  grep -i "beardog\|songbird\|tarpc\|jsonrpc"
```

**Example** (likely completed):
```rust
// TODO: Implement tarpc integration
// ^ If tarpc is working, remove this
```

**Action**: Remove TODO comment if feature is complete

#### 1.2 Future TODOs (Update)

**Search for**:
```bash
# TODOs without context
grep -r "^[[:space:]]*//[[:space:]]*TODO:" crates/ --include="*.rs"
```

**Current**:
```rust
// TODO: Implement feature X
```

**Update to**:
```rust
// TODO(future): Implement feature X when Y is ready
```

**Action**: Add context about when/why deferred

#### 1.3 Test Placeholder TODOs (Document)

**File**: `tests/error_paths_discovery_tests.rs`

**Current** (lines 255-278):
```rust
// TODO: Implementation with corrupted cache data
// TODO: Implementation with unresolvable hostname
// TODO: Implementation with invalid certificate
// Each TODO represents a specific error path that needs implementation
```

**Action**: Keep but add tracking comment:
```rust
// TODO(test-coverage): Implement error path tests for:
// - Corrupted cache data recovery
// - Unresolvable hostname handling
// - Invalid certificate validation
// See TEST_COVERAGE_ANALYSIS_JAN_15_2026.md for priority
```

---

### Action 2: Review Dead Code Attributes (Priority: Medium)

**Target**: ~262 `#[allow(dead_code)]` across 110 files

#### 2.1 GPU Backend Fields (Keep)

**Example**:
```rust
pub struct VulkanBackend {
    #[allow(dead_code)]
    device: Arc<Device>, // Needed for future Vulkan ops
}
```

**Reason**: Future hardware support  
**Action**: Keep as-is

#### 2.2 Completed Features (Remove if Safe)

**Search for**:
```bash
# Dead code in completed modules
rg "#\[allow\(dead_code\)\]" crates/integration/ -A 2
```

**Example**:
```rust
#[allow(dead_code)]
pub struct OldIntegrationStub {
    // If this is replaced by new impl, remove
}
```

**Action**: Remove if:
1. Feature is complete
2. No longer referenced
3. Tests pass without it

#### 2.3 Test Fixtures (Keep)

**Example**:
```rust
#[allow(dead_code)]
fn test_helper_function() {
    // Used by multiple tests
}
```

**Reason**: Test infrastructure  
**Action**: Keep as-is

---

### Action 3: Clean Build Artifacts (Priority: Low)

**Found**: 3 empty `.rs` files in `target/`

```
./target/debug/build/mime_guess-*/out/mime_types_generated.rs
./target/llvm-cov-target/debug/build/mime_guess-*/out/mime_types_generated.rs
```

**Action**: None needed (cargo clean handles these)

---

### Action 4: No Backup Files (Priority: N/A)

**Found**: 0 backup files (`.bak`, `.old`, `~`, `.swp`)

**Action**: None needed ✅

---

## 📋 Execution Plan

### Phase 1: Code TODOs (Low Risk)

**Time**: ~30 minutes

1. **Update Future TODOs** (non-breaking):
   ```bash
   # Find TODOs without context
   rg "^[[:space:]]*//[[:space:]]*TODO:" crates/ --include="*.rs"
   
   # Update format:
   # TODO: X  →  TODO(future): X when Y ready
   ```

2. **Remove Completed TODOs**:
   ```bash
   # Find beardog/tarpc integration TODOs
   rg "TODO.*beardog|TODO.*tarpc" crates/ --include="*.rs"
   
   # If feature works, remove TODO
   ```

3. **Document Test TODOs**:
   ```bash
   # Update tests/error_paths_discovery_tests.rs
   # Add tracking comment with context
   ```

### Phase 2: Dead Code Review (Medium Risk)

**Time**: ~1 hour

1. **Identify Safe Removals**:
   ```bash
   # Find dead code in completed modules
   rg "#\[allow\(dead_code\)\]" crates/integration/ -A 5
   
   # Check if:
   # - Module is complete
   # - No references exist
   # - Tests pass without it
   ```

2. **Remove Safe Dead Code**:
   ```bash
   # Remove attribute + unused code
   # Run: cargo test --workspace
   # Verify: no breakage
   ```

3. **Document Kept Dead Code**:
   ```bash
   # Add comment explaining why kept
   #[allow(dead_code)] // Future GPU backend support
   ```

### Phase 3: Validation (Critical)

**Time**: ~15 minutes

1. **Build Check**:
   ```bash
   cargo check --workspace
   # Must: Pass cleanly
   ```

2. **Test Check**:
   ```bash
   cargo test --workspace --lib
   # Must: All tests pass (same count as before)
   ```

3. **Clippy Check**:
   ```bash
   cargo clippy --workspace
   # Must: No new warnings
   ```

---

## 🎯 Specific Files to Review

### High Priority (Code TODOs)

1. **`tests/error_paths_discovery_tests.rs`**:
   - Lines 255-278 (4 TODOs)
   - Action: Add tracking comment

2. **`examples/real_gpu_pool.rs`**:
   - Line 71: "TODO: GPU resource auto-detection"
   - Action: Update to TODO(future) with context

3. **`examples/gpu_cpu_pool_demo.rs`**:
   - Line 65: "TODO: Register GPU resource"
   - Action: Update to TODO(future) with context

4. **`crates/distributed/src/songbird_integration/types.rs`**:
   - 12 TODOs found
   - Action: Review if songbird integration is complete

5. **`crates/cli/src/ecosystem/discovery.rs`**:
   - 6 TODOs found
   - Action: Review ecosystem discovery status

### Medium Priority (Dead Code)

1. **`crates/runtime/gpu/src/unified_memory/backends/*.rs`**:
   - Multiple `#[allow(dead_code)]` for GPU fields
   - Action: Keep (future hardware support)

2. **`crates/distributed/src/security_provider/beardog_impl/*.rs`**:
   - Dead code in beardog integration
   - Action: Review if integration is complete

3. **`crates/cli/src/executor/*.rs`**:
   - Dead code in executor implementations
   - Action: Review if features are complete

---

## ⚠️ Safety Rules

### ALWAYS Keep

1. **Documentation**: All .md files (fossil record)
2. **Test Fixtures**: Test helper code
3. **GPU Backends**: Future hardware support code
4. **Framework Adapters**: Future integration code
5. **Error Paths**: Even if not implemented, keep TODO

### NEVER Remove Without Testing

1. **Integration Code**: May be used indirectly
2. **Dead Code in Libraries**: May be public API
3. **GPU/Runtime Code**: May be selected at runtime

### Safe to Clean

1. **Build Artifacts**: cargo clean handles
2. **Backup Files**: If found (none currently)
3. **Completed TODOs**: If feature demonstrably works
4. **Old Stubs**: If replaced and unreferenced

---

## 📊 Expected Impact

### Before Cleanup

- **TODOs**: 1,588 (mostly in docs)
- **Dead Code**: 262 attributes
- **Code Quality**: A- (87/100)

### After Cleanup (Estimated)

- **TODOs**: ~1,550 (docs kept, ~38 code cleaned)
- **Dead Code**: ~240 (safely remove ~22)
- **Code Quality**: A- (87/100, maintain grade)

**Net Improvement**:
- Clearer future work tracking
- Less noise for developers
- Better TODO context
- Same functionality (zero breaking changes)

---

## 🚀 Execution Timeline

**Total Time**: ~2 hours

| Phase | Duration | Risk | Validation |
|-------|----------|------|------------|
| **Phase 1: TODOs** | 30 min | Low | cargo check |
| **Phase 2: Dead Code** | 60 min | Medium | cargo test |
| **Phase 3: Validation** | 15 min | Critical | Full test suite |
| **Phase 4: Commit** | 15 min | Low | Git status |

---

## 📝 Commit Plan

### Commit 1: Update Future TODOs (Low Risk)

```bash
git add -p  # Review each change
git commit -m "chore: clarify future TODO context

Add context to TODO comments explaining when/why deferred:
- TODO(future): marker for post-1.0 features
- TODO(test-coverage): marker for test expansion
- Improves developer clarity without changing functionality

Changed: ~30 TODO comments across crates/
Impact: Documentation only, zero functional changes
"
```

### Commit 2: Remove Completed TODOs (Low Risk)

```bash
git add -p
git commit -m "chore: remove completed TODO comments

Remove TODO markers for completed features:
- beardog integration (complete)
- tarpc integration (complete)
- jsonrpc integration (complete)

Changed: ~8 TODO comments removed
Validation: cargo test --workspace (all passing)
"
```

### Commit 3: Clean Safe Dead Code (Medium Risk)

```bash
git add -p
git commit -m "chore: remove safe dead code attributes

Remove #[allow(dead_code)] from completed features:
- Verified no references exist
- All tests passing
- No clippy warnings added

Changed: ~20 dead code attributes
Validation: cargo test + cargo clippy (all clean)
"
```

---

## ✅ Success Criteria

**Must Achieve**:
1. ✅ `cargo check --workspace` passes
2. ✅ `cargo test --workspace --lib` passes (same test count)
3. ✅ `cargo clippy --workspace` no new warnings
4. ✅ Zero functional changes
5. ✅ All docs preserved (fossil record)

**Nice to Have**:
- Clearer TODO comments (~30 updated)
- Fewer noise TODOs (~8 removed)
- Cleaner dead code (~20 removed)

---

## 📞 Review Required

**Before Executing**:

1. **Review TODO List**: 
   - Are listed TODOs actually complete?
   - Should they be removed or updated?

2. **Review Dead Code**:
   - Is it safe to remove?
   - Are there hidden references?

3. **Test Strategy**:
   - Run full test suite before/after
   - Compare test counts
   - Check for new failures

---

## 🎯 Recommendation

**Proceed with Phase 1 (TODOs) only for now**:
- Low risk (documentation changes)
- High value (clearer context)
- Fast execution (~30 minutes)
- Easy to validate

**Defer Phase 2 (Dead Code)** to future session:
- Needs more careful review
- Medium risk (code changes)
- Can be done separately

---

**Status**: ✅ **READY FOR REVIEW**  
**Risk**: Low (Phase 1), Medium (Phase 2)  
**Time**: 30 min (Phase 1), 2 hours (both)  
**Value**: Clearer codebase, better developer experience

---

**Created**: January 15, 2026  
**Purpose**: Archive cleanup while preserving fossil record  
**Next**: Review and execute Phase 1 (TODO updates)
