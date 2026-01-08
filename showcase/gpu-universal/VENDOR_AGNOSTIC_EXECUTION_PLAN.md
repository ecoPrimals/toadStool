# Vendor-Agnostic GPU Execution - Complete Plan

**Date**: January 8, 2026  
**Status**: 🚀 READY TO EXECUTE  
**Goal**: Same code on NVIDIA + AMD via OpenCL and Vulkan

---

## 🎯 Mission

**Prove**: "The metal you own, not the capabilities you have"

**Execute**:
1. ✅ OpenCL detection working at system level (clinfo)
2. → Create Rust abstraction for OpenCL (bypasses Python issues)
3. → Verify Vulkan on NVIDIA (not just AMD)
4. → Unified backend that uses best available
5. → Same compute workload on both GPUs

---

## ✅ Current Status

### System-Level OpenCL (Working!)

```bash
$ clinfo -l
Platform #0: Clover
Platform #1: AMD Accelerated Parallel Processing
 `-- Device #0: gfx1030 (AMD RX 6950 XT) ✅
Platform #2: NVIDIA CUDA
 `-- Device #0: NVIDIA GeForce RTX 3090 ✅
```

**Result**: BOTH GPUs detected via OpenCL at system level! ✅

### ROCm Detection (Working!)

```bash
$ rocminfo
Agent 3: gfx1030
Marketing Name: AMD Radeon RX 6950 XT
Compute Unit: 80
Feature: KERNEL_DISPATCH ✅
```

**Result**: AMD GPU ready for compute! ✅

### Python Binding (Issue)

```python
pyopencl.get_platforms()
# Only shows NVIDIA ❌
```

**Issue**: Python binding not seeing AMD platform  
**Solution**: Use Rust directly (our crates already do this!)

---

## 🏗️ Architecture

### Current Working Paths

**Rust Code** (from earlier tests):
```
Neural Network Inference
         ↓
ToadStool GPU Runtime
    ┌────┴────┐
    ↓         ↓
OpenCL    Vulkan
    ↓         ↓
NVIDIA    AMD
   ✅        ✅
```

**Python Code** (for LLMs):
```
Mistral 7B
    ↓
PyTorch
    ↓
 CUDA
    ↓
NVIDIA
   ✅
```

### Target Architecture

**Unified Rust Backend**:
```rust
Compute Workload
       ↓
Backend Selector
    ┌──┴──┐
    ↓     ↓
OpenCL Vulkan  wgpu
  ↓       ↓     ↓
All    All    All
GPUs   GPUs   GPUs
```

---

## 🚀 Execution Plan

### Phase 1: Verify OpenCL in Rust (1 hour)

**Create**: `showcase/gpu-universal/opencl-test/`

**Test**:
```rust
use ocl::{Platform, Device};

fn main() -> Result<()> {
    // Enumerate all platforms
    let platforms = Platform::list();
    
    for platform in platforms {
        println!("Platform: {}", platform.name()?);
        
        let devices = Device::list_all(platform)?;
        for device in devices {
            println!("  Device: {}", device.name()?);
            println!("    Memory: {} GB", device.mem_size()? / 1e9);
        }
    }
}
```

**Expected**:
- Platform: NVIDIA CUDA → NVIDIA RTX 3090 ✅
- Platform: AMD → AMD RX 6950 XT ✅

**Value**: Proves Rust sees both GPUs via OpenCL

### Phase 2: Verify Vulkan on NVIDIA (1 hour)

**Create**: `showcase/gpu-universal/vulkan-test/`

**Test**:
```rust
use ash::{vk, Entry};

fn main() -> Result<()> {
    let entry = Entry::linked();
    let instance = create_instance(&entry)?;
    
    // Enumerate physical devices
    let devices = unsafe {
        instance.enumerate_physical_devices()?
    };
    
    for device in devices {
        let props = unsafe {
            instance.get_physical_device_properties(device)
        };
        
        let name = unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
        };
        
        println!("Device: {:?}", name);
        println!("  Type: {:?}", props.device_type);
        println!("  Vendor: {:#x}", props.vendor_id);
    }
}
```

**Expected**:
- AMD RX 6950 XT ✅ (already working)
- NVIDIA RTX 3090 ✅ (to verify)

**Value**: Proves Vulkan works on BOTH vendors

### Phase 3: Unified Backend Abstraction (2-3 hours)

**Create**: `crates/runtime/gpu/src/unified_backend.rs`

**Interface**:
```rust
pub enum ComputeBackend {
    OpenCL(ocl::Device),
    Vulkan(VulkanDevice),
    Wgpu(wgpu::Device),
}

pub struct UnifiedGpuRuntime {
    backends: Vec<ComputeBackend>,
}

impl UnifiedGpuRuntime {
    pub fn discover_all() -> Result<Self> {
        let mut backends = Vec::new();
        
        // Try OpenCL (works on NVIDIA + AMD)
        if let Ok(opencl_devices) = discover_opencl() {
            backends.extend(opencl_devices.into_iter()
                .map(ComputeBackend::OpenCL));
        }
        
        // Try Vulkan (works on NVIDIA + AMD)
        if let Ok(vulkan_devices) = discover_vulkan() {
            backends.extend(vulkan_devices.into_iter()
                .map(ComputeBackend::Vulkan));
        }
        
        // Try wgpu (pure Rust, works on all)
        if let Ok(wgpu_device) = discover_wgpu().await {
            backends.push(ComputeBackend::Wgpu(wgpu_device));
        }
        
        Ok(Self { backends })
    }
    
    pub fn execute_compute(
        &self,
        kernel: &ComputeKernel,
        input: &[f32],
    ) -> Result<Vec<f32>> {
        // Pick best backend for this workload
        let backend = self.select_backend(kernel)?;
        
        match backend {
            ComputeBackend::OpenCL(device) => {
                execute_opencl(device, kernel, input)
            },
            ComputeBackend::Vulkan(device) => {
                execute_vulkan(device, kernel, input)
            },
            ComputeBackend::Wgpu(device) => {
                execute_wgpu(device, kernel, input).await
            },
        }
    }
}
```

**Value**: Single API, multiple backends, vendor-agnostic

### Phase 4: Cross-Vendor Test (1 hour)

**Create**: `showcase/gpu-universal/src/bin/vendor_agnostic_demo.rs`

**Demo**:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Discovering GPUs...");
    let runtime = UnifiedGpuRuntime::discover_all()?;
    
    println!("✓ Found {} compute backends", runtime.backends.len());
    
    // Test workload: Vector addition
    let a = vec![1.0f32; 1000];
    let b = vec![2.0f32; 1000];
    
    println!("\n📊 Running same workload on all backends:");
    
    for (i, backend) in runtime.backends.iter().enumerate() {
        println!("\n  Backend {}: {:?}", i, backend);
        
        let start = Instant::now();
        let result = backend.vector_add(&a, &b)?;
        let elapsed = start.elapsed();
        
        // Verify correctness
        assert_eq!(result[0], 3.0);
        
        println!("    Time: {:?}", elapsed);
        println!("    Correct: ✅");
    }
    
    println!("\n╔══════════════════════════════════════╗");
    println!("║  VENDOR-AGNOSTIC COMPUTE WORKING ✅  ║");
    println!("╚══════════════════════════════════════╝");
}
```

**Expected Output**:
```
🔍 Discovering GPUs...
✓ Found 4 compute backends

📊 Running same workload on all backends:

  Backend 0: OpenCL(NVIDIA RTX 3090)
    Time: 1.2ms
    Correct: ✅

  Backend 1: OpenCL(AMD RX 6950 XT)
    Time: 1.5ms
    Correct: ✅

  Backend 2: Vulkan(NVIDIA RTX 3090)
    Time: 1.3ms
    Correct: ✅

  Backend 3: Vulkan(AMD RX 6950 XT)
    Time: 1.4ms
    Correct: ✅

╔══════════════════════════════════════╗
║  VENDOR-AGNOSTIC COMPUTE WORKING ✅  ║
╚══════════════════════════════════════╝
```

**Value**: Proves complete vendor agnosticism!

---

## 💡 Key Insights

### Why This Matters

**Problem**: Python ML ecosystem has vendor lock-in
- PyTorch: CUDA-centric
- PyOpenCL: Binding issues
- Fragmentation across vendors

**ToadStool Solution**: Abstract at Rust level
- Direct API access (no Python bindings)
- Multiple backends (OpenCL, Vulkan, wgpu)
- Runtime selection (best for workload)
- Vendor-agnostic (same code, any GPU)

### Evolution Gaps Solved

**Gap 1**: Python binding issues  
**Solution**: Use Rust directly (ocl, ash, wgpu crates)

**Gap 2**: Vendor-specific code paths  
**Solution**: Unified backend abstraction

**Gap 3**: Fragmented ML ecosystem  
**Solution**: Build pure Rust ML stack

---

## 📊 Success Criteria

### Must Have ✅

1. **OpenCL Detection**
   - [ ] Rust code detects NVIDIA via OpenCL
   - [ ] Rust code detects AMD via OpenCL
   - [ ] Same OpenCL code runs on both

2. **Vulkan Detection**
   - [ ] Rust code detects AMD via Vulkan
   - [ ] Rust code detects NVIDIA via Vulkan
   - [ ] Same Vulkan code runs on both

3. **Unified Interface**
   - [ ] Single API abstracts backend differences
   - [ ] Runtime backend selection working
   - [ ] Vendor-agnostic compute verified

### Nice to Have

4. **Performance Comparison**
   - [ ] OpenCL vs Vulkan on same GPU
   - [ ] NVIDIA vs AMD on same backend
   - [ ] Document trade-offs

5. **wgpu Integration**
   - [ ] Pure Rust path working
   - [ ] Cross-platform (Vulkan/Metal/DX12)
   - [ ] WebGPU standard compliance

---

## 🔧 Implementation Details

### Rust Crates Needed

```toml
[dependencies]
# OpenCL (works on NVIDIA + AMD)
ocl = "0.19"

# Vulkan (works on NVIDIA + AMD)
ash = "0.37"

# Pure Rust GPU (works everywhere)
wgpu = "0.19"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
anyhow = "1"
```

### Directory Structure

```
showcase/gpu-universal/
├── opencl-test/          # Test OpenCL on both GPUs
│   ├── Cargo.toml
│   └── src/main.rs
├── vulkan-test/          # Test Vulkan on both GPUs
│   ├── Cargo.toml
│   └── src/main.rs
└── vendor-agnostic-demo/ # Unified demo
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── unified_backend.rs
```

---

## 🎯 Timeline

**Total**: 5-6 hours for complete vendor-agnostic solution

**Phase 1** (1 hour): OpenCL Rust test  
**Phase 2** (1 hour): Vulkan Rust test (NVIDIA)  
**Phase 3** (2-3 hours): Unified backend  
**Phase 4** (1 hour): Cross-vendor demo

---

## 💎 Expected Outcomes

### Technical Proof

**Capability Matrix**:

| GPU | OpenCL | Vulkan | wgpu | Vendor Lock-in |
|-----|--------|--------|------|----------------|
| **NVIDIA RTX 3090** | ✅ | ✅ | ✅ | ❌ None |
| **AMD RX 6950 XT** | ✅ | ✅ | ✅ | ❌ None |

**Result**: Complete vendor freedom! ✅

### Value Delivered

**For Users**:
- Buy any GPU (NVIDIA, AMD, Intel)
- Same code works on all
- Runtime optimization (pick best backend)
- No vendor lock-in

**For ToadStool**:
- Proves core value proposition
- Demonstrates abstraction power
- Shows evolution from gaps found
- Production-ready architecture

---

## 🚀 Next Actions

### Immediate (Now)

**Start with Rust OpenCL test**:
```bash
cd showcase/gpu-universal
cargo new opencl-test
cd opencl-test
# Add ocl dependency
# Write detection code
cargo run
```

**Expected**:
- 15 minutes to create
- Should immediately see both GPUs
- Proves OpenCL works in Rust

### Short-Term (Today)

**Complete all 4 phases**:
1. OpenCL test (1 hour)
2. Vulkan test (1 hour)
3. Unified backend (2-3 hours)
4. Cross-vendor demo (1 hour)

**Result**: Full vendor-agnostic compute in 5-6 hours

### Medium-Term (This Week)

**Integrate with LLM**:
- Use Rust ML libraries (candle, burn)
- Load models without Python
- Run on any GPU via unified backend
- Prove complete stack is vendor-agnostic

---

## 📝 Documentation Plan

### Files to Create

**Code**:
- `opencl-test/src/main.rs` - OpenCL detection
- `vulkan-test/src/main.rs` - Vulkan detection
- `vendor-agnostic-demo/` - Unified demo
- `unified_backend.rs` - Core abstraction

**Docs**:
- `OPENCL_DETECTION.md` - OpenCL results
- `VULKAN_CROSS_VENDOR.md` - Vulkan on both
- `UNIFIED_BACKEND.md` - Architecture
- `VENDOR_AGNOSTIC_COMPLETE.md` - Final report

---

## 💡 Why This Approach Works

### Bypasses Python Issues

**Python**: Binding-dependent, fragile  
**Rust**: Direct API access, solid

### Proven Technology

**OpenCL**: System-level detection works ✅  
**Vulkan**: AMD already working ✅  
**wgpu**: Pure Rust, verified ✅

### Scalable Architecture

**Today**: OpenCL + Vulkan  
**Tomorrow**: Add Metal (Apple)  
**Future**: Add custom backends

### Aligns with Vision

**"The metal you own, not the capabilities you have"**

This architecture delivers that promise! ✅

---

**Document Version**: 1.0  
**Last Updated**: January 8, 2026  
**Status**: Ready to Execute  
**Next**: Create opencl-test

---

*ToadStool: Building True Vendor Agnosticism* 🚀

**"Same code. Any GPU. No compromises."**

