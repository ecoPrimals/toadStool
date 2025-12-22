# Error Module Refactoring - Implementation Status

**Started**: December 22, 2025  
**Status**: 🔄 **IN PROGRESS** (Phase 1)  
**Estimated Completion**: 2-3 hours total

---

## Implementation Phases

### ✅ Phase 0: Preparation (COMPLETE)
- [x] Created `error/` directory
- [x] Analyzed current `error.rs` structure (1,088 lines)
- [x] Mapped line boundaries for each logical section
- [x] Created detailed refactoring plan

### 🔄 Phase 1: Create Module Files (IN PROGRESS - 45 min)

**Tasks**:
1. [ ] Create `types.rs` (lines 33-325, ~290 lines)
   - All error enum definitions
   - Import statements
   - Module documentation

2. [ ] Create `conversions.rs` (lines 326-391, ~65 lines)
   - From<std::io::Error> impls
   - From<serde_json::Error> impls
   - Other standard conversions

3. [ ] Create `constructors.rs` (lines 392-549, ~160 lines)
   - Helper constructors for each error type
   - Ergonomic creation methods

4. [ ] Create `context.rs` (lines 550-768, ~210 lines)
   - ToadStoolError context methods
   - ToadStoolErrorExt trait

5. [ ] Create `extensions.rs` (lines 769-1088, ~330 lines)
   - ToadStoolErrorWithCode struct
   - Additional trait impls
   - Tests

6. [ ] Create `mod.rs` (~80 lines)
   - Module documentation
   - Re-exports for public API
   - Module declarations

### ⏸️ Phase 2: Integration (PENDING - 30 min)

**Tasks**:
1. [ ] Backup current `error.rs` as `error.rs.backup`
2. [ ] Replace `error.rs` with new `error/mod.rs` structure
3. [ ] Update internal `use` statements
4. [ ] Fix cross-module references

### ⏸️ Phase 3: Testing & Validation (PENDING - 30 min)

**Tasks**:
1. [ ] Run `cargo build` - ensure compilation
2. [ ] Run `cargo test` - ensure all tests pass
3. [ ] Run `cargo clippy` - check for warnings
4. [ ] Run `cargo doc` - verify documentation builds
5. [ ] Check file sizes (all should be < 400 lines)

### ⏸️ Phase 4: Documentation (PENDING - 30 min)

**Tasks**:
1. [ ] Update module-level documentation
2. [ ] Add cross-references between modules
3. [ ] Update CHANGELOG
4. [ ] Update evolution progress docs

---

## File Boundaries (Precise Line Numbers)

### types.rs (~290 lines)
```
Source: error.rs lines 33-325
Content:
  - Line 33-36: Imports (use statements)
  - Line 38-75: ToadStoolError enum
  - Line 77-118: ExecutionError enum
  - Line 120-157: ConfigError enum
  - Line 159-193: ResourceError enum
  - Line 195-225: IntegrationError enum
  - Line 227-257: SecurityError enum
  - Line 259-292: NetworkError enum
  - Line 294-324: SystemError enum
  - Line 326-355: Result type aliases
```

### conversions.rs (~65 lines)
```
Source: error.rs lines 326-391
Content:
  - Line 358-365: From<std::io::Error> for ToadStoolError
  - Line 367-374: From<serde_json::Error> for ToadStoolError
  - Line 376-382: From<std::io::Error> for SystemError
  - Line 384-391: From<serde_json::Error> for SystemError
```

### constructors.rs (~160 lines)
```
Source: error.rs lines 392-549
Content:
  - Line 396-425: ExecutionError constructors
  - Line 427-446: ConfigError constructors
  - Line 448-469: ResourceError constructors
  - Line 471-487: IntegrationError constructors
  - Line 489-505: SecurityError constructors
  - Line 507-523: NetworkError constructors
  - Line 525-549: SystemError constructors
```

### context.rs (~210 lines)
```
Source: error.rs lines 550-768
Content:
  - Line 554-768: ToadStoolError impl (context methods)
  - Line 769-778: ToadStoolErrorExt trait
```

### extensions.rs (~330 lines)
```
Source: error.rs lines 769-1088
Content:
  - Line 780-783: ToadStoolErrorWithCode struct
  - Line 785-835: ToadStoolErrorWithCode impls
  - Line 837-1088: Tests
```

### mod.rs (~80 lines)
```
New file - module coordination
Content:
  - Module documentation (from lines 1-32 of error.rs)
  - Module declarations
  - Public re-exports
  - Ensure all public APIs preserved
```

---

## Key Principles

1. **Preserve Public API**: All public types, traits, and functions must remain accessible
2. **No Breaking Changes**: Existing code using the error module should work unchanged
3. **Logical Organization**: Group by function, not arbitrary lines
4. **Clear Documentation**: Each module should explain its purpose
5. **Test Preservation**: All existing tests must continue to work

---

## Success Criteria

- [ ] All files < 400 lines ✅
- [ ] `cargo build` succeeds ✅
- [ ] `cargo test` all tests pass ✅
- [ ] `cargo clippy` no new warnings ✅
- [ ] `cargo doc` builds successfully ✅
- [ ] No breaking changes to public API ✅
- [ ] Code organization more discoverable ✅

---

## Risk Mitigation

### Backup Strategy
```bash
# Before starting migration
cp crates/core/common/src/error.rs crates/core/common/src/error.rs.backup
```

### Rollback Plan
If any issues occur:
```bash
# Restore backup
rm -rf crates/core/common/src/error/
mv crates/core/common/src/error.rs.backup crates/core/common/src/error.rs
```

### Testing Strategy
After each phase:
1. Run quick compilation check
2. Run subset of tests
3. Only proceed if passing

---

## Next Actions

### Immediate (Continue Now):
1. Create `types.rs` with error enums
2. Create `conversions.rs` with From impls
3. Create `constructors.rs` with helpers
4. Create `context.rs` with ToadStoolError impl
5. Create `extensions.rs` with extras
6. Create `mod.rs` with re-exports

### Then:
1. Backup original file
2. Replace with new structure
3. Test compilation
4. Fix any import issues
5. Validate all tests pass

---

## Time Tracking

- **Preparation**: 30 min ✅ (Complete)
- **File Creation**: 45 min 🔄 (In Progress - 0% complete)
- **Integration**: 30 min ⏸️ (Pending)
- **Testing**: 30 min ⏸️ (Pending)
- **Documentation**: 30 min ⏸️ (Pending)
- **Total**: ~2.5 hours (est. remaining: ~2 hours)

---

**Status**: Ready to create module files systematically

**Next Step**: Create `types.rs` with all error enum definitions

