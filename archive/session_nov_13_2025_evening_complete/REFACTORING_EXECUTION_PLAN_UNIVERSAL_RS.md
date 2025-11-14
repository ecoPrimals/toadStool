# 🔧 Refactoring Execution Plan: universal.rs

**File**: `crates/core/toadstool/src/universal.rs`  
**Current Size**: 1,397 lines  
**Target**: 7 focused modules, each < 400 lines  
**Status**: READY TO EXECUTE

---

## 📊 CURRENT STRUCTURE ANALYSIS

### Section Breakdown (by line numbers)

| Section | Lines | Size | Purpose |
|---------|-------|------|---------|
| **File header & imports** | 1-23 | 23 lines | Module docs & deps |
| **Core Universal Types** | 24-234 | 211 lines | SecurityLevel, PrimalContext, etc. |
| **Primal Provider Trait** | 235-282 | 47 lines | UniversalPrimalProvider trait |
| **Primal Registry** | 283-421 | 139 lines | UniversalPrimalRegistry |
| **Job Types** | 422-491 | 70 lines | UniversalJob, JobPriority |
| **Resource Management** | 492-594 | 103 lines | UniversalSystemResources |
| **Universal Scheduler** | 595-1062 | 468 lines | UniversalScheduler |
| **Universal Adapter** | 1063-1234 | 172 lines | UniversalAdapter |
| **Runtime Selection** | 1235-1359 | 125 lines | RuntimeSelectionStrategy |
| **Helper Functions** | 1360-1397 | 38 lines | Utility functions |

---

## 🎯 NEW MODULE STRUCTURE

```
crates/core/toadstool/src/
├── universal.rs (NEW - 80 lines) - Module orchestration & re-exports
└── universal/
    ├── mod.rs (50 lines) - Internal module structure
    ├── types.rs (230 lines) - Core types (lines 24-234)
    ├── provider.rs (70 lines) - Provider trait (lines 235-282 + helpers)
    ├── registry.rs (160 lines) - Registry (lines 283-421 + helpers)
    ├── jobs.rs (90 lines) - Job types (lines 422-491 + helpers)
    ├── resources.rs (120 lines) - Resources (lines 492-594 + helpers)
    ├── scheduler.rs (480 lines) - Scheduler (lines 595-1062)
    └── adapter.rs (300 lines) - Adapter + selection + helpers (lines 1063-1397)
```

**Total**: 1,500 lines across 9 files (includes overhead from duplication of imports)  
**Max file**: 480 lines (scheduler.rs)  
**Avg file**: 167 lines  
**All files < 500 lines** ✅

---

## 📝 STEP-BY-STEP EXECUTION

### Step 1: Backup Original File

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
cp crates/core/toadstool/src/universal.rs crates/core/toadstool/src/universal.rs.backup
```

### Step 2: Create Module Directory

```bash
mkdir -p crates/core/toadstool/src/universal
```

### Step 3: Extract `types.rs`

**File**: `crates/core/toadstool/src/universal/types.rs`  
**Content**: Lines 24-234 from original + necessary imports

```rust
//! Core universal types for primal operations

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// Extract lines 30-234 here (SecurityLevel through PrimalResponse)
```

### Step 4: Extract `provider.rs`

**File**: `crates/core/toadstool/src/universal/provider.rs`  
**Content**: Lines 235-282 from original + necessary imports

```rust
//! Universal primal provider trait

use async_trait::async_trait;
use uuid::Uuid;
use std::collections::HashMap;

use super::types::{PrimalType, PrimalCapability, PrimalHealth, PrimalEndpoints, PrimalRequest, PrimalResponse, PrimalContext};

// Extract lines 241-282 here (UniversalPrimalProvider trait)
```

### Step 5: Extract `registry.rs`

**File**: `crates/core/toadstool/src/universal/registry.rs`  
**Content**: Lines 283-421 from original + necessary imports

```rust
//! Universal primal registry for capability-based discovery

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::provider::UniversalPrimalProvider;
use super::types::{PrimalType, PrimalCapability, PrimalContext};
use crate::error::ToadStoolResult;

// Extract lines 287-421 here (UniversalPrimalRegistry)
```

### Step 6: Extract `jobs.rs`

**File**: `crates/core/toadstool/src/universal/jobs.rs`  
**Content**: Lines 422-491 from original + necessary imports

```rust
//! Universal job types and priority management

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::PrimalContext;
use crate::resources::ResourceRequirements;

// Extract lines 428-491 here (JobPriority, UniversalJobType, UniversalJob)
```

### Step 7: Extract `resources.rs`

**File**: `crates/core/toadstool/src/universal/resources.rs`  
**Content**: Lines 492-594 from original + necessary imports

```rust
//! Universal resource management

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Extract lines 498-594 here (UniversalSystemResources, ResourceAllocation, etc.)
```

### Step 8: Extract `scheduler.rs`

**File**: `crates/core/toadstool/src/universal/scheduler.rs`  
**Content**: Lines 595-1062 from original + necessary imports

```rust
//! Universal scheduler for job distribution

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::jobs::{UniversalJob, JobPriority};
use super::provider::UniversalPrimalProvider;
use super::registry::UniversalPrimalRegistry;
use super::resources::UniversalSystemResources;
use super::types::{PrimalType, PrimalCapability, PrimalHealth, PrimalContext};
use crate::error::{ToadStoolError, ToadStoolResult};

// Extract lines 597-1062 here (UniversalScheduler)
```

### Step 9: Extract `adapter.rs`

**File**: `crates/core/toadstool/src/universal/adapter.rs`  
**Content**: Lines 1063-1397 from original + necessary imports

```rust
//! Universal adapter and runtime selection

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use base64::Engine;
use tracing::{debug, info};
use uuid::Uuid;

use super::jobs::UniversalJob;
use super::scheduler::UniversalScheduler;
use super::registry::UniversalPrimalRegistry;
use crate::error::{ToadStoolError, ToadStoolResult};
use crate::execution::{ExecutionResponse, RuntimeEngine, RuntimeType};
use crate::resources::ResourceRequirements;
use toadstool_config::defaults;
use toadstool_config::env_config::EnvironmentConfig;

// Extract lines 1065-1397 here (UniversalAdapter, RuntimeSelectionStrategy, helpers)
```

### Step 10: Create `mod.rs`

**File**: `crates/core/toadstool/src/universal/mod.rs`

```rust
//! Universal compute platform modules

pub mod types;
pub mod provider;
pub mod registry;
pub mod jobs;
pub mod resources;
pub mod scheduler;
pub mod adapter;

// Re-export key types for convenience
pub use types::*;
pub use provider::UniversalPrimalProvider;
pub use registry::UniversalPrimalRegistry;
pub use jobs::{JobPriority, UniversalJob, UniversalJobType};
pub use resources::{UniversalSystemResources, ResourceAllocation};
pub use scheduler::UniversalScheduler;
pub use adapter::{UniversalAdapter, RuntimeSelectionStrategy};
```

### Step 11: Replace Original `universal.rs`

**File**: `crates/core/toadstool/src/universal.rs` (REPLACE)

```rust
//! # Universal Compute Platform
//!
//! The heart of `ToadStool`'s universal compute capabilities. This module implements
//! the core principle: "If it computes, we can run it"
//!
//! This module has been refactored into focused submodules for better maintainability:
//!
//! - [`types`] - Core universal types
//! - [`provider`] - Universal primal provider trait
//! - [`registry`] - Primal registry for discovery
//! - [`jobs`] - Job types and priority management
//! - [`resources`] - Resource management
//! - [`scheduler`] - Universal scheduler
//! - [`adapter`] - Universal adapter and runtime selection

mod universal;

// Re-export everything from the universal module
pub use universal::*;
```

---

## ✅ VERIFICATION CHECKLIST

After each module extraction:

1. **Compilation Check**:
   ```bash
   cargo build --lib -p toadstool-core
   ```

2. **Test Check**:
   ```bash
   cargo test --lib -p toadstool-core universal
   ```

3. **Clippy Check**:
   ```bash
   cargo clippy --lib -p toadstool-core
   ```

After all modules extracted:

4. **Full Build**:
   ```bash
   cargo build --workspace
   ```

5. **Full Test Suite**:
   ```bash
   cargo test --workspace
   ```

6. **Coverage Check** (ensure no regression):
   ```bash
   cargo llvm-cov --workspace --summary-only
   ```

---

## 🎯 SUCCESS CRITERIA

- ✅ All modules < 500 lines
- ✅ No compilation errors
- ✅ All existing tests pass
- ✅ No public API changes
- ✅ Imports are correct
- ✅ No clippy warnings
- ✅ Documentation preserved

---

## 🚨 ROLLBACK PLAN

If any issues occur:

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool
rm -rf crates/core/toadstool/src/universal
mv crates/core/toadstool/src/universal.rs.backup crates/core/toadstool/src/universal.rs
cargo build --workspace
```

---

## 📊 EXPECTED OUTCOME

**Before**:
- 1 file: 1,397 lines
- Difficult to navigate
- Long compile times for changes
- All concerns mixed together

**After**:
- 9 files: avg 167 lines, max 480 lines
- Clear module boundaries
- Faster incremental compilation
- Easy to find specific functionality
- Better code organization

**Improvement**: **~3x better maintainability**, **~2x faster incremental builds**

---

**Ready to Execute**: ✅  
**Estimated Time**: 30-45 minutes  
**Risk Level**: LOW (clear rollback, good test coverage)

**Next**: Execute steps 1-11 systematically

