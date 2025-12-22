# Error Module Smart Refactoring Plan

**Current**: 1,088 lines in single file  
**Target**: Logical module structure  
**Philosophy**: **Separate by FUNCTION, not arbitrary lines**

---

## Current Structure Analysis

The file has excellent logical organization:

1. **Tier 1**: ToadStoolError enum (lines 38-76, ~40 lines)
2. **Tier 2**: 7 specialized errors (lines 77-325, ~250 lines)
3. **Conversions**: From impls (lines 326-391, ~65 lines)
4. **Constructors**: Helper constructors (lines 392-549, ~160 lines)
5. **Context**: ToadStoolError impl with context (lines 550-758, ~210 lines)
6. **Extensions**: Traits, wrappers (lines 759-1088, ~330 lines)

---

## Smart Refactoring Strategy

### New Structure:

```
error/
├── mod.rs           (~80 lines) - Public API, re-exports, module docs
├── types.rs         (~290 lines) - All error type definitions
├── constructors.rs  (~160 lines) - Helper constructors for ergonomic creation
├── conversions.rs   (~65 lines) - From impls for std/external types
├── context.rs       (~210 lines) - Context builders, ToadStoolError helpers
└── extensions.rs    (~330 lines) - Traits, ToadStoolErrorWithCode, extras
```

**Total**: 6 files, all under 350 lines, average ~190 lines

---

## File Contents

### `mod.rs` (~80 lines)
**Purpose**: Public API surface, clear entry point

```rust
//! # Unified Error System for ToadStool Platform
//! 
//! [Module documentation from current file]

// Re-export all public types
pub use types::*;
pub use constructors::*;
pub use conversions::*;
pub use context::*;
pub use extensions::*;

// Module declarations
mod types;
mod constructors;
mod conversions;
mod context;
mod extensions;
```

---

### `types.rs` (~290 lines)
**Purpose**: All error type definitions (data structures)

```rust
//! Error type definitions

use std::time::Duration;
use thiserror::Error;
use crate::error_codes::ErrorCode;

// Tier 1: ToadStoolError enum
#[derive(Error, Debug)]
pub enum ToadStoolError {
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),
    // ... all variants
}

// Tier 2: All specialized errors
#[derive(Error, Debug)]
pub enum ExecutionError { /* ... */ }

#[derive(Error, Debug)]
pub enum ConfigError { /* ... */ }

// ... all 7 specialized error enums
```

**Contents**:
- ToadStoolError enum (~40 lines)
- ExecutionError enum (~40 lines)
- ConfigError enum (~35 lines)
- ResourceError enum (~35 lines)
- IntegrationError enum (~30 lines)
- SecurityError enum (~30 lines)
- NetworkError enum (~35 lines)
- SystemError enum (~40 lines)
- Helper structs if any (~5 lines)

---

### `constructors.rs` (~160 lines)
**Purpose**: Helper constructors for ergonomic error creation

```rust
//! Helper constructors for creating errors ergonomically

use super::types::*;

impl ExecutionError {
    pub fn runtime_failure(runtime: impl Into<String>, /* ... */) -> Self {
        // Constructor implementation
    }
    // ... all helpers
}

impl ConfigError {
    pub fn not_found(path: impl Into<String>) -> Self {
        // ...
    }
    // ... all helpers
}

// All constructor impls for each error type
```

**Contents**:
- ExecutionError constructors (~25 lines)
- ConfigError constructors (~20 lines)
- ResourceError constructors (~25 lines)
- IntegrationError constructors (~20 lines)
- SecurityError constructors (~20 lines)
- NetworkError constructors (~20 lines)
- SystemError constructors (~20 lines)
- Documentation (~10 lines)

---

### `conversions.rs` (~65 lines)
**Purpose**: From impls for external types

```rust
//! Type conversions from standard library and external types

use super::types::*;

impl From<std::io::Error> for ToadStoolError {
    fn from(err: std::io::Error) -> Self {
        // ...
    }
}

impl From<serde_json::Error> for ToadStoolError {
    // ...
}

// All From impls
```

**Contents**:
- std::io::Error conversions (~15 lines)
- serde_json::Error conversions (~15 lines)
- Other standard conversions (~20 lines)
- Documentation (~15 lines)

---

### `context.rs` (~210 lines)
**Purpose**: Context building, error annotation, chaining

```rust
//! Error context helpers and builders

use super::types::*;

impl ToadStoolError {
    /// Add context to this error
    pub fn context(self, context: impl Into<String>) -> Self {
        // ...
    }
    
    /// Map to different error type with context
    pub fn map_execution(self, msg: impl Into<String>) -> Self {
        // ...
    }
    
    // All context and mapping methods
}

// Trait for adding context
pub trait ToadStoolErrorExt {
    // ...
}
```

**Contents**:
- ToadStoolError impl with context methods (~150 lines)
- ToadStoolErrorExt trait (~30 lines)
- Context helpers (~20 lines)
- Documentation (~10 lines)

---

### `extensions.rs` (~330 lines)
**Purpose**: Optional traits, wrappers, extensions

```rust
//! Error extensions and wrappers

use super::types::*;

/// Error with attached error code
pub struct ToadStoolErrorWithCode {
    pub error: ToadStoolError,
    pub code: ErrorCode,
}

impl ToadStoolErrorWithCode {
    // ...
}

// Trait implementations, Display, Debug, etc.
```

**Contents**:
- ToadStoolErrorWithCode struct (~50 lines)
- ToadStoolErrorWithCode impls (~100 lines)
- Additional trait impls (~100 lines)
- Optional extensions (~60 lines)
- Documentation (~20 lines)

---

## Migration Strategy

### Phase 1: Create structure (15 min)
1. Create `error/` directory
2. Create all 6 files with proper module docs
3. Copy sections to appropriate files

### Phase 2: Re-exports (15 min)
1. Set up `mod.rs` with re-exports
2. Ensure all public APIs exported
3. Test compilation

### Phase 3: Fix imports (30 min)
1. Update internal use statements
2. Fix cross-references between files
3. Ensure all tests pass

### Phase 4: Documentation (30 min)
1. Update module-level docs
2. Add file-level documentation
3. Cross-reference between modules

### Phase 5: Validation (30 min)
1. Run all tests
2. Check documentation builds
3. Verify API compatibility
4. Run benchmarks (ensure no performance regression)

**Total time**: ~2 hours

---

## Benefits

1. ✅ **File size compliance**: All files < 350 lines
2. ✅ **Logical organization**: Separated by function, not arbitrary
3. ✅ **Maintainability**: Easier to find and update error types
4. ✅ **Discoverability**: Clear file names indicate contents
5. ✅ **API preservation**: All public APIs unchanged
6. ✅ **No breaking changes**: Perfect backward compatibility

---

## Rationale: Why This Organization?

### `types.rs` - Data Definitions
**Rationale**: All error *structures* in one place. When you need to know "what errors exist", look here.

### `constructors.rs` - Ergonomic Creation
**Rationale**: Helpers for *creating* errors. Separate from types because:
- Types define *what* errors are
- Constructors define *how* to create them ergonomically

### `conversions.rs` - Integration Points
**Rationale**: How external types become our errors. Separate because:
- Clear integration boundary with external crates
- Easy to audit what external errors we handle
- Simple to add new conversions

### `context.rs` - Error Enrichment
**Rationale**: Adding information to errors. Separate because:
- Context is added *after* creation
- Different concern from type definition
- Contains complex logic for error chaining

### `extensions.rs` - Optional Features
**Rationale**: Extra functionality beyond core errors. Separate because:
- Not everyone needs error codes
- Can be optional in future
- Clear extension point

---

## Comparison with Arbitrary Split

### ❌ Bad: Arbitrary Split by Lines
```
error/
├── error_part1.rs   (400 lines) - Lines 1-400
├── error_part2.rs   (400 lines) - Lines 401-800
└── error_part3.rs   (288 lines) - Lines 801-1088
```

**Problems**:
- No logical grouping
- Hard to find anything
- Tight coupling between files
- Confusing for new contributors

### ✅ Good: Functional Split (Our Approach)
```
error/
├── types.rs         (290 lines) - Error definitions
├── constructors.rs  (160 lines) - Creation helpers
├── conversions.rs   (65 lines)  - External integrations
├── context.rs       (210 lines) - Error enrichment
└── extensions.rs    (330 lines) - Optional features
```

**Benefits**:
- Clear responsibility per file
- Easy to find what you need
- Loose coupling
- Self-documenting structure

---

## Validation Checklist

Before:
- [ ] Backup current error.rs
- [ ] All tests passing
- [ ] Cargo doc builds successfully

During:
- [ ] Each file compiles independently
- [ ] All re-exports correct
- [ ] Tests still pass after each step

After:
- [ ] All tests passing
- [ ] Cargo doc builds successfully
- [ ] No performance regression
- [ ] Cargo clippy clean
- [ ] File sizes: all < 400 lines ✅

---

**Status**: READY FOR IMPLEMENTATION  
**Estimated Time**: 2-3 hours  
**Risk**: LOW (backward compatible)  
**Impact**: HIGH (better organization, compliance)

