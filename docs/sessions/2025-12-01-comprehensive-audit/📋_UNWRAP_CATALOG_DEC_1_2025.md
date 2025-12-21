# 📋 UNWRAP CATALOG - December 1, 2025

**Purpose**: Systematic catalog of all production unwraps for prioritized fixing  
**Scope**: All `src/` files (excluding tests)  
**Status**: 🔄 In Progress  
**Goal**: Safe error handling everywhere

---

## 🎯 OVERVIEW

### Risk Categories:

**HIGH RISK** 🔴 - Can panic in normal operation:
- `.lock().unwrap()` - Lock poisoning
- `.read().unwrap()` - RwLock poisoning  
- `.write().unwrap()` - RwLock poisoning
- `.send().unwrap()` - Channel closed
- `.recv().unwrap()` - Channel closed

**MEDIUM RISK** 🟡 - Can fail on bad input:
- `.parse().unwrap()` - Invalid format
- `.get().unwrap()` - Missing key/index
- Option unwraps - None case
- `.join().unwrap()` - Path joining

**LOW RISK** 🟢 - Safe in context:
- Config defaults (known valid)
- Type conversions (known safe)
- Test utilities (not production)

---

## 📊 CATALOGING IN PROGRESS

### Top Files by Unwrap Count:

Analyzing now...

---

## 🔴 HIGH RISK - Lock/Channel Unwraps

### Production Lock Unwraps:

**Already Fixed**:
- ✅ `crates/core/config/src/ports.rs` (5 instances)
- ✅ `crates/server/src/handlers.rs` (2 instances)

**Still to Fix**:
Cataloging now...

---

## 🟡 MEDIUM RISK - Parse/Option Unwraps

Cataloging now...

---

## 🟢 LOW RISK - Safe Context Unwraps

Cataloging now...

---

## 📋 FIX PRIORITY

### Phase 1: Critical (This Week)
- 🔴 All lock unwraps
- 🔴 All channel unwraps

### Phase 2: Important (Next Week)
- 🟡 Parse unwraps with external input
- 🟡 Option unwraps in hot paths

### Phase 3: Nice to Have (Future)
- 🟢 Add expect messages to safe unwraps
- 🟢 Document why each is safe

---

**Status**: 🔄 Cataloging in progress...  
**Last Updated**: December 1, 2025

