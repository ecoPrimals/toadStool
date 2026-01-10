# 🧹 Code & Documentation Cleanup Plan

**Date**: January 10, 2026  
**Purpose**: Clean archive code, outdated TODOs, and false positives  
**Status**: Ready for Execution

---

## 📋 Summary

### Findings
- ✅ **Legacy code**: Properly deprecated, keep as-is (exemplary pattern)
- ⚠️ **TODOs**: 4 tarpc TODOs are placeholders for future work (KEEP)
- 🗂️ **Archive docs**: 20+ showcase session documents to archive

---

## 🎯 Action Plan

### Action 1: Archive Showcase Session Documents ✅

**Move to**: `showcase/archive/sessions_2025/`

**Files to archive** (20+ files):
```
COMPLETE_SESSION_SUMMARY_DEC_21_2025.md
SESSION_COMPLETE_CROSS_TOWER_DEC_18_2025.md
SESSION_FINAL_DEC_18_2025.md
SESSION_PROGRESS_DEC_21_2025.md
SESSION_SUMMARY_DEC_18_INTER_PRIMAL.md
SESSION_SUMMARY_LEVEL3_DEC_21_2025.md
SESSION_SUMMARY_NESTGATE_SHOWCASE_DEC_21_2025.md
SHOWCASE_COMPLETE_DEC_18_2025.md
SHOWCASE_EXECUTION_COMPLETE_DEC_21_2025.md
SHOWCASE_REVIEW_SUMMARY_DEC_21_2025.md
ECOSYSTEM_SHOWCASE_REVIEW_DEC_21_2025.md
FINAL_ACHIEVEMENT_LEVEL3_COMPLETE_DEC_21_2025.md
GPU_ENABLEMENT_PLAN_DEC_15_2025.md
GPU_WIRING_COMPLETE_STATUS.md
LOCAL_SHOWCASE_BUILDOUT_DEC_21_2025.md
NESTGATE_SHOWCASE_INDEX_DEC_21_2025.md
NESTGATE_SHOWCASE_PLAN_DEC_21_2025.md
NEXT_SHOWCASE_ACTIONS_DEC_21_2025.md
VISUAL_GUIDE_NESTGATE_DEC_21_2025.md
FULL_DAY_SUMMARY_JAN7_2026.md
```

---

### Action 2: Update TODO Comments (tarpc)  ✅

**File**: `crates/core/toadstool/src/ecosystem/communication.rs`

**Current TODOs** (Lines 108, 128, 170, 257):
```rust
// TODO: Implement actual tarpc message sending
// TODO: Implement WebSocket message sending  
// TODO: Implement tarpc health check
// TODO: Create actual tarpc client
```

**Analysis**:
- ✅ **KEEP**: These are valid future work items
- ✅ tarpc protocol is recognized but not yet fully implemented
- ✅ Fallback to HTTP/JSON-RPC works correctly
- ✅ Warning message exists: "tarpc not yet wired - falling back to HTTP"

**Action**: Update TODOs to be more descriptive:
```rust
// TODO(future): Implement tarpc message sending when tarpc integration complete
// TODO(future): Implement WebSocket message sending for realtime updates
// TODO(future): Implement tarpc health check when tarpc integration complete
// TODO(future): Create tarpc client when tarpc integration complete
```

---

### Action 3: Keep Other TODOs ✅

**Valid TODOs** (Keep as-is):
- `buffer.rs:457` - Async cleanup mechanism (valid optimization)
- `tarpc_server.rs:197` - Calculate resource utilization (future feature)
- `jsonrpc_server.rs:272` - Track actual workloads (future feature)
- `service_discovery.rs:357` - mDNS discovery (future feature)
- `service_discovery.rs:365` - Config file discovery (future feature)

**Reason**: These are legitimate future work items

---

### Action 4: Keep Legacy Code ✅

**File**: `crates/core/toadstool/src/ecosystem/legacy.rs`

**Status**: ✅ **Keep as-is** - Exemplary deprecation pattern

**Why**:
- Properly marked with `#[deprecated]`
- Clear documentation explaining migration path
- Returns empty results (safe)
- Guides users to modern capability-based discovery
- Serves as documentation for old API

**This is a model for how to deprecate code!**

---

## 🗂️ Execution Steps

### Step 1: Create Showcase Archive Directory
```bash
mkdir -p showcase/archive/sessions_2025
```

### Step 2: Move Session Documents
```bash
cd showcase
mv *DEC_*.md *SESSION*.md *COMPLETE*.md FULL_DAY_SUMMARY_JAN7_2026.md archive/sessions_2025/
```

### Step 3: Create Archive README
Create `showcase/archive/sessions_2025/README.md` explaining archived docs

### Step 4: Update TODO Comments
Update tarpc TODOs in `communication.rs` to be more descriptive

### Step 5: Git Commit
```bash
git add .
git commit -m "chore: archive showcase session documents and clarify future TODOs"
```

---

## 📊 Impact Summary

### Documentation
- **Before**: 25+ session docs in showcase root
- **After**: Clean showcase root, 25 docs archived
- **Improvement**: 📁 Better organization

### Code
- **Legacy Code**: ✅ Kept (exemplary pattern)
- **TODOs**: ✅ Clarified (4 updated, 5 kept)
- **Test Coverage**: ✅ No impact

---

## ✅ Verification Checklist

- [ ] Showcase archive directory created
- [ ] Session documents moved to archive
- [ ] Archive README created
- [ ] TODO comments updated in communication.rs
- [ ] Build still passes (`cargo build`)
- [ ] Tests still pass (`cargo test`)
- [ ] Git commit prepared

---

## 🎯 Result

After cleanup:
- ✅ **Cleaner showcase directory**
- ✅ **Historical docs preserved**
- ✅ **Clearer TODO comments**
- ✅ **No functional changes**
- ✅ **Ready for git push**

---

*Ready to execute!*
