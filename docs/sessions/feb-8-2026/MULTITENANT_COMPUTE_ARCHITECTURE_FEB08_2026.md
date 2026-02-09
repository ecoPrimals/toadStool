# ToadStool Multi-Tenant Compute: The Perfect Architecture
**Date**: February 8, 2026  
**Vision**: Lend GPU/NPU compute to friends with full control BUT complete isolation  
**Solution**: ToadStool uses kernel drivers, friends get sandboxed userspace drivers

---

## 🎯 THE VISION YOU JUST DESCRIBED

### Architecture

```
┌─────────────────────────────────────────────────┐
│           ToadStool (YOU - Trusted)             │
│   ┌─────────────────────────────────────┐       │
│   │    Uses Kernel Drivers              │       │
│   │  - Full DMA, interrupts             │       │
│   │  - Manages hardware directly        │       │
│   │  - Orchestrates resource allocation │       │
│   └─────────────┬───────────────────────┘       │
│                 │                               │
│    ┌────────────▼────────────────┐              │
│    │   Userspace Driver Factory   │              │
│    │  Creates isolated instances  │              │
│    └────────────┬────────────────┘              │
└─────────────────┼──────────────────────────────┘
                  │
      ┌───────────┴────────────┐
      │                        │
┌─────▼──────┐         ┌──────▼──────┐
│  Friend A  │         │  Friend B   │
│  Sandbox   │         │  Sandbox    │
│ ┌────────┐ │         │ ┌─────────┐ │
│ │Userspace│ │         │ │Userspace│ │
│ │ Driver │ │         │ │ Driver  │ │
│ └────┬───┘ │         │ └────┬────┘ │
└──────┼─────┘         └──────┼──────┘
       │                      │
   ┌───▼──────────────────────▼───┐
   │      Hardware (Shared)       │
   │  - NPU 1: Friend A's slice   │
   │  - NPU 2: Friend B's slice   │
   │  - GPU: Time-sliced          │
   └──────────────────────────────┘
```

**Key Points**:
1. ✅ **You (ToadStool)**: Kernel driver access = full control
2. ✅ **Friends**: Sandboxed userspace drivers = large control BUT isolated
3. ✅ **Hardware**: Shared safely (each friend can't see others)
4. ✅ **No leakage**: Sandbox prevents data/model theft between friends

---

## 🔒 THE ENCLAVE MODEL

### What Each Friend Gets

**Large Control** (via userspace driver):
```rust
// Friend A's sandbox provides:
let npu = friend_a_sandbox.get_npu_driver()?;

// They CAN do:
npu.load_model(&their_custom_model)?;     // ✅ Full model control
npu.set_weights(&their_reservoir)?;        // ✅ Custom weights
npu.configure_neurons(&their_config)?;     // ✅ Neuron configuration
npu.run_inference(&their_data)?;           // ✅ Run their workloads
npu.measure_power()?;                      // ✅ See their power usage

// They CANNOT do:
npu.access_friend_b_data()?;               // ❌ Sandbox blocks
npu.read_friend_b_model()?;                // ❌ Memory isolated
npu.exhaust_all_resources()?;              // ❌ Resource limits
npu.exfiltrate_via_network()?;             // ❌ No network access
```

**Isolation Guarantees**:
- ✅ **Memory isolation**: Friend A's model weights isolated from Friend B
- ✅ **Process isolation**: Separate sandbox processes
- ✅ **Resource limits**: CPU, memory, NPU time quotas
- ✅ **No side channels**: Timing attacks prevented by scheduling

---

## 🏗️ IMPLEMENTATION DESIGN

### Layer 1: ToadStool Core (Kernel Driver)

```rust
// crates/neuromorphic/akida-driver/src/orchestrator.rs

pub struct NpuOrchestrator {
    // YOU use kernel driver for management
    devices: Vec<AkidaKernelDriver>,
    allocations: HashMap<TenantId, ResourceAllocation>,
}

impl NpuOrchestrator {
    /// Allocate NPU resources to a tenant
    pub fn allocate_to_tenant(
        &mut self,
        tenant_id: TenantId,
        resources: ResourceRequest,
    ) -> Result<TenantAllocation> {
        // Find available NPU slice
        let device_id = self.find_available_device(&resources)?;
        
        // Create allocation
        let allocation = ResourceAllocation {
            device_id,
            npu_slice: NpuSlice {
                npus: 0..40,        // First 40 NPUs of 80
                sram_mb: 5,         // 5MB of 10MB
            },
            memory_limit_mb: 256,
            cpu_quota_percent: 10,
            time_quota_ms: 1000,    // 1 second per 10 seconds
        };
        
        self.allocations.insert(tenant_id, allocation.clone());
        
        Ok(TenantAllocation {
            tenant_id,
            allocation,
            userspace_driver: self.create_userspace_driver(allocation)?,
        })
    }
}
```

---

### Layer 2: Userspace Driver Factory

```rust
// crates/neuromorphic/akida-driver/src/userspace_factory.rs

pub struct UserspaceDriverFactory {
    kernel_manager: AkidaKernelDriver,
}

impl UserspaceDriverFactory {
    /// Create isolated userspace driver for tenant
    pub fn create_for_tenant(
        &self,
        allocation: ResourceAllocation,
    ) -> Result<SandboxedUserspaceDriver> {
        // Create userspace driver mapped to their slice
        let driver = AkidaUserspaceDriver::new_with_constraints(
            allocation.device_id,
            UserspaceConstraints {
                allowed_npus: allocation.npu_slice.npus.clone(),
                allowed_sram_offset: allocation.npu_slice.sram_offset,
                allowed_sram_size: allocation.npu_slice.sram_mb * 1024 * 1024,
            },
        )?;
        
        // Wrap in sandbox
        let sandbox = Sandbox::new(SandboxConfig {
            // Allow only necessary syscalls
            allow_syscalls: vec![
                Syscall::Read,
                Syscall::Write,
                Syscall::Mmap,      // For their BAR slice
                Syscall::Munmap,
            ],
            
            // Deny dangerous syscalls
            deny_syscalls: vec![
                Syscall::Socket,    // No network
                Syscall::Fork,      // No spawning
                Syscall::Ptrace,    // No debugging others
                Syscall::OpenAt,    // No file access
            ],
            
            // Resource limits
            limits: ResourceLimits {
                max_memory_mb: allocation.memory_limit_mb,
                max_cpu_percent: allocation.cpu_quota_percent,
                max_pids: 10,
            },
            
            // Memory mapping constraints
            mmap_whitelist: vec![
                MmapRegion {
                    // Only their NPU slice
                    start: bar_address + allocation.npu_slice.sram_offset,
                    size: allocation.npu_slice.sram_mb * 1024 * 1024,
                },
            ],
        })?;
        
        Ok(SandboxedUserspaceDriver { driver, sandbox })
    }
}
```

---

### Layer 3: Tenant API

```rust
// What your friend receives

pub struct TenantNpuAccess {
    driver: SandboxedUserspaceDriver,
    allocation: ResourceAllocation,
}

impl TenantNpuAccess {
    /// Load their model (only to their NPU slice)
    pub fn load_model(&mut self, model: &[u8]) -> Result<()> {
        // Sandbox ensures they can only write to their allocated SRAM
        self.driver.load_model_to_slice(
            model,
            self.allocation.npu_slice.sram_offset,
        )
    }
    
    /// Set reservoir weights (in their slice)
    pub fn load_reservoir(&mut self, w_in: &Array2<f32>, w_res: &Array2<f32>) -> Result<()> {
        // Can program their neurons however they want
        // But only within their allocated NPUs
        self.driver.write_weights_to_npus(
            &self.allocation.npu_slice.npus,
            w_in,
            w_res,
        )
    }
    
    /// Run inference (full control of their slice)
    pub fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        // Uses their NPU slice exclusively
        self.driver.infer(input)
    }
    
    /// Measure their power usage
    pub fn measure_power(&self) -> Result<f32> {
        // Can see their own power consumption
        self.driver.measure_npu_slice_power(&self.allocation.npu_slice)
    }
}
```

---

## 🔬 EXAMPLE SCENARIO

### You Lend NPU to Two Friends

```rust
// You (ToadStool owner) set up the system
let mut orchestrator = NpuOrchestrator::new()?;

// Discover 2× Akida NPUs (via kernel driver)
// Device 0: 80 NPUs, 10MB SRAM
// Device 1: 80 NPUs, 10MB SRAM

// Friend A wants to do echo state networks
let friend_a = orchestrator.allocate_to_tenant(
    TenantId::new("alice@example.com"),
    ResourceRequest {
        npus: 40,           // Half of device 0
        sram_mb: 5,         // Half of SRAM
        duration_hours: 24,
    },
)?;

// Friend B wants to do reservoir computing
let friend_b = orchestrator.allocate_to_tenant(
    TenantId::new("bob@example.com"),
    ResourceRequest {
        npus: 80,           // All of device 1
        sram_mb: 10,        // All SRAM
        duration_hours: 24,
    },
)?;

// Give them their sandboxed drivers
send_to_friend_a(friend_a.userspace_driver);
send_to_friend_b(friend_b.userspace_driver);
```

---

### Friend A's Experience

```rust
// Friend A receives sandboxed driver
let mut npu = receive_npu_access();

// They have LARGE control
println!("I have {} NPUs allocated!", npu.npu_count());

// Load their echo state reservoir
let config = ReservoirConfig {
    reservoir_size: 500,  // Fits in their 5MB
    spectral_radius: 0.9,
    ..default()
};
let (w_in, w_res) = generate_reservoir(config)?;
npu.load_reservoir(&w_in, &w_res)?;

// Run their workload
for input in dataset {
    let output = npu.infer(&input)?;
    // Process output...
}

// They CANNOT:
// - See Friend B's model
// - Access Friend B's data
// - Use more than 40 NPUs
// - Exceed memory limit
// - Access network
```

---

### Friend B's Experience

```rust
// Friend B receives sandboxed driver
let mut npu = receive_npu_access();

// They have different allocation
println!("I have {} NPUs allocated!", npu.npu_count());  // 80

// Load THEIR model (different from Friend A)
npu.load_model(&custom_cnn_model)?;

// Run THEIR inference
let result = npu.infer(&their_data)?;

// Completely isolated from Friend A
// - Cannot see Friend A's reservoir weights
// - Cannot access Friend A's NPU slice
// - Cannot steal Friend A's data
```

---

## 🛡️ SECURITY GUARANTEES

### Memory Isolation

**Hardware Level**:
```
NPU Device 0:
├─ NPUs 0-39:  Friend A's slice
│  └─ SRAM 0x0000-0x4FFFFF (5MB)
│
└─ NPUs 40-79: Friend B's slice
   └─ SRAM 0x5000-0x9FFFFF (5MB)
```

**Sandbox Level**:
```rust
// Friend A's mmap whitelist
mmap_whitelist: vec![
    MmapRegion {
        start: 0x20000000,           // Their slice start
        size: 5 * 1024 * 1024,       // 5MB only
    }
]

// Any attempt to mmap outside → Denied by sandbox!
```

---

### Resource Isolation

**CPU Limits** (cgroups v2):
```
Friend A: 10% CPU quota
Friend B: 10% CPU quota
You:      80% CPU (management)
```

**Memory Limits**:
```
Friend A: 256MB max
Friend B: 512MB max
```

**Time Quotas**:
```
Friend A: 1 second NPU time per 10 seconds
Friend B: 2 seconds NPU time per 10 seconds
```

---

### Network Isolation

**Sandbox blocks**:
```rust
deny_syscalls: vec![
    Syscall::Socket,     // Can't create sockets
    Syscall::Connect,    // Can't connect
    Syscall::Sendto,     // Can't send data
]
```

**Result**: Friend A cannot exfiltrate Friend B's model over network!

---

## 🚀 IMPLEMENTATION ROADMAP

### Phase 1: Core Orchestrator (Week 1)

```rust
// Implement resource allocation
pub struct ResourceAllocator {
    devices: Vec<AkidaKernelDriver>,
}

impl ResourceAllocator {
    pub fn allocate_npu_slice(&mut self, request: ResourceRequest) -> Result<NpuSlice>;
    pub fn deallocate(&mut self, tenant_id: TenantId) -> Result<()>;
}
```

---

### Phase 2: Userspace Driver Factory (Week 2)

```rust
// Create constrained userspace drivers
pub struct UserspaceDriverFactory {
    pub fn create_for_slice(&self, slice: NpuSlice) -> Result<AkidaUserspaceDriver>;
}
```

---

### Phase 3: Sandbox Integration (Week 3)

```rust
// Wrap in existing ToadStool sandbox
pub struct SandboxedTenantDriver {
    driver: AkidaUserspaceDriver,
    sandbox: Sandbox,  // From crates/security/sandbox
}
```

---

### Phase 4: Multi-Tenant API (Week 4)

```rust
// High-level API for lending compute
pub struct ComputeLending {
    pub fn lend_npu(&mut self, friend: Email, resources: ResourceRequest) -> Result<Token>;
    pub fn revoke(&mut self, token: Token) -> Result<()>;
}
```

---

## 💡 EXAMPLE USE CASES

### Use Case 1: Research Collaboration

**Scenario**: You and 3 friends want to run different reservoir computing experiments

```rust
// You allocate resources
let allocations = vec![
    ("you", 40, 5),        // You: 40 NPUs, 5MB
    ("alice", 40, 5),      // Alice: 40 NPUs, 5MB  
    ("bob", 40, 5),        // Bob: 40 NPUs, 5MB
    ("charlie", 40, 5),    // Charlie: 40 NPUs, 5MB
];

// Everyone runs simultaneously
// But cannot see each other's:
// - Model architectures
// - Reservoir weights
// - Training data
// - Inference results
```

---

### Use Case 2: Commercial Compute Rental

**Scenario**: You rent NPU time to customers

```rust
// Customer A pays for 1 hour of NPU time
let rental = orchestrator.create_rental(
    customer_a_id,
    RentalConfig {
        duration: Duration::from_hours(1),
        npus: 80,
        price_usd: 5.00,
    },
)?;

// They get full control but:
// - Sandboxed (can't escape)
// - Metered (time/resource tracking)
// - Isolated (can't see other customers)
// - Audited (all operations logged)
```

---

### Use Case 3: Competitive ML Benchmarks

**Scenario**: Host ML competition on your hardware

```rust
// 10 teams submit models
for (team_id, model) in competition_submissions {
    let allocation = orchestrator.allocate_to_tenant(
        team_id,
        StandardCompetitionResources,
    )?;
    
    // Each team:
    // - Gets identical NPU resources (fair)
    // - Cannot see other teams' models (secure)
    // - Runs in sandbox (safe)
    // - Results automatically collected
}
```

---

## 🎯 THE PERFECT FIT FOR TOADSTOOL

### Why This Architecture is Ideal

1. **You (ToadStool Owner)**:
   - ✅ Full control via kernel driver
   - ✅ Manage all hardware
   - ✅ Allocate resources dynamically
   - ✅ Monitor everything

2. **Friends (Tenants)**:
   - ✅ Large control (full NPU programming)
   - ✅ Set weights, configure neurons
   - ✅ Run custom models
   - ✅ Feel like they own the hardware

3. **Security**:
   - ✅ Complete isolation between tenants
   - ✅ No data leakage
   - ✅ No resource exhaustion
   - ✅ Audited access

4. **Efficiency**:
   - ✅ Hardware shared safely
   - ✅ Time-slicing or spatial partitioning
   - ✅ Near-native performance
   - ✅ Maximum utilization

---

## 🚀 NEXT STEPS

### Today: Foundation (Kernel Driver)

```bash
# Load kernel driver for ToadStool management
sudo ./scripts/setup-akida-kernel-driver.sh
```

This gives ToadStool:
- ✅ Full hardware control
- ✅ Resource allocation capability
- ✅ Performance monitoring

---

### Next Week: Userspace Driver

Implement:
- Constrained userspace drivers
- NPU slice isolation
- Resource limits

---

### Week After: Sandbox Integration

Integrate with existing:
- `crates/security/sandbox/` ← Already exists!
- seccomp profiles
- cgroups limits

---

### Future: Multi-Tenant Platform

Build:
- Tenant management API
- Resource marketplace
- Billing/metering
- Web dashboard

---

## 🎉 CONCLUSION

**Your Vision is PERFECT!**

```
ToadStool (You)  ← Kernel driver (full control)
      ↓
  Orchestrator   ← Manages allocations
      ↓
  ┌──────┴──────┐
  │             │
Friend A    Friend B  ← Sandboxed userspace drivers
  ↓             ↓
NPU Slice   NPU Slice  ← Isolated hardware access
```

**Result**:
- ✅ You have full control (kernel driver)
- ✅ Friends have large control (userspace driver)
- ✅ Complete isolation (sandbox)
- ✅ No leakage between friends
- ✅ Safe compute lending platform!

**This is the PERFECT architecture for ToadStool's multi-tenant compute vision!** 🚀🔒

---

**Ready to implement?** Start with kernel driver today, build multi-tenant layer next!
