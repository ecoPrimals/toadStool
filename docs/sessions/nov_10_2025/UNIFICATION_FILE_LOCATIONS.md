# 📍 ToadStool Unification - File Locations Reference

**Purpose**: Quick lookup for files needing attention during unification  
**Based on**: UNIFICATION_TECHNICAL_AUDIT_NOV_10_2025.md  
**Last Updated**: November 10, 2025

---

## 🎯 HIGH-PRIORITY FILES

### **async_trait Migration** (74 instances, 37 files)

#### **Top Priority** (Most instances)

```
crates/core/toadstool/src/os_layer/compat.rs:5
├─ Lines with #[async_trait]: 5 traits
└─ Impact: Core platform compatibility

crates/core/common/src/infant_discovery/sources.rs:5
├─ Lines with #[async_trait]: 5 traits
└─ Impact: Service discovery

crates/core/common/src/infant_discovery/detectors.rs:5
├─ Lines with #[async_trait]: 5 traits
└─ Impact: Service detection

crates/core/toadstool/src/biomeos_integration/storage_backend.rs:4
├─ Lines with #[async_trait]: 4 traits
└─ Impact: BiomeOS storage integration

crates/core/toadstool/src/execution.rs:3
├─ Lines with #[async_trait]: 3 traits
└─ Impact: Core execution engine

crates/core/toadstool/src/biomeos_integration/auth_backend.rs:3
├─ Lines with #[async_trait]: 3 traits
└─ Impact: BiomeOS authentication

crates/core/toadstool/src/biomeos_integration/agent_backend.rs:3
├─ Lines with #[async_trait]: 3 traits
└─ Impact: BiomeOS agent integration

crates/core/common/src/infant_discovery/engine.rs:3
├─ Lines with #[async_trait]: 3 traits
└─ Impact: Discovery engine

crates/core/common/src/infant_discovery/capabilities.rs:3
├─ Lines with #[async_trait]: 3 traits
└─ Impact: Capability detection
```

#### **Medium Priority** (2 instances each)

```
crates/distributed/src/primal_capabilities/adapters.rs:2
crates/runtime/gpu/src/frameworks.rs:2
crates/runtime/python/src/lib.rs:1
crates/runtime/wasm/src/lib.rs:2
crates/core/toadstool/src/universal.rs:2
crates/security/policies/src/manager.rs:2
crates/integration/protocols/src/lib.rs:2
crates/management/performance/src/lib.rs:2
crates/management/analytics/src/lib.rs:2
crates/core/toadstool/src/byob.rs:2
crates/core/toadstool/src/production_hardening.rs:2
crates/security/sandbox/src/lib.rs:2
```

#### **Quick Wins** (1 instance each)

```
crates/distributed/src/cloud/core.rs:1
crates/testing/src/mocks/runtime_engines.rs:1
crates/runtime/native/src/lib.rs:1
crates/runtime/gpu/src/traits.rs:1
crates/runtime/gpu/src/engine.rs:1
crates/runtime/edge/src/lib.rs:1
crates/runtime/edge/src/platforms/esp32.rs:1
crates/runtime/edge/src/platforms/arduino.rs:1
crates/runtime/edge/src/platforms/mod.rs:1
crates/runtime/container/src/lib.rs:1
crates/core/toadstool/src/security.rs:1
crates/runtime/wasm/src/component_model.rs:1
crates/core/toadstool/src/os_layer/compat.rs:1
```

---

### **Documentation Enhancement** (7 priority files)

#### **Large Config Modules** (Need module-level docs)

```
crates/cli/src/network_config/types.rs
├─ Size: ~1032 lines
├─ Configs: 36 structs
├─ Priority: HIGH
└─ Add: Module overview, organization, relationships

crates/core/toadstool/src/biomeos_integration/types.rs
├─ Size: 989 lines
├─ Configs: 27 structs
├─ Priority: HIGH
└─ Add: BiomeOS integration guide, type relationships

crates/core/toadstool/src/universal.rs
├─ Size: 1,397 lines
├─ Scope: Universal compute types
├─ Priority: HIGH
└─ Add: Architecture overview, type hierarchy

crates/core/config/src/lib.rs
├─ Size: 1,556 lines
├─ Scope: Core configuration
├─ Priority: HIGH
└─ Add: Configuration system overview

crates/distributed/src/songbird_integration/types.rs
├─ Configs: 14 structs
├─ Priority: MEDIUM
└─ Add: Songbird integration patterns

crates/distributed/src/cloud/types.rs
├─ Configs: 14 structs
├─ Priority: MEDIUM
└─ Add: Cloud provider abstraction guide

crates/runtime/specialty/src/types/configs.rs
├─ Configs: 13 structs
├─ Priority: MEDIUM
└─ Add: Specialty runtime configuration guide
```

---

### **Deprecated Marker Cleanup** (50 files)

#### **Top Files** (Most references)

```
crates/runtime/specialty/src/embedded.rs
crates/runtime/specialty/src/mainframe.rs
crates/runtime/specialty/src/industrial.rs
crates/runtime/specialty/src/realtime.rs
crates/runtime/specialty/src/lib.rs
crates/runtime/specialty/src/types/*.rs (5 files)
crates/core/common/src/infant_discovery/*.rs (4 files)
crates/distributed/src/primal_capabilities/*.rs (3 files)
crates/core/toadstool/src/os_layer/compat.rs
crates/core/toadstool/src/execution.rs
crates/integration/protocols/src/config.rs
```

**Actions for each file**:
1. Search for "deprecated", "compat", "shim", "legacy" (case-insensitive)
2. Determine if:
   - Feature (keep, update naming)
   - Transitional (add removal date)
   - Unused (remove)

---

## 📊 STATISTICS BY DIRECTORY

### **Largest Files** (Top 10, all compliant)

```
1,556 lines: crates/core/config/src/lib.rs                          ✅
1,472 lines: crates/testing/src/integration.rs                      ✅
1,450 lines: crates/security/sandbox/src/lib.rs                     ✅
1,424 lines: crates/core/toadstool/tests/biomeos_integration_tests.rs ✅
1,401 lines: crates/api/src/handlers.rs                             ✅
1,397 lines: crates/security/policies/tests/comprehensive_policy_tests.rs ✅
1,397 lines: crates/core/toadstool/src/universal.rs                 ✅
1,394 lines: crates/core/toadstool/src/byob.rs                      ✅
1,322 lines: crates/runtime/specialty/src/embedded.rs               ✅
1,265 lines: crates/auto_config/src/natural_language.rs             ✅
```

**Status**: All well below 2,000 line limit ✅

---

### **Config Concentration** (Files with 10+ configs)

```
36 configs: crates/cli/src/network_config/types.rs
27 configs: crates/core/toadstool/src/biomeos_integration/types.rs
21 configs: crates/core/config/src/lib.rs
14 configs: crates/distributed/src/songbird_integration/types.rs
14 configs: crates/distributed/src/cloud/types.rs
13 configs: crates/runtime/specialty/src/types/configs.rs
12 configs: crates/runtime/gpu/src/config.rs
```

---

### **Clone Usage Hotspots** (20+ clones)

```
20 clones: crates/cli/src/universal/operations/utilities.rs
20 clones: crates/core/toadstool/src/biomeos_integration/storage_backend.rs
20 clones: crates/integration/protocols/src/client.rs
14 clones: crates/runtime/specialty/src/mainframe.rs
12 clones: crates/runtime/specialty/src/embedded.rs
11 clones: crates/distributed/src/songbird_integration/discovery.rs
11 clones: crates/api/src/handlers.rs
```

**Note**: Profile before optimizing - many are Arc::clone() (cheap) or necessary

---

## 🔍 KEY TYPE LOCATIONS

### **Canonical Types** (Single Source of Truth)

```
JobPriority (CANONICAL):
├─ Location: crates/core/toadstool/src/universal.rs:418-495
├─ Re-exported in: crates/core/toadstool/src/lib.rs
└─ Usage: Import via `use toadstool::JobPriority;`

ResourceRequirements (CANONICAL):
├─ Location: crates/core/toadstool/src/resources.rs:1-79
├─ Domain variants:
│   ├─ distributed::ResourceRequirements (network-optimized)
│   └─ client::ResourceRequirements (user-friendly)
└─ Conversions: Bidirectional From traits implemented

UniversalJobType:
├─ Location: crates/core/toadstool/src/universal.rs
└─ Extended in: crates/distributed/src/types/jobs.rs (with scheduling hints)

UniversalSystemResources:
├─ Location: crates/core/toadstool/src/universal.rs
├─ Note: Renamed from SystemResources to avoid collision
└─ Includes: Special hardware mapping for exotic platforms
```

---

### **Base Configs** (Composition Patterns)

```
Location: crates/core/common/src/config_bases.rs

Available base configs:
├─ TimeoutConfig (4 timeout fields)
├─ RetryConfig (exponential backoff)
├─ HealthCheckConfig (health monitoring)
├─ HttpHealthCheckConfig (HTTP-specific health)
├─ ConnectionPoolConfig (connection pooling)
├─ CacheConfig (cache with TTL)
├─ BackendEndpoint (network endpoints)
├─ ValidationConfig (security validation)
├─ BaseResourceConfig (resource limits)
└─ ResourceLimit (individual resource specs)
```

---

### **Constants** (Default Values)

```
Location: crates/core/config/src/defaults.rs

Module organization:
├─ network     - Service ports (SONGBIRD_PORT, API_PORT, etc.)
├─ ports       - Port ranges (CONTAINER_START, RANGE_END, etc.)
├─ timeouts    - Timeout values (all in milliseconds)
├─ retries     - Retry configuration (MAX_ATTEMPTS, BACKOFF_MS, etc.)
├─ storage     - Storage URLs (DISTRIBUTED_URL, REDIS_PORT, etc.)
├─ resources   - Resource limits (WORKER_THREADS, MAX_CONNECTIONS, etc.)
├─ endpoints   - Helper functions (songbird(), api(), etc.)
├─ logging     - Log config (LEVEL, FORMAT)
├─ validation  - Min/max thresholds (NEW: 16 constants)
└─ durations   - Duration helpers (connection(), request(), etc.)

Documentation: CONSTANTS_REFERENCE.md (630 lines)
```

---

### **Error System** (Error Handling)

```
Primary error type:
├─ Location: crates/core/common/src/error.rs (971 lines)
├─ Type: ToadStoolError enum (7 top-level variants)
├─ Context: ResultExt trait for rich errors
└─ Documentation: docs/ERROR_CODE_SYSTEM_DESIGN.md

Domain-specific error types:
├─ crates/integration/primals/src/error.rs (IntegrationError)
├─ crates/client/src/client/error.rs (ClientError)
├─ crates/server/src/errors.rs (ServerError)
└─ All convert to/from ToadStoolError
```

---

## 🛠️ TOOLS & SCRIPTS

### **Finding async_trait Instances**

```bash
# Find all async_trait usage
grep -rn "#\[async_trait\]" crates --include="*.rs"

# Count by file
grep -rn "#\[async_trait\]" crates --include="*.rs" | cut -d: -f1 | sort | uniq -c | sort -rn

# Find specific trait definitions
grep -A 5 "#\[async_trait\]" crates --include="*.rs" | grep "pub trait"
```

### **Finding Deprecated Markers**

```bash
# Find all deprecated references
grep -rin "deprecated\|legacy\|compat\|shim" crates --include="*.rs"

# Count by file
grep -ril "deprecated\|legacy\|compat\|shim" crates --include="*.rs" | wc -l

# Find specific patterns
grep -rn "// deprecated" crates --include="*.rs" -i
```

### **Finding Config Structs**

```bash
# Find all config structs
grep -rn "pub struct.*Config" crates --include="*.rs"

# Count by file
grep -rn "pub struct.*Config" crates --include="*.rs" | cut -d: -f1 | sort | uniq -c | sort -rn

# Find config concentrations (10+ configs per file)
grep -rn "pub struct.*Config" crates --include="*.rs" | cut -d: -f1 | sort | uniq -c | sort -rn | awk '$1 >= 10'
```

### **Finding Clone Usage**

```bash
# Find all .clone() calls
grep -rn "\.clone()" crates --include="*.rs"

# Count by file
grep -rn "\.clone()" crates --include="*.rs" | cut -d: -f1 | sort | uniq -c | sort -rn | head -20

# Find Arc::clone patterns (cheap)
grep -rn "Arc::clone" crates --include="*.rs"
```

### **Finding Large Files**

```bash
# Find all Rust files with line counts
find crates -name "*.rs" -type f -exec wc -l {} + | sort -rn | head -30

# Find files approaching 2000 lines
find crates -name "*.rs" -type f -exec wc -l {} + | awk '$1 >= 1500' | sort -rn
```

---

## 📚 DOCUMENTATION LOCATIONS

### **Reference Guides** (Root Directory)

```
TYPES_REFERENCE.md              - Canonical type definitions (505 lines)
CONFIG_PATTERNS_GUIDE.md        - Configuration patterns (652 lines)
CONSTANTS_REFERENCE.md          - Default constants (630 lines)
QUICK_REFERENCE_CARD.md         - Quick lookup reference
ERROR_CODE_SYSTEM_DESIGN.md     - Error handling (in docs/)
```

### **Status Reports** (Root Directory)

```
STATUS.md                       - Current production readiness
00_START_HERE.md                - Project overview
README.md                       - Main project documentation
PRODUCTION_DEPLOYMENT_GUIDE.md  - Deployment instructions
```

### **Audit Reports** (Root Directory)

```
UNIFICATION_TECHNICAL_AUDIT_NOV_10_2025.md  - Full technical audit
UNIFICATION_QUICK_ACTION_GUIDE.md           - Quick action reference
EXECUTIVE_UNIFICATION_SUMMARY.md            - Executive summary
UNIFICATION_FILE_LOCATIONS.md               - This file
```

### **Organized Documentation** (docs/ directory)

```
docs/
├── guides/                    - How-to guides
├── reports/                   - Technical reports
├── sessions/                  - Session notes
└── archive/                   - Historical documentation
```

---

## 🎯 MIGRATION CHECKLISTS

### **async_trait Migration Checklist** (Per File)

```
For each file with async_trait:

[ ] Locate #[async_trait] attribute
[ ] Note trait name and method signatures
[ ] Remove #[async_trait] attribute
[ ] Update trait method signatures:
    - Change: async fn method(...) -> Result<T>
    - To: fn method(...) -> impl Future<Output = Result<T>> + Send
[ ] Find all trait implementations
[ ] Update each implementation:
    - Remove #[async_trait] from impl
    - Wrap body in async move { ... }
[ ] Run: cargo check --package <package>
[ ] Run: cargo test --package <package>
[ ] Benchmark: Compare performance before/after
[ ] Commit: "Migrate [TraitName] from async_trait to native async"
```

### **Documentation Enhancement Checklist** (Per Module)

```
For each priority module:

[ ] Read through module to understand organization
[ ] Identify main types and their relationships
[ ] Create module-level documentation:
    //! Module: [Name]
    //!
    //! [Brief description]
    //!
    //! # Organization
    //! - [Type 1] - Purpose
    //! - [Type 2] - Purpose
    //!
    //! # Examples
    //! ```rust
    //! [Example code]
    //! ```
    //!
    //! # Relationships
    //! - Uses: [Related modules]
    //! - Consumed by: [Consumers]
[ ] Add inline docs for complex types
[ ] Add examples for common use cases
[ ] Cross-reference related modules
[ ] Run: cargo doc --open
[ ] Review generated documentation
[ ] Commit: "Add comprehensive docs for [module]"
```

### **Deprecated Marker Cleanup Checklist** (Per File)

```
For each file with deprecated markers:

[ ] Search for: deprecated, compat, shim, legacy (case-insensitive)
[ ] For each occurrence, determine:
    [ ] Is this a FEATURE? (specialty runtime, compat layer)
        → Keep, update documentation to clarify it's a feature
    [ ] Is this TRANSITIONAL? (migration in progress)
        → Add removal date: // DEPRECATED: Remove after YYYY-MM-DD
    [ ] Is this UNUSED? (old code path)
        → Remove or refactor
[ ] Update any misleading names (e.g., "legacy" → "specialty")
[ ] Run: cargo check
[ ] Run: cargo test
[ ] Commit: "Clean up deprecated markers in [file]"
```

---

## 🚀 QUICK START COMMANDS

### **Start async_trait Migration**

```bash
# 1. Find all instances
grep -rn "#\[async_trait\]" crates --include="*.rs" > async_trait_list.txt

# 2. Start with simplest files (1-2 instances)
grep -c "#\[async_trait\]" crates/**/*.rs | grep ":1$" | cut -d: -f1

# 3. Pick first file and migrate
# 4. Test immediately
cargo test --package [package-name]

# 5. Benchmark (optional, for hot paths)
cargo bench --package [package-name]
```

### **Start Documentation Enhancement**

```bash
# 1. Identify large modules
find crates -name "*.rs" -exec wc -l {} + | sort -rn | head -20

# 2. Pick module with most configs
grep -rn "pub struct.*Config" crates | cut -d: -f1 | sort | uniq -c | sort -rn

# 3. Open in editor and add module docs
$EDITOR crates/cli/src/network_config/types.rs

# 4. Preview documentation
cargo doc --open --package toadstool-cli
```

### **Start Deprecated Cleanup**

```bash
# 1. Find all deprecated markers
grep -rin "deprecated" crates --include="*.rs" > deprecated_list.txt

# 2. Review each and categorize
cat deprecated_list.txt | less

# 3. Clean up one file at a time
$EDITOR crates/runtime/specialty/src/lib.rs

# 4. Test
cargo check && cargo test
```

---

**Last Updated**: November 10, 2025  
**For**: ToadStool Unification Phase  
**Reference**: UNIFICATION_TECHNICAL_AUDIT_NOV_10_2025.md

