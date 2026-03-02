# ToadStool Types Reference Guide
## Canonical Type Definitions and Usage Patterns

**Last Updated**: March 1, 2026  
**Status**: Production-Ready

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

### 1.4 Adapter Type: `src/universal_adapter.rs::ResourceRequirements`

**Location**: `src/universal_adapter.rs`  
**Status**: ✅ **Unified with conversions** (November 11, 2025)

**Definition**:
```rust
pub struct ResourceRequirements {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: u32,
    pub storage_bytes: u64,
    pub network_bandwidth: Option<String>,
}
```

**Purpose**: Simplified resource requirements for universal adapter coordination

**Conversions**:
- ✅ `From<ResourceRequirements> for toadstool::resources::ResourceRequirements`
- ✅ `From<toadstool::resources::ResourceRequirements> for ResourceRequirements`

**Notes**:
- Flat structure for easier adapter usage
- Converts to/from canonical nested structure
- Used in `WorkloadSpec` and coordination traits
- Helper function `parse_bandwidth_string()` for bandwidth parsing
- Bandwidth strings: "100Mbps", "1Gbps", "500Kbps", or "1000000bps"

---

### 1.5 System Resources

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

## 7. Module Structure Reference

The following modules were refactored from single-file layouts into multi-file module structures. Use these paths when locating types or extending functionality.

### 7.1 `toadstool_common::primal_integration`

**Location**: `crates/core/common/src/primal_integration/`

**Structure**:
- `mod.rs` — Module root, re-exports, integration patterns
- `capabilities.rs` — Capability definitions and discovery helpers
- `socket.rs` — Unix socket / IPC primitives
- `discovery.rs` — Runtime discovery of ecoPrimal services by capability
- `tests.rs` — Integration tests

**Purpose**: Inter-Primal integration discovery; capability-based runtime discovery of ecoPrimal services.

---

### 7.2 `toadstool_common::capability_provider`

**Location**: `crates/core/common/src/capability_provider/`

**Structure**:
- `mod.rs` — Module root, re-exports (`discover_all`, `CapabilityError`, `CapabilityProvider`)
- `error.rs` — `CapabilityError` and `Result` type
- `serialize.rs` — Capability serialization/deserialization
- `discovery.rs` — Capability discovery logic
- `provider.rs` — `CapabilityProvider` implementation

**Purpose**: Capability-based service discovery and invocation; primals discover each other by capability at runtime.

---

### 7.3 `integration::primals` (toadstool-primals)

**Location**: `crates/integration/primals/src/`

**Structure**:
- `lib.rs` — Crate root, `PrimalIntegration` trait, re-exports
- `primal_types.rs` — `PrimalType`, `PrimalConfig`, `PrimalResources`, `GpuAllocation`
- `service.rs` — `ServiceEndpoint`, `ServiceRegistration`, `StartupResult`, `StartupStatus`
- `health.rs` — `HealthCheck`, `HealthCheckStatus`, `HealthStatus`
- `messaging.rs` — `PrimalMessage`, `PrimalMessageType`, `PrimalMetrics`
- `integration_manifest.rs` — `BiomeManifest`, `BiomeMetadata`
- `manager.rs` — `PrimalIntegrationManager`, `PrimalIntegrationConfig`, `BootstrapResult`, `PrimalBootstrapResult`

**Purpose**: Universal Primal integration framework; consistent interface for integrating with all Primals in the ecoPrimals ecosystem.

---

### 7.4 `toadstool::workload`

**Location**: `crates/core/toadstool/src/workload/`

**Structure**:
- `mod.rs` — Module root, workload orchestration
- `types.rs` — Workload type definitions and related structures

**Purpose**: Workload specification and orchestration types (refactored from single `workload.rs`).

---

### 7.5 `barracuda::device`

**Location**: `crates/barracuda/src/device/`

**Structure** (refactored from `unified.rs`):
- `device_types.rs` — Device type definitions (new)
- `routing.rs` — Device routing logic (new)
- `capabilities.rs` — Extended device capabilities
- `unified.rs` — Unified device interface (retained, delegates to above)
- `mod.rs` — Module root, re-exports

**Purpose**: GPU device abstraction; types, routing, and capabilities split from unified module for clarity.

---

### 7.6 `barracuda::shaders::precision`

**Location**: `crates/barracuda/src/shaders/precision/`

**Structure**:
- `mod.rs` — Module root, precision types and re-exports
- `compiler.rs` — Precision shader compilation (split from mod)
- `polyfill.rs` — Precision polyfills for WGSL (split from mod)
- `math_f64.rs`, `templates.rs`, `cpu.rs` — Supporting modules
- `precision_tests.rs`, `precision_chaos_tests.rs` — Tests

**Purpose**: Shader precision handling; compiler and polyfill logic extracted from monolithic mod.

---

### 7.7 `runtime::gpu::backends::opencl_impl`

**Location**: `crates/runtime/gpu/src/backends/opencl_impl/`

**Structure**:
- `mod.rs` — Module root, re-exports
- `backend.rs` — OpenCL backend implementation
- `resource.rs` — Resource management
- `context.rs` — Context handling
- `kernels.rs` — Kernel dispatch and management
- `tests.rs` — Backend tests

**Purpose**: OpenCL backend for GPU runtime; split from single `opencl_impl.rs` for maintainability.

---

### 7.8 `core::config::runtime_defaults::env_overrides`

**Location**: `crates/core/config/src/runtime_defaults/env_overrides/`

**Structure**:
- `mod.rs` — Module root, re-exports
- `parse.rs` — Environment variable parsing
- `app.rs` — Application-level overrides
- `network.rs` — Network configuration overrides
- `resources.rs` — Resource-related overrides
- `features.rs` — Feature flags overrides
- `runtime.rs` — Runtime configuration overrides
- `security.rs` — Security-related overrides
- `logging.rs` — Logging configuration overrides
- `tests.rs` — Unit tests

**Purpose**: Environment-based runtime configuration overrides; split by domain for clarity.

---

## 8. See Also

- `CONFIG_PATTERNS_GUIDE.md` - Configuration composition patterns
- `CONSTANTS_REFERENCE.md` - Default constants and thresholds
- `00_START_HERE.md` - Project overview and status
- `STATUS.md` - Current production readiness metrics

---

## 9. Feedback and Updates

This reference is a living document. When types change:

1. Update this document
2. Add entry to `CHANGELOG.md`
3. Update affected conversion implementations
4. Run full test suite to verify

**Questions?** Check the inline documentation in the source files or ask the team.

