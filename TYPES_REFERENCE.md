# ToadStool Types Reference Guide
## Canonical Type Definitions and Usage Patterns

**Last Updated**: November 9, 2025  
**Status**: Production-Ready ✅

---

## Overview

This document provides a comprehensive reference for all canonical types in the ToadStool codebase. After the November 2025 type system unification effort, all types now have clear ownership and single sources of truth.

## Core Principles

1. **Single Source of Truth**: Each logical type has one canonical definition
2. **Conversion Implementations**: Bidirectional `From` traits between equivalent types
3. **Backward Compatibility**: Legacy types preserved where necessary
4. **Clear Documentation**: Each type documents its purpose and relationships

---

## 1. Resource Types

### 1.1 Canonical: `toadstool::resources::ResourceRequirements`

**Location**: `crates/core/toadstool/src/resources.rs`

**Definition**:
```rust
pub struct ResourceRequirements {
    pub cpu: CpuRequirements,
    pub memory: MemoryRequirements,
    pub storage: StorageRequirements,
    pub gpu: Option<GpuRequirements>,
    pub network: NetworkRequirements,
}
```

**Purpose**: Comprehensive resource specification with detailed sub-structures

**Use Cases**:
- Internal runtime resource management
- Detailed resource allocation and monitoring
- System resource queries

**Sub-Types**:
```rust
pub struct CpuRequirements {
    pub min_cores: f64,
    pub max_cores: Option<f64>,
    pub architecture: Option<String>,
}

pub struct MemoryRequirements {
    pub min_bytes: u64,
    pub max_bytes: Option<u64>,
}

pub struct StorageRequirements {
    pub min_bytes: u64,
    pub max_bytes: Option<u64>,
    pub storage_type: Option<String>,
}

pub struct GpuRequirements {
    pub min_units: u32,
    pub max_units: Option<u32>,
    pub gpu_type: Option<String>,
    pub min_memory_bytes: Option<u64>,
}

pub struct NetworkRequirements {
    pub min_bandwidth: Option<u64>,
    pub max_bandwidth: Option<u64>,
    pub max_latency_ms: Option<u64>,
}
```

---

### 1.2 Domain-Specific: `distributed::ResourceRequirements`

**Location**: `crates/distributed/src/types/resources.rs`

**Definition**:
```rust
pub struct ResourceRequirements {
    pub cpu: CpuRequirements,           // min_cores, max_cores
    pub memory: MemoryRequirements,     // min_bytes, max_bytes
    pub storage: StorageRequirements,   // min_bytes, max_bytes
    pub network: NetworkRequirements,   // bandwidth_mbps, latency_ms
    pub gpu: Option<GpuRequirements>,   // min_memory_gb, compute_capability
}
```

**Purpose**: Distributed execution resource specification

**Conversions**:
- ✅ `From<distributed::ResourceRequirements> for toadstool::resources::ResourceRequirements`
- ✅ `From<toadstool::resources::ResourceRequirements> for distributed::ResourceRequirements`

**Notes**:
- Slightly different field names (e.g., `bandwidth_mbps` vs `min_bandwidth` in bytes/sec)
- GPU specified by memory in GB rather than units
- Conversions handle unit translations automatically

---

### 1.3 Client-Facing: `client::ResourceRequirements`

**Location**: `crates/client/src/client/types.rs`

**Definition**:
```rust
pub struct ResourceRequirements {
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub gpu_required: Option<bool>,
}
```

**Purpose**: Simplified client API for workload submission

**Conversions**:
- ✅ `From<client::ResourceRequirements> for toadstool::resources::ResourceRequirements`
- ✅ `From<toadstool::resources::ResourceRequirements> for client::ResourceRequirements`

**Notes**:
- All fields are `Option` for ease of use
- GPU is a simple boolean flag
- Units are in MB/cores for simplicity
- Conversions provide sensible defaults (1 core, 1GB RAM if not specified)

---

### 1.4 System Resources

#### Canonical: `toadstool::resources::SystemResources`

**Location**: `crates/core/toadstool/src/resources.rs`

**Definition**:
```rust
pub struct SystemResources {
    pub available_cpu_cores: f64,
    pub available_memory_bytes: u64,
    pub available_storage_bytes: u64,
    pub available_network_bandwidth: Option<u64>,
    pub available_gpu_units: u32,
}
```

**Purpose**: Current system resource availability for scheduling decisions

---

#### Universal: `toadstool::universal::UniversalSystemResources`

**Location**: `crates/core/toadstool/src/universal.rs`

**Definition**:
```rust
pub struct UniversalSystemResources {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bandwidth: u64,
    pub gpu_units: u32,
    pub special_hardware: HashMap<String, u32>,
}
```

**Purpose**: Universal compute platform resource tracking (includes special hardware)

**Notes**:
- Renamed from `SystemResources` in November 2025 to avoid naming collision
- Used by `UniversalScheduler` and `ResourceCoordinator`
- Includes special hardware mapping for exotic platforms

---

## 2. Job Priority

### 2.1 Canonical: `toadstool::JobPriority`

**Location**: `crates/core/toadstool/src/universal.rs`  
**Re-exported**: `crates/core/toadstool/src/lib.rs`

**Definition**:
```rust
pub enum JobPriority {
    Emergency = 0,   // Highest priority
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
    Background = 5,  // Lowest priority
}
```

**Purpose**: Standard priority levels for job scheduling

**Key Properties**:
- **Ordering**: Lower number = higher priority (standard for priority queues)
- **Derives**: `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize`

**Usage Across Codebase**:
- ✅ `crates/core/toadstool/src/universal.rs` - Canonical definition
- ✅ `crates/distributed/src/types/jobs.rs` - Re-exported
- ✅ `crates/client/src/client/types.rs` - Re-exported

**Migration Notes**:
- **Before**: 4 different definitions with inconsistent ordering
- **After**: Single canonical definition, all others removed or converted

---

### 2.2 Legacy: `legacy::JobPriority`

**Location**: `crates/runtime/legacy/src/types/configs.rs`

**Definition**:
```rust
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
    RealTime,  // Maps to Emergency in canonical
}
```

**Purpose**: Backward compatibility with legacy systems

**Conversions**:
- ✅ `From<legacy::JobPriority> for toadstool::JobPriority`
- ✅ `From<toadstool::JobPriority> for legacy::JobPriority`

**Mapping**:
| Legacy | Canonical |
|--------|-----------|
| `RealTime` | `Emergency` |
| `Critical` | `Critical` |
| `High` | `High` |
| `Normal` | `Normal` |
| `Low` | `Low` |
| (none) | `Background` → `Low` |

---

## 3. Job and Workload Types

### 3.1 Universal Jobs

#### `toadstool::UniversalJobType`

**Location**: `crates/core/toadstool/src/universal.rs`

**Definition**:
```rust
pub enum UniversalJobType {
    Native {
        executable: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Wasm {
        module: Vec<u8>,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Primal {
        primal_type: String,
        endpoint: String,
        payload: serde_json::Value,
    },
    BiomeOS {
        biome_manifest: serde_json::Value,
        team_id: String,
    },
}
```

**Purpose**: Core job types for universal compute platform

---

#### `distributed::UniversalJobType`

**Location**: `crates/distributed/src/types/jobs.rs`

**Definition**:
```rust
pub enum UniversalJobType {
    // Execution targets
    Local,
    RemoteToadStool { endpoint: String },
    EcosystemTool { tool_name: String, endpoint: String },
    RecursiveHosting { toadstool_config: ToadStoolHostingConfig },
    OSLayerCompatibility { compatibility_mode: CompatibilityMode },
    
    // Resource classifications
    ComputeIntensive,
    MemoryIntensive,
    NetworkIntensive,
    StorageIntensive,
    Hybrid,
    
    // Workload types
    DataProcessing,
    MachineLearning,
    Simulation,
    Native,
    Container,
    WASM,
    GPU,
    Custom(String),
}
```

**Purpose**: Extended job classification for distributed scheduling

**Notes**:
- More detailed than core `UniversalJobType`
- Includes resource usage hints for scheduler optimization
- Includes execution target information

---

### 3.2 Client Workload Types

**Location**: `crates/client/src/client/types.rs`

**Definition**:
```rust
pub enum WorkloadType {
    Native {
        executable: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
        working_dir: Option<String>,
    },
    Wasm {
        module_data: Vec<u8>,
        args: Vec<String>,
    },
    Python {
        script: String,
        requirements: Vec<String>,
    },
    Custom {
        workload_data: serde_json::Value,
    },
}
```

**Purpose**: Client-facing workload specification API

**Notes**:
- Includes `working_dir` for better UX
- Python workloads are first-class
- Custom type for extensibility

---

## 4. Usage Guidelines

### 4.1 When to Use Each Type

#### Use Canonical Types For:
- ✅ New internal code
- ✅ Core runtime logic
- ✅ Type-safe interfaces between modules

#### Use Domain-Specific Types For:
- ✅ Domain-specific optimizations (e.g., distributed scheduling)
- ✅ External protocol compatibility
- ✅ Simplified client APIs

#### Use Conversions When:
- ✅ Crossing module boundaries
- ✅ Adapting external data to internal format
- ✅ Providing backward compatibility

---

### 4.2 Conversion Best Practices

```rust
// ✅ GOOD: Explicit conversion at boundaries
fn submit_workload(client_req: client::ResourceRequirements) -> Result<ExecutionId> {
    let core_req: toadstool::resources::ResourceRequirements = client_req.into();
    scheduler.schedule(core_req)
}

// ❌ BAD: Mixing types without conversion
fn schedule_job(job: distributed::UniversalJob, resources: client::ResourceRequirements) {
    // Type mismatch! Need conversion first
}

// ✅ GOOD: Use canonical types in internal APIs
fn allocate_resources(req: &toadstool::resources::ResourceRequirements) -> Result<Allocation> {
    // Internal function uses canonical type
}
```

---

### 4.3 Adding New Types

When adding a new type to the system:

1. **Determine if it's truly new** or can use an existing canonical type
2. **If new, decide placement**:
   - Core types → `crates/core/toadstool/src/`
   - Domain-specific → relevant domain crate
3. **Document the type** in this reference
4. **Add conversions** if it overlaps with existing types
5. **Update exports** in `lib.rs` files
6. **Add tests** for conversions

---

## 5. Type Hierarchy Visualization

```
Core (toadstool)
├── resources::ResourceRequirements (canonical)
│   ├── CpuRequirements
│   ├── MemoryRequirements
│   ├── StorageRequirements
│   ├── GpuRequirements
│   └── NetworkRequirements
│
├── resources::SystemResources (current availability)
│
├── universal::UniversalSystemResources (universal platform)
│
├── universal::JobPriority (canonical)
│   ├── Emergency (0)
│   ├── Critical (1)
│   ├── High (2)
│   ├── Normal (3)
│   ├── Low (4)
│   └── Background (5)
│
└── universal::UniversalJobType
    ├── Native
    ├── Wasm
    ├── Primal
    └── BiomeOS

Distributed (toadstool-distributed)
├── types::ResourceRequirements → converts to/from core
├── types::JobPriority → re-exports core
└── types::UniversalJobType (extended)

Client (toadstool-client)
├── types::ResourceRequirements → converts to/from core
├── types::JobPriority → re-exports core
└── types::WorkloadType

Legacy (toadstool-runtime-legacy)
└── types::JobPriority → converts to/from core
```

---

## 6. Migration Status

| Component | Old State | New State | Status |
|-----------|-----------|-----------|--------|
| `SystemResources` | Name collision | `UniversalSystemResources` | ✅ Complete |
| `JobPriority` | 4 definitions | 1 canonical + 1 legacy | ✅ Complete |
| `ResourceRequirements` | 3 separate types | 1 canonical + conversions | ✅ Complete |
| `UniversalJobType` | 2 definitions | 2 (different purposes) | ✅ Clarified |

---

## 7. See Also

- `CONFIG_PATTERNS_GUIDE.md` - Configuration composition patterns
- `CONSTANTS_REFERENCE.md` - Default constants and thresholds
- `00_START_HERE.md` - Project overview and status
- `STATUS.md` - Current production readiness metrics

---

## 8. Feedback and Updates

This reference is a living document. When types change:

1. Update this document
2. Add entry to `CHANGELOG.md`
3. Update affected conversion implementations
4. Run full test suite to verify

**Questions?** Check the inline documentation in the source files or ask the team.

