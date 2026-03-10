# Multi-Tenant Compute Architecture
**Version**: 1.1  
**Date**: March 10, 2026  
**Status**: Active - Implementation in Progress (S142+ P2)

---

## 1. EXECUTIVE SUMMARY

ToadStool provides **multi-tenant NPU/GPU compute** with two-tier architecture:

- **Tier 1 (Privileged)**: ToadStool core uses kernel drivers for full hardware control
- **Tier 2 (Sandboxed)**: Tenants receive sandboxed userspace drivers with large control but complete isolation

**Goal**: Enable safe compute lending where tenants have full programming capability but cannot:
- Access other tenants' data/models
- Exhaust system resources
- Escape sandbox boundaries
- Exfiltrate data via network

### Deployment Models (S142+)

| Model | Description | Tenant Count | Isolation |
|-------|-------------|-------------|-----------|
| **Local Direct** | Spring runs directly on strandgate GPUs | 1 (self) | None — full access |
| **Local Multi** | Multiple springs share strandgate GPUs | 2-4 | Priority-based quotas |
| **Cloud Rental** | Our GPUs rented to external tenants | N | Full sandbox + quotas |
| **Cloud Consumer** | We rent external GPUs (spot/reserved) | 1 (self) | Checkpointing for preemption |

The same `science.gpu.dispatch` API works in all models. The `ResourceOrchestrator`
decides isolation and allocation based on deployment mode. In local-direct mode
(strandgate), orchestration is trivially "give everything." In cloud-rental mode,
it enforces quotas, priorities, and GPU time-slicing.

**Spring-primal parity**: When hotSpring trusts toadStool's orchestrator to place
and optimize workloads as well as hotSpring does directly, hotSpring focuses
entirely on science. Multi-tenancy is the mechanism that proves this trust — if
toadStool can fairly allocate between competing tenants, a single tenant (a spring)
gets optimal allocation by default.

---

## 2. ARCHITECTURE OVERVIEW

### 2.1 System Layers

```
┌─────────────────────────────────────────────────┐
│  Layer 1: ToadStool Core (Privileged)          │
│  - Uses kernel drivers (/dev/akida*, CUDA)     │
│  - Full hardware control                        │
│  - Resource orchestration                       │
│  - Tenant management                            │
└────────────────┬────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────┐
│  Layer 2: Orchestrator                          │
│  - Resource allocation                          │
│  - Sandbox creation                             │
│  - Quota enforcement                            │
│  - Audit logging                                │
└────────────────┬────────────────────────────────┘
                 │
     ┌───────────┼────────────┐
     │           │            │
┌────▼───┐  ┌───▼────┐  ┌───▼────┐
│ Tenant │  │ Tenant │  │ Tenant │
│   A    │  │   B    │  │   C    │
│(Sandbox│  │(Sandbox│  │(Sandbox│
└────┬───┘  └───┬────┘  └───┬────┘
     │          │            │
┌────▼──────────▼────────────▼────┐
│  Layer 3: Sandboxed Userspace   │
│  - Userspace drivers             │
│  - seccomp + namespaces          │
│  - Resource limits               │
│  - Memory isolation              │
└────────────────┬────────────────┘
                 │
┌────────────────▼────────────────┐
│  Hardware Layer                  │
│  - 2× Akida NPUs                │
│  - NVIDIA/AMD GPUs              │
│  - Sliced/time-shared           │
└──────────────────────────────────┘
```

---

## 3. COMPONENT SPECIFICATIONS

### 3.1 Kernel Driver Manager

**Location**: `crates/neuromorphic/akida-driver/src/kernel/`

**Responsibilities**:
- Initialize NPU devices via kernel module
- Manage DMA transfers
- Handle interrupts
- Monitor hardware health

**API**:
```rust
pub struct KernelDriverManager {
    devices: Vec<AkidaKernelDevice>,
}

impl KernelDriverManager {
    /// Initialize from kernel drivers
    pub fn from_kernel() -> Result<Self>;
    
    /// Get full hardware control
    pub fn device(&mut self, id: usize) -> &mut AkidaKernelDevice;
    
    /// Allocate device slice for tenant
    pub fn create_allocation(&mut self, request: AllocationRequest) 
        -> Result<TenantAllocation>;
}
```

---

### 3.2 Userspace Driver Factory

**Location**: `crates/neuromorphic/akida-driver/src/userspace/`

**Responsibilities**:
- Create constrained userspace drivers
- Map only allocated hardware regions
- Apply memory protections
- Provide tenant API

**API**:
```rust
pub struct UserspaceDriverFactory {
    kernel_manager: Arc<Mutex<KernelDriverManager>>,
}

impl UserspaceDriverFactory {
    /// Create userspace driver for allocation
    pub fn create_for_allocation(
        &self,
        allocation: &TenantAllocation,
    ) -> Result<UserspaceDriver>;
}

pub struct UserspaceDriver {
    device_id: String,
    npu_slice: Range<usize>,
    sram_region: MemoryRegion,
}

impl UserspaceDriver {
    /// Load model to tenant's SRAM slice
    pub fn load_model(&mut self, model: &[u8]) -> Result<()>;
    
    /// Set custom reservoir weights
    pub fn load_reservoir(&mut self, w_in: &Array2<f32>, w_res: &Array2<f32>) -> Result<()>;
    
    /// Run inference on tenant's NPU slice
    pub fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>>;
}
```

---

### 3.3 Sandbox Manager

**Location**: Uses existing `crates/security/sandbox/`

**Integration**:
```rust
pub struct SandboxedTenantDriver {
    driver: UserspaceDriver,
    sandbox: Sandbox,
    limits: ResourceLimits,
}

impl SandboxedTenantDriver {
    pub fn new(
        driver: UserspaceDriver,
        config: SandboxConfig,
    ) -> Result<Self> {
        let sandbox = Sandbox::new(config)?;
        Ok(Self { driver, sandbox, limits })
    }
    
    /// Execute in sandbox
    pub fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut UserspaceDriver) -> Result<T>,
    {
        self.sandbox.exec(|| f(&mut self.driver))
    }
}
```

---

### 3.4 Resource Orchestrator

**Location**: `crates/neuromorphic/akida-orchestrator/`

**Responsibilities**:
- Tenant lifecycle management
- Resource allocation/deallocation
- Quota enforcement
- Billing/metering
- Audit trail

**API**:
```rust
pub struct ResourceOrchestrator {
    kernel_manager: Arc<Mutex<KernelDriverManager>>,
    factory: UserspaceDriverFactory,
    allocations: HashMap<TenantId, TenantAllocation>,
}

impl ResourceOrchestrator {
    /// Allocate resources to tenant
    pub fn allocate(
        &mut self,
        tenant_id: TenantId,
        request: ResourceRequest,
    ) -> Result<TenantToken>;
    
    /// Get tenant's sandboxed driver
    pub fn get_driver(&self, token: &TenantToken) 
        -> Result<SandboxedTenantDriver>;
    
    /// Revoke tenant access
    pub fn deallocate(&mut self, tenant_id: TenantId) -> Result<()>;
}
```

---

## 4. SECURITY MODEL

### 4.1 Memory Isolation

**NPU SRAM Partitioning**:
```
Device 0 (10MB SRAM):
├─ 0x20000000-0x204FFFFF: Tenant A (5MB)
└─ 0x20500000-0x209FFFFF: Tenant B (5MB)

Device 1 (10MB SRAM):
└─ 0x20000000-0x209FFFFF: Tenant C (10MB)
```

**Enforcement**:
```rust
pub struct MemoryRegion {
    base: usize,
    size: usize,
}

impl UserspaceDriver {
    fn validate_access(&self, offset: usize, size: usize) -> Result<()> {
        if offset < self.sram_region.base ||
           offset + size > self.sram_region.base + self.sram_region.size
        {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}
```

---

### 4.2 Sandbox Configuration

**Per-Tenant Profile**:
```rust
pub fn tenant_sandbox_config(allocation: &TenantAllocation) -> SandboxConfig {
    SandboxConfig {
        // Allowed syscalls (minimal)
        allow_syscalls: vec![
            Syscall::Read,
            Syscall::Write,
            Syscall::Mmap,    // Only for their region
            Syscall::Munmap,
            Syscall::Brk,     // Memory management
        ],
        
        // Denied syscalls (security)
        deny_syscalls: vec![
            Syscall::Socket,   // No network
            Syscall::Connect,
            Syscall::Fork,     // No spawning
            Syscall::Clone,
            Syscall::Execve,   // No execution
            Syscall::Ptrace,   // No debugging
            Syscall::Open,     // No file access
        ],
        
        // Memory mapping whitelist
        mmap_whitelist: vec![allocation.sram_region.clone()],
        
        // Resource limits
        limits: ResourceLimits {
            max_memory_mb: allocation.memory_limit_mb,
            max_cpu_percent: allocation.cpu_quota,
            max_pids: 10,
            max_open_files: 100,
        },
        
        // Namespace isolation
        namespaces: Namespaces {
            pid: true,   // Isolated process tree
            net: true,   // No network
            uts: true,   // Isolated hostname
            ipc: true,   // Isolated IPC
            mount: true, // Isolated mounts
        },
    }
}
```

---

### 4.3 Security Guarantees

| Threat | Mitigation |
|--------|------------|
| **Data theft** | Memory isolation + mmap whitelist |
| **Model stealing** | Separate SRAM regions, no cross-tenant access |
| **Resource exhaustion** | cgroups v2 CPU/memory limits |
| **Network exfiltration** | seccomp blocks all network syscalls |
| **Privilege escalation** | No setuid, no capabilities, sandboxed |
| **Side channels** | Time-slicing randomization, constant-time ops |

---

## 5. RESOURCE ALLOCATION

### 5.1 Allocation Request

```rust
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Number of NPUs requested
    pub npus: usize,
    
    /// SRAM in MB
    pub sram_mb: usize,
    
    /// Duration (for billing)
    pub duration: Duration,
    
    /// Priority level
    pub priority: Priority,
}

#[derive(Debug, Clone)]
pub enum Priority {
    Low,       // Best-effort, can be preempted
    Normal,    // Standard allocation
    High,      // Reserved, guaranteed
}
```

---

### 5.2 Allocation Strategy

**Spatial Partitioning** (preferred for NPU):
```rust
// Each tenant gets fixed NPU slice
Tenant A: NPUs 0-39 on Device 0
Tenant B: NPUs 40-79 on Device 0
Tenant C: NPUs 0-79 on Device 1
```

**Time Slicing** (for GPU, or oversubscribed NPU):
```rust
// Tenants share hardware with time quotas
Tenant A: 33% time quota (1s per 3s)
Tenant B: 33% time quota (1s per 3s)
Tenant C: 33% time quota (1s per 3s)
```

---

## 6. API SPECIFICATION

### 6.1 Tenant-Facing API

```rust
/// Tenant's view of allocated NPU
pub struct TenantNpuAccess {
    driver: SandboxedTenantDriver,
    allocation: TenantAllocation,
}

impl TenantNpuAccess {
    /// Get allocation info
    pub fn info(&self) -> &AllocationInfo;
    
    /// Load model
    pub fn load_model(&mut self, model: &[u8]) -> Result<ModelHandle>;
    
    /// Set reservoir weights
    pub fn load_reservoir(
        &mut self,
        w_in: &Array2<f32>,
        w_res: &Array2<f32>,
    ) -> Result<()>;
    
    /// Run inference
    pub fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>>;
    
    /// Get resource usage
    pub fn usage(&self) -> ResourceUsage;
    
    /// Measure power consumption
    pub fn power_usage(&self) -> Result<f32>;
}
```

---

### 6.2 Admin API

```rust
/// ToadStool owner's management interface
pub struct AdminInterface {
    orchestrator: ResourceOrchestrator,
}

impl AdminInterface {
    /// List all tenants
    pub fn list_tenants(&self) -> Vec<TenantInfo>;
    
    /// Allocate to new tenant
    pub fn create_allocation(
        &mut self,
        tenant_id: TenantId,
        request: ResourceRequest,
    ) -> Result<TenantToken>;
    
    /// Revoke allocation
    pub fn revoke(&mut self, tenant_id: TenantId) -> Result<()>;
    
    /// Get system status
    pub fn system_status(&self) -> SystemStatus;
    
    /// View audit logs
    pub fn audit_logs(&self, tenant_id: Option<TenantId>) 
        -> Result<Vec<AuditEntry>>;
}
```

---

## 7. IMPLEMENTATION PHASES

### Phase 1: Foundation (Week 1)
- ✅ Load kernel driver (script → Rust binary)
- ✅ Implement `KernelDriverManager`
- ✅ Test full hardware access
- ✅ Validate DMA, interrupts

**Deliverable**: Rust binary for NPU initialization

---

### Phase 2: Userspace Driver (Week 2)
- ✅ Implement `UserspaceDriver` with mmap
- ✅ Add memory region constraints
- ✅ Test basic NPU operations
- ✅ Validate isolation

**Deliverable**: Constrained userspace driver library

---

### Phase 3: Sandbox Integration (Week 3)
- ✅ Integrate with existing `crates/security/sandbox/`
- ✅ Implement `SandboxedTenantDriver`
- ✅ Configure seccomp profiles
- ✅ Test escape prevention

**Deliverable**: Sandboxed tenant driver

---

### Phase 4: Orchestrator (Week 4)
- ✅ Implement `ResourceOrchestrator`
- ✅ Add allocation/deallocation logic
- ✅ Implement quota enforcement
- ✅ Add audit logging

**Deliverable**: Complete multi-tenant system

---

## 8. TESTING STRATEGY

### 8.1 Unit Tests
- Memory region validation
- Sandbox configuration generation
- Resource allocation logic
- Quota calculations

### 8.2 Integration Tests
- Multi-tenant isolation verification
- Cross-tenant data leakage tests
- Resource exhaustion prevention
- Sandbox escape attempts

### 8.3 Security Tests
- Side-channel analysis
- Timing attacks
- Memory corruption attempts
- Privilege escalation attempts

---

## 9. FUTURE ENHANCEMENTS

### 9.1 GPU Support
Extend to NVIDIA/AMD GPUs:
- Userspace Vulkan compute for tenants
- Time-slicing via GPU scheduler
- Memory isolation via separate contexts

### 9.2 Marketplace
- Automated resource rental
- Dynamic pricing
- Reputation system
- SLA enforcement

### 9.3 Federation
- Multi-host orchestration
- Cross-system workload migration
- Distributed tenant management

---

## 10. REFERENCES

- [SECURITY_SANDBOXING.md](./SECURITY_SANDBOXING.md) - Existing sandbox infrastructure
- [UNIVERSAL_COMPUTE_ORCHESTRATOR.md](./UNIVERSAL_COMPUTE_ORCHESTRATOR.md) - Runtime execution
- Linux seccomp: https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html
- cgroups v2: https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html

---

**Status**: Specification complete, implementation ready to begin  
**Next**: Implement kernel driver manager in Rust (no scripts!)
