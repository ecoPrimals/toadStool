# 📏 Large File Refactoring Strategy - Smart, Not Split

**Date**: December 5, 2025  
**Target**: 1000 LOC max per file  
**Philosophy**: **Smart refactoring over naive splitting**

---

## 📊 CURRENT STATE

### Files Over 1000 LOC

| **File** | **LOC** | **Type** | **Priority** |
|----------|---------|----------|--------------|
| `biomeos_integration_tests.rs` | 1424 | Test | Medium |
| `comprehensive_policy_tests.rs` | 1397 | Test | Medium |
| `comprehensive_sandbox_tests.rs` | 1188 | Test | Low |
| `runtime_comprehensive_tests.rs` | 1129 | Test | Low |
| `executor_comprehensive_lifecycle_tests.rs` | 1119 | Test | Low |
| `comprehensive_client_tests_expansion.rs` | 1093 | Test | Low |
| `integration_test.rs` | 1072 | Test | Low |
| `network_config_tests.rs` | 1032 | Test | Low |

**Key Observation**: ✅ **ALL 8 files are test files**

**Impact**: Low - Test files can be slightly larger than production code

---

## 🎯 PHILOSOPHY: Smart vs Naive Refactoring

### ❌ NAIVE APPROACH (Don't Do This)

```
# Bad: Arbitrary split by line count
comprehensive_policy_tests.rs (1397 lines)
  └─> policy_tests_part1.rs (700 lines)
  └─> policy_tests_part2.rs (697 lines)
```

**Problems**:
- Arbitrary boundaries
- Lost cohesion
- Harder to navigate
- Unclear organization
- No actual improvement

### ✅ SMART APPROACH (Do This)

```
# Good: Organize by feature/concern
comprehensive_policy_tests.rs (1397 lines)
  └─> policy_creation_tests.rs (350 lines)
  └─> policy_validation_tests.rs (280 lines)
  └─> policy_enforcement_tests.rs (390 lines)
  └─> policy_lifecycle_tests.rs (270 lines)
  └─> policy_edge_cases_tests.rs (107 lines)
```

**Benefits**:
- Clear separation of concerns
- Logical grouping
- Easy to find tests
- Maintainable organization
- Actual improvement

---

## 🏗️ REFACTORING PATTERNS

### Pattern 1: By Feature/Module

**When**: Tests cover multiple distinct features

**Example**: `biomeos_integration_tests.rs` (1424 LOC)

```
Before:
  tests/biomeos_integration_tests.rs (1424 lines)
    - Storage tests (400 lines)
    - Auth tests (300 lines)
    - Agent tests (350 lines)
    - Coordination tests (374 lines)

After:
  tests/biomeos/
    ├── storage_integration_tests.rs (420 lines)
    ├── auth_integration_tests.rs (320 lines)
    ├── agent_integration_tests.rs (360 lines)
    ├── coordination_integration_tests.rs (390 lines)
    └── mod.rs (50 lines - shared fixtures)
```

**Shared Code**: Extract common fixtures to `mod.rs`

```rust
// tests/biomeos/mod.rs
pub mod fixtures {
    pub fn create_test_biomeos_context() -> BiomeOSContext { /* ... */ }
    pub fn mock_biomeos_server() -> MockServer { /* ... */ }
}

pub mod assertions {
    pub fn assert_biomeos_response_valid(resp: &Response) { /* ... */ }
}
```

### Pattern 2: By Test Lifecycle Phase

**When**: Tests follow a clear lifecycle (setup → action → teardown)

**Example**: `executor_comprehensive_lifecycle_tests.rs` (1119 LOC)

```
Before:
  tests/executor_comprehensive_lifecycle_tests.rs (1119 lines)
    - Initialization tests (250 lines)
    - Execution tests (400 lines)
    - Monitoring tests (200 lines)
    - Cleanup tests (200 lines)
    - Error handling (69 lines)

After:
  tests/executor_lifecycle/
    ├── initialization_tests.rs (270 lines)
    ├── execution_tests.rs (420 lines)
    ├── monitoring_tests.rs (220 lines)
    ├── cleanup_tests.rs (210 lines)
    ├── error_handling_tests.rs (90 lines)
    └── mod.rs (80 lines - lifecycle helpers)
```

### Pattern 3: By Test Type

**When**: Mix of unit, integration, and E2E tests

**Example**: `comprehensive_client_tests_expansion.rs` (1093 LOC)

```
Before:
  tests/comprehensive_client_tests_expansion.rs (1093 lines)
    - Unit tests (300 lines)
    - Integration tests (500 lines)
    - E2E tests (293 lines)

After:
  tests/client/
    ├── unit/
    │   ├── client_creation_tests.rs (150 lines)
    │   ├── client_configuration_tests.rs (160 lines)
    │   └── mod.rs
    ├── integration/
    │   ├── client_server_communication_tests.rs (250 lines)
    │   ├── client_retry_logic_tests.rs (260 lines)
    │   └── mod.rs
    ├── e2e/
    │   ├── full_request_cycle_tests.rs (150 lines)
    │   ├── error_scenarios_tests.rs (153 lines)
    │   └── mod.rs
    └── mod.rs (20 lines - re-exports)
```

### Pattern 4: By Concern

**When**: Tests cover orthogonal concerns

**Example**: `comprehensive_policy_tests.rs` (1397 LOC)

```
Before:
  tests/comprehensive_policy_tests.rs (1397 lines)
    - CRUD operations (350 lines)
    - Validation (280 lines)
    - Enforcement (390 lines)
    - Permissions (270 lines)
    - Edge cases (107 lines)

After:
  tests/policy/
    ├── crud_tests.rs (370 lines)
    ├── validation_tests.rs (300 lines)
    ├── enforcement_tests.rs (410 lines)
    ├── permissions_tests.rs (290 lines)
    ├── edge_cases_tests.rs (127 lines)
    └── mod.rs (50 lines - test fixtures)
```

---

## 🔧 REFACTORING PROCESS

### Step 1: Analyze the File

```bash
# Read the file
cat crates/path/to/large_file.rs

# Identify logical sections
# - Count test functions: grep -c "#\[test\]" file.rs
# - Identify imports/fixtures
# - Look for comment sections
# - Check for natural boundaries
```

### Step 2: Plan the Split

**Questions to ask**:
1. What are the major themes/features?
2. Are there natural groupings?
3. What code is shared across groups?
4. How do tests relate to each other?

**Document the plan**:
```markdown
# Refactoring Plan: comprehensive_policy_tests.rs

## Current Structure (1397 LOC)
- Lines 1-50: Imports and shared fixtures
- Lines 51-400: CRUD operation tests (15 tests)
- Lines 401-680: Validation tests (12 tests)
- Lines 681-1070: Enforcement tests (18 tests)
- Lines 1071-1340: Permission tests (11 tests)
- Lines 1341-1397: Edge cases (4 tests)

## Target Structure
tests/policy/
  ├── mod.rs - Shared fixtures (50 lines)
  ├── crud_tests.rs - CRUD operations (370 lines)
  ├── validation_tests.rs - Validation (300 lines)
  ├── enforcement_tests.rs - Enforcement (410 lines)
  ├── permissions_tests.rs - Permissions (290 lines)
  └── edge_cases_tests.rs - Edge cases (127 lines)
```

### Step 3: Extract Shared Code First

```bash
# Create directory
mkdir -p tests/policy

# Create mod.rs with shared fixtures
cat > tests/policy/mod.rs << 'EOF'
//! Shared test fixtures for policy tests

use super::*;

pub fn create_test_policy() -> Policy {
    // Shared fixture
}

pub fn assert_policy_valid(policy: &Policy) {
    // Shared assertion
}
EOF
```

### Step 4: Create Focused Files

```bash
# Move CRUD tests
# Extract lines 51-400 to crud_tests.rs
# Update imports to use super::fixtures::*
# Add file-level documentation

# Repeat for each section
```

### Step 5: Update Parent mod.rs

```rust
// tests/mod.rs or appropriate parent
pub mod policy {
    mod crud_tests;
    mod validation_tests;
    mod enforcement_tests;
    mod permissions_tests;
    mod edge_cases_tests;
    
    // Re-export fixtures if needed
    pub use self::mod_file::fixtures;
}
```

### Step 6: Verify

```bash
# Run tests to ensure nothing broke
cargo test --package crate-name --test policy

# Check file sizes
find tests/policy -name "*.rs" -exec wc -l {} \;

# Verify all tests still run
cargo test --package crate-name
```

---

## 📋 REFACTORING PRIORITY

### High Priority (>1300 LOC)

1. **`biomeos_integration_tests.rs` (1424 lines)**
   - Strategy: Split by BiomeOS module (storage, auth, agents, coordination)
   - Target: 4 files of 300-400 lines each

2. **`comprehensive_policy_tests.rs` (1397 lines)**
   - Strategy: Split by concern (CRUD, validation, enforcement, permissions)
   - Target: 4-5 files of 250-400 lines each

### Medium Priority (1100-1300 LOC)

3. **`comprehensive_sandbox_tests.rs` (1188 lines)**
4. **`runtime_comprehensive_tests.rs` (1129 lines)**
5. **`executor_comprehensive_lifecycle_tests.rs` (1119 lines)**

### Low Priority (1000-1100 LOC)

6. **`comprehensive_client_tests_expansion.rs` (1093 lines)**
7. **`integration_test.rs` (1072 lines)**
8. **`network_config_tests.rs` (1032 lines)**

**Note**: Low priority files are barely over the limit and may not need immediate refactoring.

---

## ✅ EXAMPLE: Complete Refactoring

### Before: `comprehensive_policy_tests.rs` (1397 LOC)

```rust
// tests/comprehensive_policy_tests.rs (1397 lines)

use toadstool_security::policies::*;
use tokio::test;

// Shared fixtures (50 lines)
fn create_test_policy() -> Policy { /* ... */ }
fn create_test_context() -> PolicyContext { /* ... */ }

// CRUD Tests (350 lines)
#[tokio::test]
async fn test_create_policy() { /* ... */ }

#[tokio::test]
async fn test_read_policy() { /* ... */ }

// ... 13 more CRUD tests ...

// Validation Tests (280 lines)
#[tokio::test]
async fn test_validate_policy_name() { /* ... */ }

// ... 11 more validation tests ...

// Enforcement Tests (390 lines)
#[tokio::test]
async fn test_enforce_policy() { /* ... */ }

// ... 17 more enforcement tests ...

// Permission Tests (270 lines)
#[tokio::test]
async fn test_check_permissions() { /* ... */ }

// ... 10 more permission tests ...

// Edge Cases (107 lines)
#[tokio::test]
async fn test_policy_with_empty_name() { /* ... */ }

// ... 3 more edge case tests ...
```

### After: Organized Structure

```
tests/policy/
├── mod.rs (50 lines)
├── crud_tests.rs (370 lines)
├── validation_tests.rs (300 lines)
├── enforcement_tests.rs (410 lines)
├── permissions_tests.rs (290 lines)
└── edge_cases_tests.rs (127 lines)
```

#### `tests/policy/mod.rs`

```rust
//! Shared test fixtures and utilities for policy tests

use toadstool_security::policies::*;

pub mod fixtures {
    use super::*;
    
    pub fn create_test_policy() -> Policy {
        Policy::builder()
            .name("test-policy")
            .rules(vec![/* test rules */])
            .build()
    }
    
    pub fn create_test_context() -> PolicyContext {
        PolicyContext::new_for_testing()
    }
}

pub mod assertions {
    use super::*;
    
    pub fn assert_policy_valid(policy: &Policy) {
        assert!(!policy.name().is_empty());
        assert!(!policy.rules().is_empty());
    }
}

// Re-export test modules
mod crud_tests;
mod validation_tests;
mod enforcement_tests;
mod permissions_tests;
mod edge_cases_tests;
```

#### `tests/policy/crud_tests.rs`

```rust
//! CRUD operation tests for policies

use super::fixtures::*;
use toadstool_security::policies::*;
use tokio::test;

#[tokio::test]
async fn test_create_policy() {
    let policy = create_test_policy();
    let storage = PolicyStorage::new_in_memory();
    
    let result = storage.create(policy.clone()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_read_policy() {
    // ... test implementation ...
}

// ... 13 more focused CRUD tests ...
```

#### `tests/policy/validation_tests.rs`

```rust
//! Policy validation tests

use super::fixtures::*;
use toadstool_security::policies::*;
use tokio::test;

#[tokio::test]
async fn test_validate_policy_name() {
    let mut policy = create_test_policy();
    policy.set_name("");
    
    let result = policy.validate();
    assert!(result.is_err());
}

// ... 11 more validation tests ...
```

**Benefits**:
- Clear organization ✅
- Each file under 500 LOC ✅
- Shared code in mod.rs ✅
- Easy to find specific tests ✅
- Maintainable structure ✅

---

## 🎯 SUCCESS CRITERIA

### For Each Refactored File:

1. ✅ **All files < 1000 LOC** (preferably < 500)
2. ✅ **Logical grouping** by feature/concern
3. ✅ **Shared code extracted** to mod.rs or fixtures module
4. ✅ **Clear naming** that indicates content
5. ✅ **All tests still pass** after refactoring
6. ✅ **Documentation** in each file explaining scope

### Overall Goals:

- [ ] 8 large files → 35-40 organized files
- [ ] Average file size: 300-400 LOC
- [ ] Clear test organization
- [ ] Improved maintainability
- [ ] No test functionality lost

---

## 📊 ESTIMATED EFFORT

### Per File:

- Analysis: 30 minutes
- Planning: 30 minutes
- Extraction: 2 hours
- Verification: 30 minutes
- **Total**: ~3.5 hours per file

### All 8 Files:

- High priority (2 files): 7 hours
- Medium priority (3 files): 10.5 hours
- Low priority (3 files): 10.5 hours (optional)
- **Total**: 14-28 hours (2-4 work days)

---

## 🚀 RECOMMENDED APPROACH

### Phase 1: Proof of Concept (Week 1)

**Target**: 1 high-priority file
**File**: `comprehensive_policy_tests.rs` (1397 LOC)
**Goal**: Establish pattern and verify approach

### Phase 2: High Priority (Week 2)

**Target**: Remaining high-priority file
**File**: `biomeos_integration_tests.rs` (1424 LOC)
**Goal**: Apply learned patterns

### Phase 3: Medium Priority (Week 3)

**Target**: 3 medium-priority files
**Goal**: Complete files significantly over limit

### Phase 4: Low Priority (Optional, Week 4)

**Target**: 3 low-priority files
**Decision**: Only if time permits and value is clear

---

## ✅ DECISION: Low Priority for Test Files

### Why Test Files Over 1000 LOC Are Acceptable:

1. **Test files are not shipped** to production
2. **Comprehensive tests naturally get large**
3. **Splitting can hurt cohesion** if done poorly
4. **Industry standard** allows larger test files

### When to Refactor Test Files:

- ✅ When they become hard to navigate
- ✅ When they cover multiple distinct features
- ✅ When shared code is duplicated
- ❌ Not just because of LOC count

### Recommended Threshold:

- Production code: **1000 LOC hard limit**
- Test code: **1500 LOC soft limit, 2000 hard limit**

**Current status**: All 8 files are 1032-1424 LOC (acceptable range)

---

## 🎖️ VERDICT

### **Current State**: ACCEPTABLE ✅

**Why**:
- All large files are tests
- Sizes are reasonable (1000-1400 LOC)
- No production files over limit
- Comprehensive tests are naturally larger

### **Recommended Action**: Smart Refactoring (Optional)

**Priority**: Low-Medium
**Timeline**: Optional improvement, not critical
**Approach**: Feature-based organization, not arbitrary splitting

**If refactoring**: Do high-priority files first (biomeos, policy)

**If not refactoring**: Document decision and monitor

---

## 📝 ALTERNATIVE: Document Decision

If team decides not to refactor, update guidelines:

```markdown
# Code Size Guidelines

## Production Code
- **Hard limit**: 1000 LOC
- **Soft limit**: 500 LOC
- **Guideline**: Refactor if growing

## Test Code
- **Hard limit**: 2000 LOC
- **Soft limit**: 1500 LOC
- **Guideline**: Organize by feature if growing
- **Exception**: Comprehensive integration tests

## Current Exceptions (Documented)
- biomeos_integration_tests.rs (1424 LOC) - Acceptable
- comprehensive_policy_tests.rs (1397 LOC) - Acceptable
- ... (6 more files documented)
```

---

**Analysis Complete**: December 5, 2025  
**Recommendation**: Low priority, smart refactoring if/when undertaken  
**Status**: ✅ **ACCEPTABLE AS-IS** (all large files are tests)

---

*"Smart refactoring improves architecture. Naive splitting just creates more files."* 🎯

