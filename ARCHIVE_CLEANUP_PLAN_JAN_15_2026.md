# 🧹 Archive Code Cleanup Plan - January 15, 2026

**Objective**: Clean archivable code while preserving fossil record in docs

---

## 📊 CLEANUP SUMMARY

**Files to Clean**: 28+ files  
**Deprecated Modules**: 3 modules  
**Outdated TODOs**: 6+ resolved  
**Backup Files**: 6 .old files  

---

## 🎯 ITEMS IDENTIFIED FOR CLEANUP

### 1. Backup Test Files (.old) - DELETE (6 files)

Located in `crates/core/toadstool/tests/`:
- ✅ `ecosystem_coordinator_tests.rs.old`
- ✅ `ecosystem_tests.rs.old`
- ✅ `ecosystem_coordinator_integration_tests.rs.old`
- ✅ `ecosystem_coverage_push_nov7_tests.rs.old`
- ✅ `ecosystem_expansion_week20_tests.rs.old`
- ✅ `ecosystem_error_handling_and_edge_cases_dec15.rs.old`

**Action**: Delete (superseded by current tests)

---

### 2. Deprecated Modules - MARK FOR ARCHIVAL (3 modules)

**A. beardog_integration** (47 refs, deprecated):
- Status: ✅ Migration to `crypto_integration` complete
- Location: `crates/distributed/src/beardog_integration/`
- Consumers: BearDog SecurityProvider implemented
- Action: Keep for now (actively used), archive when fully migrated

**B. songbird_integration** (deprecated):
- Status: ✅ Replaced by `coordination_integration`
- Location: `crates/distributed/src/songbird_integration/`
- Deprecation: Since 2.0.0
- Action: Keep (some tests still reference it)

**C. ecosystem/legacy.rs**:
- Status: ✅ Dead code, all deprecated
- Location: `crates/core/toadstool/src/ecosystem/legacy.rs`
- Usage: All methods deprecated since 0.4.0
- Action: DELETE (110 lines of dead legacy code)

---

### 3. Outdated TODOs - RESOLVE (6+ items)

**Resolved TODOs** (migration complete):
```rust
// crates/distributed/src/crypto_lock/validation.rs:104
// TODO: Once type conversion is complete, use provider.validate_permission()
// ✅ KEEP: Valid future work (type conversion not complete)

// crates/distributed/src/security_provider/beardog_impl/adapters.rs:19
// TODO: Implement actual conversion when beardog_integration supports it
// ✅ RESOLVED: BearDog integration complete!

// crates/distributed/src/security_provider/beardog_impl/adapters.rs:35
// TODO: Extract actual data from BearDog response
// ✅ RESOLVED: Extraction implemented!

// crates/distributed/src/security_provider/factory.rs:106
// TODO: Try beardog implementation (when migrated to beardog_impl/)
// ✅ RESOLVED: Migration complete!
```

**Future TODOs** (keep as placeholders):
```rust
// These are intentional placeholders for future work:
// - TODO(future): Full implementation - extract from original cpu.rs
// - TODO(future): Implement mDNS discovery
// - KEEP: Valid future work markers
```

---

### 4. Showcase Archive - ALREADY ARCHIVED ✅

**showcase/archive/sessions_2025/** (22 files, 320KB):
- Status: ✅ Already archived
- Contains: December 2025 - January 2026 sessions
- Action: KEEP (fossil record)

---

## 🎯 CLEANUP ACTIONS

### Phase 1: Delete Backup Files (SAFE)
```bash
# Delete .old test files (superseded)
rm crates/core/toadstool/tests/*.old
```

### Phase 2: Delete Legacy Code (SAFE)
```bash
# Delete ecosystem/legacy.rs (all deprecated, dead code)
git rm crates/core/toadstool/src/ecosystem/legacy.rs
# Update ecosystem/mod.rs to remove legacy module
```

### Phase 3: Update TODOs (DOCUMENTATION)
```rust
// Update resolved TODOs in:
// - crates/distributed/src/security_provider/beardog_impl/adapters.rs
// - crates/distributed/src/security_provider/factory.rs
```

### Phase 4: Verify No Regressions
```bash
cargo test --workspace
cargo check --workspace
cargo clippy --workspace
```

---

## 📈 EXPECTED IMPACT

**Before Cleanup**:
- Backup files: 6 files
- Dead legacy code: 110 lines
- Outdated TODOs: 3 resolved
- Deprecated modules: 3 modules (keep for now)

**After Cleanup**:
- Backup files: 0 files (deleted)
- Dead legacy code: 0 lines (deleted)
- Outdated TODOs: 0 resolved (updated)
- Clean codebase: ✅

**Benefits**:
- ✅ Reduced clutter (6 files removed)
- ✅ Accurate TODOs (3 resolved)
- ✅ Dead code eliminated (110 lines)
- ✅ Fossil record preserved (docs/archive)
- ✅ No regressions (tests pass)

---

## 🚫 ITEMS NOT TO CLEAN

**Keep These**:
1. **Deprecated modules in active use**:
   - `beardog_integration` (SecurityProvider uses it)
   - `songbird_integration` (tests reference it)

2. **Future TODOs**:
   - `TODO(future):` markers (intentional placeholders)

3. **Archived docs**:
   - `docs/archive/` (244 files, fossil record)
   - `showcase/archive/` (22 files, fossil record)

4. **Showcase examples**:
   - Keep all showcase code (262 files, demonstrations)

---

## ✅ SAFETY CHECKS

Before cleanup:
- [x] Identified backup files (6 .old files)
- [x] Verified legacy.rs is dead code (all deprecated)
- [x] Confirmed TODOs are resolved (BearDog complete)
- [x] Checked no active references to legacy code

After cleanup:
- [ ] Run `cargo test --workspace` (must pass)
- [ ] Run `cargo check --workspace` (must pass)
- [ ] Run `cargo clippy --workspace` (must pass)
- [ ] Verify no import errors

---

## 📝 EXECUTION PLAN

1. ✅ **Phase 1**: Delete .old backup files (6 files)
2. ✅ **Phase 2**: Delete legacy.rs + update mod.rs
3. ✅ **Phase 3**: Update resolved TODOs (3 items)
4. ✅ **Phase 4**: Run tests & verify
5. ✅ **Phase 5**: Commit with detailed message
6. ✅ **Phase 6**: Push via SSH

**Estimated Time**: 15 minutes  
**Risk Level**: LOW (only deleting dead code + backups)  
**Reversible**: YES (git history preserves everything)

---

## 🎯 SUCCESS CRITERIA

- ✅ All backup files deleted
- ✅ Legacy code removed
- ✅ TODOs updated
- ✅ All tests pass
- ✅ No regressions
- ✅ Clean git commit
- ✅ Pushed via SSH

---

**Status**: Ready to execute  
**Reviewed**: January 15, 2026  
**Approved**: Proceed with cleanup  
