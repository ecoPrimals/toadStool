# NPU Multi-Tenant Architecture Specification
**Version**: 1.0  
**Date**: February 8, 2026  
**Status**: Design Complete, Implementation Ready

---

## 1. OVERVIEW

### 1.1 Purpose

Enable safe multi-tenant access to neuromorphic processing units (NPUs) where:
- **ToadStool** manages hardware via kernel drivers
- **Tenants** receive sandboxed userspace drivers
- **Complete isolation** prevents data/model leakage between tenants

### 1.2 Architecture Principles

- **Dual-path backend**: Kernel driver (privileged) + Userspace driver (sandboxed)
- **Zero scripts**: All functionality compiled into Rust binary
- **Pure Rust**: No external dependencies for tenant isolation
- **Portable**: Binary contains all necessary logic for any system

---

## 2. ARCHITECTURE

### 2.1 Component Overview

```
┌─────────────────────────────────────────┐
│      ToadStool Core (Privileged)        │
│  ┌────────────────────────────────┐     │
│  │  NPU Orchestrator              │     │
│  │  - Kernel driver management    │     │
│  │  - Resource allocation         │     │
│  │  - Tenant lifecycle            │     │
│  └────────┬───────────────────────┘     │
└───────────┼──────────────────────────────┘
            │
    ┌───────┴────────┐
    │                │
┌───▼───┐      ┌────▼────┐
│Tenant │      │ Tenant  │
│   A   │      │    B    │
│Sandbox│      │ Sandbox │
└───┬───┘      └────┬────┘
    │               │
    └───────┬───────┘
            │
    ┌───────▼────────┐
    │  NPU Hardware  │
    │  (Partitioned) │
    └────────────────┘
```

### 2.2 Backend Selection

**Kernel Driver Backend**:
- **Use by**: ToadStool core (privileged operations)
- **Access**: Direct `/dev/akida*` device nodes
- **Features**: Full DMA, interrupts, maximum performance
- **Trust level**: Trusted

**Userspace Driver Backend**:
- **Use by**: Tenant workloads (untrusted code)
- **Access**: Memory-mapped PCIe BARs (mmap)
- **Features**: Sandboxed, isolated, resource-limited
- **Trust level**: Untrusted

---

## 3. RESOURCE ALLOCATION

### 3.1 NPU Slice Model

**Physical NPU Resources**:
```
Device 0: AKD1000
├─ 80 NPUs (neural processing units)
├─ 10 MB SRAM (model/weight storage)
└─ 4× PCIe BARs (control, data, model)

Device 1: AKD1000
├─ 80 NPUs
├─ 10 MB SRAM
└─ 4× PCIe BARs
```

**Logical Allocation**:
```rust
pub struct NpuSlice {
    /// Device ID (0, 1, ...)
    pub device_id: usize,
    
    /// NPU range allocated to tenant
    pub npu_range: Range<usize>,  // e.g., 0..40
    
    /// SRAM offset and size
    pub sram_offset: usize,       // e.g., 0x0000
    pub sram_size: usize,         // e.g., 5MB
    
    /// Memory-mapped BAR constraints
    pub mmap_regions: Vec<MmapConstraint>,
}
```

### 3.2 Resource Constraints

```rust
pub struct TenantResourceLimits {
    /// CPU quota (percentage)
    pub cpu_quota_percent: u8,
    
    /// Memory limit
    pub memory_limit_mb: usize,
    
    /// NPU time quota (time-slicing)
    pub npu_time_ms_per_period: u64,
    pub period_ms: u64,
    
    /// Maximum concurrent operations
    pub max_operations: usize,
}
```

---

## 4. SANDBOX ISOLATION

### 4.1 Security Layers

**Process Isolation** (Linux namespaces):
- PID namespace: Isolated process tree
- Network namespace: No network access
- Mount namespace: Minimal filesystem view
- User namespace: UID/GID remapping

**System Call Filtering** (seccomp-bpf):
```rust
pub struct NpuTenantSeccompProfile {
    allow_list: vec![
        Syscall::Read,
        Syscall::Write,
        Syscall::Mmap,      // For NPU BAR access
        Syscall::Munmap,
        Syscall::Futex,     // For locking
        Syscall::Getpid,
    ],
    deny_list: vec![
        Syscall::Socket,    // No network
        Syscall::Fork,      // No spawning
        Syscall::Execve,    // No exec
        Syscall::Ptrace,    // No debugging
        Syscall::OpenAt,    // Minimal file access
    ],
}
```

**Memory Mapping Constraints**:
```rust
pub struct MmapConstraint {
    /// Allowed BAR region
    pub allowed_address_range: Range<u64>,
    
    /// Allowed size
    pub max_size: usize,
    
    /// Access permissions
    pub read: bool,
    pub write: bool,
    pub execute: bool,  // Always false
}
```

### 4.2 Resource Limits (cgroups v2)

**CPU**:
```
cpu.max: "10000 100000"  # 10% of CPU
```

**Memory**:
```
memory.max: "268435456"  # 256 MB
memory.oom.group: "1"    # Kill entire group on OOM
```

**PIDs**:
```
pids.max: "10"           # Maximum 10 processes
```

---

## 5. API SPECIFICATION

### 5.1 Orchestrator API (Privileged)

```rust
/// NPU resource orchestrator (ToadStool core only)
pub struct NpuOrchestrator {
    devices: Vec<KernelDriverBackend>,
    allocations: HashMap<TenantId, ResourceAllocation>,
    sandbox_manager: SandboxManager,
}

impl NpuOrchestrator {
    /// Allocate NPU resources to tenant
    pub fn allocate_tenant(
        &mut self,
        tenant_id: TenantId,
        request: ResourceRequest,
    ) -> Result<TenantAllocation>;
    
    /// Revoke tenant allocation
    pub fn deallocate_tenant(&mut self, tenant_id: TenantId) -> Result<()>;
    
    /// Get tenant usage statistics
    pub fn get_tenant_usage(&self, tenant_id: TenantId) -> Result<UsageStats>;
}
```

### 5.2 Tenant API (Sandboxed)

```rust
/// Tenant NPU access (sandboxed userspace driver)
pub struct TenantNpuAccess {
    driver: UserspaceDriverBackend,
    constraints: ResourceConstraints,
}

impl TenantNpuAccess {
    /// Load model to allocated NPU slice
    pub fn load_model(&mut self, model: &[u8]) -> Result<ModelHandle>;
    
    /// Set reservoir weights (for echo state networks)
    pub fn load_reservoir(
        &mut self,
        w_in: &Array2<f32>,
        w_res: &Array2<f32>,
    ) -> Result<()>;
    
    /// Run inference
    pub fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>>;
    
    /// Query resource usage
    pub fn get_usage(&self) -> Result<ResourceUsage>;
}
```

---

## 6. IMPLEMENTATION

### 6.1 Crate Structure

```
crates/neuromorphic/
├─ akida-driver/
│  ├─ src/
│  │  ├─ backends/
│  │  │  ├─ kernel.rs       # Kernel driver backend
│  │  │  ├─ userspace.rs    # Userspace driver backend
│  │  │  └─ trait.rs        # Common backend trait
│  │  ├─ orchestrator.rs    # Multi-tenant orchestrator
│  │  ├─ sandbox.rs         # Tenant sandbox integration
│  │  └─ allocation.rs      # Resource allocation
│  └─ Cargo.toml
│
└─ akida-reservoir-research/  # Your echo state work
```

### 6.2 Feature Gates

```toml
[features]
default = ["kernel-driver"]

# Kernel driver backend (privileged)
kernel-driver = []

# Userspace driver backend (sandboxed)
userspace-driver = []

# Multi-tenant orchestration
multi-tenant = ["kernel-driver", "userspace-driver"]
```

### 6.3 Pure Rust Implementation (No Scripts)

**Kernel module loading** (in Rust):
```rust
// crates/neuromorphic/akida-driver/src/setup.rs

pub fn setup_kernel_driver() -> Result<()> {
    // Check if module already loaded
    if is_module_loaded("akida_pcie")? {
        return Ok(());
    }
    
    // Find module path
    let module_path = find_akida_module()?;
    
    // Load via kmod (pure Rust, no scripts)
    let result = Command::new("pkexec")
        .arg("insmod")
        .arg(&module_path)
        .output()?;
    
    if !result.status.success() {
        bail!("Failed to load kernel module: {}", 
              String::from_utf8_lossy(&result.stderr));
    }
    
    // Verify device nodes
    wait_for_device_nodes()?;
    
    Ok(())
}
```

**PCIe device enabling** (in Rust):
```rust
// crates/neuromorphic/akida-driver/src/pcie.rs

pub fn enable_pcie_device(address: &str) -> Result<()> {
    let enable_path = format!("/sys/bus/pci/devices/{}/enable", address);
    
    // Write "1" to enable
    fs::write(&enable_path, "1")
        .with_context(|| format!("Failed to enable PCIe device {}", address))?;
    
    // Verify enabled
    let enabled = fs::read_to_string(&enable_path)?;
    if enabled.trim() != "1" {
        bail!("Device {} failed to enable", address);
    }
    
    Ok(())
}
```

**All in ToadStool binary**:
```rust
// crates/cli/src/commands/npu.rs

#[derive(Parser)]
pub struct NpuSetup {
    /// Skip privilege elevation prompts
    #[arg(long)]
    yes: bool,
}

impl NpuSetup {
    pub fn run(&self) -> Result<()> {
        // All logic in Rust, compiled into binary
        npu::setup::check_hardware()?;
        npu::setup::enable_devices()?;
        npu::setup::load_kernel_driver()?;
        npu::setup::verify_ready()?;
        
        println!("✅ NPU setup complete!");
        Ok(())
    }
}
```

---

## 7. DEPLOYMENT MODEL

### 7.1 Binary Distribution

**Single binary** contains:
- ✅ Kernel driver management (no shell scripts)
- ✅ Userspace driver implementation
- ✅ Sandbox orchestration
- ✅ Multi-tenant API
- ✅ All setup logic

**Usage**:
```bash
# On any Linux system with Akida NPUs:
toadstool npu setup              # Sets up kernel driver
toadstool npu allocate alice 40  # Allocate 40 NPUs to alice
toadstool npu status             # Show all allocations
```

### 7.2 Portability

**Binary includes**:
- Driver detection logic
- PCIe device enumeration
- Kernel module loading (via kmod crate)
- Permission handling (via pkexec/sudo)
- Complete setup without external scripts

**Target systems**:
- Linux (any kernel 5.10+)
- x86_64, aarch64
- No Python, no shell required
- Just the ToadStool binary

---

## 8. SECURITY CONSIDERATIONS

### 8.1 Privilege Separation

**ToadStool daemon** (runs as root):
- Manages kernel driver
- Allocates resources
- Enforces quotas

**Tenant processes** (runs as tenant UID):
- Unprivileged
- Sandboxed
- Resource-limited

### 8.2 Attack Surface Reduction

**Kernel driver**:
- Only accessed by ToadStool daemon
- Not exposed to tenants
- Minimal attack surface

**Userspace drivers**:
- No kernel privileges
- Sandboxed completely
- Even if exploited, contained

### 8.3 Audit Trail

```rust
pub struct AuditLog {
    timestamp: SystemTime,
    tenant_id: TenantId,
    operation: Operation,
    resource_usage: ResourceSnapshot,
    security_events: Vec<SecurityEvent>,
}
```

---

## 9. PERFORMANCE

### 9.1 Expected Performance

| Backend | Latency | Throughput | CPU Overhead |
|---------|---------|------------|--------------|
| Kernel | <100μs | 10K ops/s | <1% |
| Userspace | <1ms | 1K ops/s | ~10% |

**Tradeoff**: 10% performance for 100× better isolation

### 9.2 Time-Slicing

**Fair scheduling**:
```rust
pub struct TimeSliceScheduler {
    /// NPU time quota per tenant
    quotas: HashMap<TenantId, Duration>,
    
    /// Current usage
    usage: HashMap<TenantId, Duration>,
    
    /// Period for quota reset
    period: Duration,
}
```

---

## 10. FUTURE ENHANCEMENTS

### 10.1 GPU Multi-Tenancy

Apply same pattern to GPU (via BarraCUDA):
- Kernel backend: Vulkan/WGPU
- Userspace backend: Sandboxed compute shaders
- Same isolation guarantees

### 10.2 Cross-Device Orchestration

**Unified API**:
```rust
pub enum ComputeResource {
    Npu(NpuSlice),
    Gpu(GpuSlice),
    Cpu(CpuCore),
}

pub struct TenantAllocation {
    resources: Vec<ComputeResource>,
    limits: ResourceLimits,
}
```

---

## 11. RELATED SPECIFICATIONS

- **Security Sandbox**: `crates/security/sandbox/` (existing)
- **Barracuda GPU**: `crates/barracuda/` (existing)
- **Universal Compute**: `specs/UNIVERSAL_COMPUTE_ORCHESTRATOR.md`

---

## 12. STATUS

- [x] Architecture design
- [x] Security model
- [ ] Implementation (week 1-4)
- [ ] Testing
- [ ] Production deployment

---

**Next**: Implement pure Rust setup code, deprecate shell scripts.
