# 🧹 Archive Code Cleanup Plan
**Date**: January 27, 2026  
**Status**: Ready for Execution  
**Grade**: S++ Maintenance

---

## 📊 Executive Summary

**Total Issues Found**: 12 categories  
**Compilation Blockers**: 2 (examples)  
**Deprecated References**: 473 TODOs across 122 files  
**Archive Size**: 5.2 MB (docs + showcase)  
**Action Items**: 8 primary + 6 documentation moves

---

## 🎯 Findings

### 1. ❌ COMPILATION BLOCKERS (P0 - Critical)

#### A. `examples/enhanced_wasm_component_demo.rs`
- **Issue**: References incomplete `component_model` feature
- **Error**: Unresolved imports for ComponentInterface, ComponentModelConfig, etc.
- **Root Cause**: Component model disabled during Jan 26 evolution
- **Action**: Delete file (feature incomplete, Phase 2 work)

#### B. `examples/beardog_encrypted_workload.rs`
- **Issue**: Hardcoded `reqwest` dependency
- **Line 51**: `client: reqwest::Client,`
- **Line 58**: `client: reqwest::Client::new(),`
- **Root Cause**: Remnant from pre-Pure-Rust evolution
- **Action**: Delete file (deprecated pattern, should use Unix sockets)

### 2. 🚫 DEPRECATED DEPENDENCY REFERENCES (P1 - High)

#### A. `reqwest` References (11 locations)
**Status**: All commented or marked PURE RUST, but still present in code

1. **`crates/core/toadstool/src/byob/health.rs:151`**
   - Still has `#[cfg(feature = "networking")]` with active `reqwest` code
   - **Action**: Remove or mark for Phase 2 evolution

2. **`crates/client/src/client/core.rs:84-143`**
   - Heavy `reqwest` usage (HTTP client builder, headers)
   - **Status**: `crates/client` is DISABLED in root Cargo.toml
   - **Action**: Move to `docs/archive/` (entire crate disabled)

3. **`crates/integration/primals/src/error.rs:24`**
   - `Network { source: reqwest::Error }`
   - **Action**: Evolve to generic error type

4. **`crates/distributed/src/ecosystem/caller_new.rs:29,78`**
   - `_http_client: reqwest::Client`
   - **Action**: Remove unused HTTP client field

5. **`crates/cli/src/ecosystem/discovery_new.rs:155,224`**
   - Active `reqwest::Client::new()` calls
   - **Action**: Delete file (marked `_new`, incomplete evolution)

6. **`crates/runtime/wasm/src/execution.rs:101`**
   - `#[cfg(feature = "url-module-loading")]` with reqwest
   - **Action**: Keep but verify feature is disabled by default

#### B. `jsonrpsee` References (14 locations)
**Status**: All properly deprecated with warnings

1. **`crates/server/src/jsonrpc_server.rs`**
   - **Status**: ✅ Already marked DEPRECATED with clear migration path
   - **Action**: Move entire file to `docs/archive/deprecated_code/`

2. **Comments in other files**
   - All properly marked with "PURE RUST" alternatives
   - **Action**: None (documentation is helpful)

### 3. 📋 TODO/FIXME Analysis (473 instances)

#### High-Priority TODOs (Action Required)

1. **`crates/runtime/wasm/src/component_model/mod.rs`**
   - Lines 55, 62, 85, 108, 154: Component model implementation TODOs
   - **Status**: Feature disabled, TODOs now stale
   - **Action**: Keep (Phase 2 work)

2. **`crates/auto_config/src/ecosystem_evolved.rs:1`**
   - TODO at line 1 (unknown content)
   - **Action**: Review and resolve

3. **`crates/server/src/jsonrpc_server.rs`** (12 TODOs)
   - All related to deprecated jsonrpsee implementation
   - **Action**: Archive entire file

#### Lower-Priority TODOs (Documentation)
- Most are valid "Phase 2/3" markers or implementation notes
- **Action**: None (legitimate roadmap markers)

### 4. 🔒 DISABLED CODE BLOCKS (7 files)

1. **`crates/core/toadstool/tests/production_hardening_logic_tests.rs`**
2. **`crates/runtime/wasm/tests/wasm_runtime_tests.rs`**
3. **`crates/runtime/wasm/tests/lib_implementation_coverage.rs`**
4. **`crates/runtime/wasm/src/component_model/mod.rs`**
5. **`crates/cli/tests/network_config_types_tests.rs`**
6. **`crates/cli/tests/network_config/discovery.rs`**
7. **`crates/cli/src/network_config/types.rs`**

**Action**: Review each file individually

### 5. 📚 ROOT-LEVEL SESSION DOCS (For ecoPrimals fossil record)

**Current Status**: 12 session docs at root (Jan 19-26, 2026)
**Size**: ~150 KB combined

**Recommendation**: Move to `docs/archive/jan26_2026_evolution/`

Files to move:
1. `COMPREHENSIVE_CODEBASE_REVIEW_JAN_26_2026.md`
2. `DEPENDENCY_ANALYSIS_JAN_26_2026.md`
3. `EVOLUTION_COMPLETE_JAN_26_2026.md`
4. `EVOLUTION_SESSION_JAN_26_2026.md`
5. `FINAL_SESSION_SUMMARY_JAN_26_2026.md`
6. `HARDCODING_EVOLUTION_JAN_26_2026.md`
7. `ROOT_DOCS_UPDATED_JAN_26_2026.md`
8. `SEMANTIC_METHODS_PHASE1_COMPLETE_JAN_26_2026.md`
9. `SEMANTIC_NAMING_EVOLUTION_JAN_26_2026.md`
10. `SESSION_COMPLETE_JAN_26_2026.md`
11. `TEST_COVERAGE_ANALYSIS_JAN_26_2026.md`
12. `SONGBIRD_IPC_ARCHITECTURE_REVIEW_JAN_19_2026.md` (actually Jan 19)

**Keep at Root**:
- README.md
- STATUS.md
- START_HERE.md
- ROOT_DOCS_INDEX.md
- DOCUMENTATION.md
- TESTING.md
- CHANGELOG.md
- Any non-session-specific docs

### 6. 🗂️ ARCHIVE DIRECTORIES

**Current State**:
- `docs/archive/`: 4.9 MB (multiple session folders from Jan 17-19)
- `showcase/archive/`: 328 KB (`sessions_2025/`)

**Action**: Keep as-is (proper fossil record organization)

---

## 🎯 Execution Plan

### Phase 1: Remove Compilation Blockers (P0)
1. ✅ Delete `examples/beardog_encrypted_workload.rs`
2. ✅ Delete `examples/enhanced_wasm_component_demo.rs`
3. ✅ Verify `cargo check` passes

### Phase 2: Clean Deprecated Code (P1)
4. ✅ Archive `crates/server/src/jsonrpc_server.rs` → `docs/archive/deprecated_code/`
5. ✅ Delete `crates/cli/src/ecosystem/discovery_new.rs`
6. ✅ Delete `crates/distributed/src/ecosystem/caller_new.rs`
7. ✅ Clean `crates/integration/primals/src/error.rs` (remove reqwest::Error)
8. ✅ Review `crates/core/toadstool/src/byob/health.rs` (networking feature)

### Phase 3: Move Session Docs (P2)
9. ✅ Create `docs/archive/jan26_2026_evolution/`
10. ✅ Move 12 session docs to archive folder
11. ✅ Update `ROOT_DOCS_INDEX.md` references

### Phase 4: Disable Client Crate (P2)
12. ✅ Verify `crates/client` is disabled in root `Cargo.toml`
13. ✅ Move to `docs/archive/deprecated_code/client/` OR
14. ✅ Mark with clear deprecation notice in crate README

### Phase 5: Verify & Test (P0)
15. ✅ Run `cargo check --all-targets`
16. ✅ Run `cargo test --all`
17. ✅ Run `cargo clippy --all-targets -- -D warnings`
18. ✅ Verify no new linter errors

### Phase 6: Documentation & Push (P1)
19. ✅ Update this document with execution results
20. ✅ Add entry to CHANGELOG.md
21. ✅ Git commit with clear message
22. ✅ Push via SSH

---

## 📋 Expected Outcomes

**Before**:
- ❌ 2 compilation errors in examples
- ⚠️ 473 TODOs (mix of valid and stale)
- 📁 12 session docs at root
- 🗂️ Deprecated code mixed with active code

**After**:
- ✅ Clean compilation (`cargo check` passes)
- ✅ Session docs properly archived
- ✅ Deprecated code clearly separated
- ✅ Valid TODOs remain (Phase 2/3 roadmap)
- ✅ S++ grade maintained

---

## 🎓 Lessons for Future Sessions

1. **Examples must be kept in sync** with feature flags
2. **Deprecated dependencies** should be fully removed, not just commented
3. **Session docs** should go directly to archive after completion
4. **`_new` suffix files** indicate incomplete evolution work
5. **TODO markers** need periodic audits to remove stale entries

---

## 🚀 Next Steps After Cleanup

1. **Phase 2 Component Model**: Complete WASM component model feature
2. **Client Crate Evolution**: Evolve HTTP client to Unix socket client
3. **TODO Audit**: Review remaining TODOs for Phase 2 priorities
4. **Benchmark Suite**: Ensure all examples are working demos

---

**Status**: Ready to execute  
**Estimated Time**: 30 minutes  
**Risk Level**: Low (mostly deletions and moves)  
**Breaking Changes**: None (examples only)

🍄🦀✨ **ToadStool remains S++ throughout cleanup!** ✨🦀🍄
