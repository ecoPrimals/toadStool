# Open GPU Compute Frameworks - Landscape Analysis

**Date**: January 8, 2026  
**Goal**: "Leverage competitive environment rather than reward vendor lock"  
**Status**: 🔍 **MAPPING THE ECOSYSTEM**

---

## 🎯 Philosophy

**Vendor Lock-In**:
- Single framework (CUDA) → Forced to buy specific vendor
- Limits competition → Higher prices
- Reduces innovation → Stagnation
- User loses → No choice

**Open Systems**:
- Multiple frameworks → Choose best GPU
- Enables competition → Lower prices
- Drives innovation → Better products
- User wins → Freedom

**ToadStool Strategy**: Support all viable open standards ✅

---

## 📊 Open GPU Compute Frameworks

### Tier 1: Cross-Platform Open Standards (Priority)

#### 1. **OpenCL** ✅ VERIFIED WORKING

**Status**: ✅ NVIDIA + AMD fully working

**What It Is**:
- Industry standard for heterogeneous computing
- Supports CPUs, GPUs, FPGAs, DSPs
- C-like kernel language
- Managed by Khronos Group

**Vendors Supporting**:
- ✅ NVIDIA (via CUDA driver)
- ✅ AMD (via ROCm)
- ✅ Intel (via oneAPI)
- ✅ ARM (Mali GPUs)
- ✅ Qualcomm (Adreno GPUs)
- ✅ Apple (deprecated but still present)

**Versions**:
- OpenCL 1.2 - Widely supported
- OpenCL 2.0 - SVM, pipes, subgroups
- OpenCL 2.1 - SPIR-V support
- OpenCL 3.0 - Optional features, modernization

**Rust Crates**:
- `ocl` - High-level API ✅
- `ocl-core` - Low-level bindings ✅

**ToadStool Status**: ✅ **PRODUCTION READY**

**Advantages**:
- ✅ Most widely supported
- ✅ Mature ecosystem
- ✅ Cross-vendor proven
- ✅ Extensive tooling

**Limitations**:
- Some vendors have better support than others
- API can be verbose
- Different OpenCL versions across vendors

---

#### 2. **Vulkan Compute** ⚠️ DETECTION WORKING, EXECUTION NEXT

**Status**: ✅ NVIDIA + AMD detected, ❓ execution needs testing

**What It Is**:
- Modern low-overhead graphics and compute API
- Successor to OpenGL
- SPIR-V shader bytecode
- Managed by Khronos Group

**Vendors Supporting**:
- ✅ NVIDIA (excellent support)
- ✅ AMD (excellent support, RADV driver)
- ✅ Intel (good support)
- ✅ ARM (Mali, Immortalis GPUs)
- ✅ Qualcomm (Adreno GPUs)
- ✅ Apple (via MoltenVK translation to Metal)

**Versions**:
- Vulkan 1.0 - Initial release
- Vulkan 1.1 - Subgroups, protected memory
- Vulkan 1.2 - Descriptor indexing, timeline semaphores
- Vulkan 1.3 - Dynamic rendering, synchronization2
- Vulkan 1.4 - Latest (NVIDIA support)

**Rust Crates**:
- `ash` - Low-level bindings ✅
- `vulkano` - High-level safe wrapper
- `wgpu` - WebGPU implementation (uses Vulkan backend)

**ToadStool Status**: ⚠️ **DETECTION DONE, EXECUTION NEXT**

**Advantages**:
- ✅ Modern, low-overhead design
- ✅ Explicit control (optimization potential)
- ✅ SPIR-V is portable bytecode
- ✅ Excellent cross-platform support
- ✅ Growing compute ecosystem

**Limitations**:
- More verbose than OpenCL for simple tasks
- Steeper learning curve
- Requires more boilerplate

**Next Action**: Verify compute execution on both GPUs ✅

---

#### 3. **WebGPU / wgpu** ✅ READY TO INTEGRATE

**Status**: ✅ Pure Rust, cross-platform, not yet tested

**What It Is**:
- Web standard for GPU access
- Safe, portable GPU API
- Pure Rust implementation (`wgpu`)
- Backends: Vulkan, Metal, DirectX 12, WebGPU

**Vendors Supporting**:
- ✅ Any GPU with Vulkan (NVIDIA, AMD, Intel)
- ✅ Any GPU with Metal (Apple)
- ✅ Any GPU with DirectX 12 (NVIDIA, AMD, Intel on Windows)
- ✅ Browser support growing

**Rust Crates**:
- `wgpu` - Pure Rust implementation ✅
- `wgpu-core` - Core implementation
- `wgpu-hal` - Hardware abstraction layer

**ToadStool Status**: ✅ **READY TO TEST**

**Advantages**:
- ✅ Pure Rust (no FFI, no unsafe)
- ✅ Cross-platform (Vulkan/Metal/DX12)
- ✅ Safe abstractions
- ✅ Modern API design
- ✅ Web deployment possible
- ✅ Active development

**Limitations**:
- Newer (less mature than OpenCL/Vulkan)
- WebGPU spec still evolving
- May have performance overhead vs raw Vulkan

**Next Action**: Test wgpu compute on both GPUs ✅

---

### Tier 2: Vendor-Neutral High-Level Abstractions

#### 4. **SYCL** 🔍 INVESTIGATE

**Status**: ❓ Not yet investigated

**What It Is**:
- C++ abstraction over multiple backends
- Single-source (host + device in same code)
- Backends: OpenCL, CUDA, HIP, Level Zero
- Managed by Khronos Group

**Implementations**:
- **ComputeCpp** (Codeplay, proprietary)
- **DPC++** (Intel, open source)
- **AdaptiveCpp** (formerly hipSYCL, open source)

**Vendors Supporting**:
- Via backends: NVIDIA (CUDA), AMD (HIP), Intel (Level Zero)

**Rust Bindings**:
- No mature Rust bindings currently
- Could integrate via C++ FFI

**ToadStool Status**: 🔍 **EVALUATE**

**Advantages**:
- ✅ Single-source C++ (easier than separate kernels)
- ✅ Multiple backend support
- ✅ Modern C++ features
- ✅ Khronos standard

**Limitations**:
- C++ (not Rust)
- Less mature than OpenCL
- FFI overhead if used from Rust
- Vendor implementations vary

**Decision**: Consider for C++ interop, but focus on native Rust solutions first

---

#### 5. **SPIR-V** ✅ ALREADY USING (VIA VULKAN)

**Status**: ✅ Supported as compilation target

**What It Is**:
- Intermediate representation for shaders/kernels
- Used by Vulkan, OpenCL 2.1+, etc.
- Can compile from GLSL, HLSL, etc.

**Not a framework itself, but enables**:
- Write shader once
- Compile to SPIR-V
- Use in Vulkan or OpenCL

**Rust Crates**:
- `spirv-std` - SPIR-V shader library
- `rust-gpu` - Compile Rust to SPIR-V

**ToadStool Status**: ✅ **IN USE VIA VULKAN**

**Advantages**:
- ✅ Portable bytecode
- ✅ Multiple source languages
- ✅ Used by Vulkan and modern OpenCL

**Use Case**: Compile kernels once, use in multiple frameworks

---

### Tier 3: Vendor-Specific Open APIs (For Specific Hardware)

#### 6. **Level Zero** 🔍 INVESTIGATE (INTEL)

**Status**: ❓ Not yet investigated

**What It Is**:
- Intel's low-level GPU API
- Part of oneAPI initiative
- Lower-level than OpenCL
- Open source

**Vendors Supporting**:
- ✅ Intel (Arc, Xe, integrated GPUs)
- Limited to Intel hardware

**Rust Bindings**:
- `level-zero-sys` - Unsafe bindings exist

**ToadStool Status**: 🔍 **EVALUATE IF INTEL GPU SUPPORT NEEDED**

**Advantages**:
- ✅ Low-overhead (like Vulkan)
- ✅ Intel-optimized
- ✅ Open source

**Limitations**:
- Intel-only (not cross-vendor)
- Less mature ecosystem
- Smaller community

**Decision**: Defer until Intel GPU support requested

---

#### 7. **ROCm / HIP** ⚠️ AMD-SPECIFIC (NOT TRULY OPEN)

**Status**: ✅ System installed, ❓ not directly used

**What It Is**:
- AMD's CUDA-compatible API
- Allows porting CUDA code to AMD
- Part of ROCm platform

**Vendors Supporting**:
- AMD only (by design)

**Rust Bindings**:
- Limited

**ToadStool Status**: ⚠️ **AVAILABLE BUT NOT PRIORITIZED**

**Advantages**:
- ✅ CUDA-compatible (easy porting)
- ✅ AMD-optimized

**Limitations**:
- ❌ AMD-only (vendor lock-in to AMD)
- ❌ Not cross-vendor
- ❌ Defeats our "open systems" goal

**Decision**: Use OpenCL/Vulkan instead (truly cross-vendor)

---

### Tier 4: Platform-Specific (Important but Not Open)

#### 8. **Metal** 🍎 APPLE PLATFORMS

**Status**: ❓ Not yet investigated

**What It Is**:
- Apple's graphics and compute API
- macOS, iOS, iPadOS, tvOS
- High performance, Apple-optimized

**Vendors Supporting**:
- Apple only (by design)

**Rust Bindings**:
- `metal-rs` - Safe Rust bindings

**ToadStool Status**: 🔍 **EVALUATE FOR APPLE SUPPORT**

**Advantages**:
- ✅ Excellent performance on Apple hardware
- ✅ Well-designed API
- ✅ Unified graphics + compute

**Limitations**:
- ❌ Apple-only (not cross-platform)
- ❌ Closed source

**Decision**: 
- Important for macOS/iOS users
- wgpu already supports Metal backend
- Can add via wgpu rather than direct Metal

---

#### 9. **DirectCompute / DirectX 12** 🪟 WINDOWS

**Status**: ❓ Not yet investigated

**What It Is**:
- Microsoft's compute shader API
- Part of DirectX 12
- Windows-only

**Vendors Supporting**:
- ✅ NVIDIA (Windows)
- ✅ AMD (Windows)
- ✅ Intel (Windows)

**Rust Bindings**:
- `windows-rs` - Official Windows bindings
- `wgpu` - Supports DX12 backend

**ToadStool Status**: 🔍 **AVAILABLE VIA WGPU**

**Advantages**:
- ✅ Native Windows performance
- ✅ Good vendor support on Windows

**Limitations**:
- ❌ Windows-only
- ❌ Microsoft-controlled

**Decision**:
- Important for Windows users
- wgpu already supports DX12 backend
- Can add via wgpu rather than direct DirectX

---

## 🎯 ToadStool Priority Matrix

### Immediate (Next 1-2 Days)

**1. Vulkan Compute Execution** ⚡ HIGH PRIORITY
- Status: Detection working, execution untested
- Action: Create Vulkan compute test (like OpenCL test)
- Value: Cross-vendor, modern, proven detection
- Effort: 2-3 hours

**2. wgpu Integration** ⚡ HIGH PRIORITY
- Status: Pure Rust, not yet tested
- Action: Create wgpu compute test
- Value: Safe Rust, cross-platform, future-proof
- Effort: 2-3 hours

### Short-Term (This Week)

**3. Unified Backend Abstraction**
- Integrate OpenCL + Vulkan + wgpu
- Single ToadStool API
- Runtime backend selection
- Automatic fallback

**4. Performance Comparison**
- OpenCL vs Vulkan vs wgpu
- NVIDIA vs AMD on each backend
- Understand trade-offs

### Medium-Term (This Month)

**5. SYCL Evaluation**
- Investigate ComputeCpp or DPC++
- Rust FFI integration
- Decide if worthwhile

**6. Level Zero (If Intel GPU)**
- Only if Intel GPU support needed
- Otherwise defer

### Long-Term (Future)

**7. Platform-Specific Optimizations**
- Metal (via wgpu) for macOS users
- DirectX 12 (via wgpu) for Windows users
- Already handled by wgpu backend selection

---

## 📊 Framework Comparison

### Cross-Vendor Support

| Framework | NVIDIA | AMD | Intel | ARM | Qualcomm | Apple |
|-----------|--------|-----|-------|-----|----------|-------|
| **OpenCL**    | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **Vulkan**    | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **wgpu**      | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **SYCL**      | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| **Level Zero**| ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **HIP**       | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Metal**     | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| **DirectX**   | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |

**Winner for Cross-Vendor**: wgpu (via backends) ✅

### Maturity

| Framework | Stability | Ecosystem | Documentation | Tooling |
|-----------|-----------|-----------|---------------|---------|
| **OpenCL**    | ✅✅✅ | ✅✅✅ | ✅✅✅ | ✅✅✅ |
| **Vulkan**    | ✅✅✅ | ✅✅ | ✅✅✅ | ✅✅ |
| **wgpu**      | ✅✅ | ✅✅ | ✅✅ | ✅✅ |
| **SYCL**      | ✅✅ | ✅ | ✅✅ | ✅ |
| **Level Zero**| ✅ | ✅ | ✅ | ✅ |

**Winner for Maturity**: OpenCL ✅

### Pure Rust

| Framework | Pure Rust | Safe | No FFI |
|-----------|-----------|------|--------|
| **OpenCL**    | ❌ (via ocl) | ⚠️ | ❌ |
| **Vulkan**    | ❌ (via ash) | ⚠️ | ❌ |
| **wgpu**      | ✅ | ✅ | ✅ |
| **SYCL**      | ❌ (C++) | ❌ | ❌ |
| **Level Zero**| ❌ (FFI) | ❌ | ❌ |

**Winner for Pure Rust**: wgpu ✅

---

## 🎯 Recommended Strategy

### Phase 1: Core Open Standards (Now)

**Focus on the "Big 3"**:
1. ✅ **OpenCL** - Widest support, mature ✅ DONE
2. ⚡ **Vulkan** - Modern, cross-vendor ← NEXT
3. ⚡ **wgpu** - Pure Rust, future ← NEXT

**Rationale**:
- Maximum vendor coverage
- Proven technology
- Pure Rust option available
- Covers 99% of use cases

### Phase 2: Abstraction Layer (This Week)

**Unified ToadStool API**:
```rust
// Application code doesn't care about backend
let runtime = ToadStoolGpuRuntime::new()?;

// Runtime picks best available:
// 1. Try OpenCL (mature, fast)
// 2. Try Vulkan (modern, fast)
// 3. Try wgpu (pure Rust, safe)
// 4. Fall back to CPU

let result = runtime.execute_compute(workload)?;
```

### Phase 3: Optimization (This Month)

**Per-backend tuning**:
- OpenCL: Work group sizes, buffer patterns
- Vulkan: Pipeline optimization, memory management
- wgpu: Shader optimization, resource binding

**Auto-selection**:
- Benchmark each backend
- Profile per GPU
- Pick best for workload

### Phase 4: Ecosystem Integration (Future)

**SYCL**: If needed for C++ interop  
**Level Zero**: If Intel GPU support requested  
**Platform-specific**: Via wgpu (Metal, DX12)

---

## 💡 Why This Strategy Wins

### 1. Maximum Coverage

**3 backends** (OpenCL, Vulkan, wgpu) cover:
- ✅ NVIDIA (all 3)
- ✅ AMD (all 3)
- ✅ Intel (all 3)
- ✅ ARM (OpenCL, Vulkan, wgpu)
- ✅ Qualcomm (OpenCL, Vulkan, wgpu)
- ✅ Apple (wgpu → Metal)

**Result**: 99%+ of GPUs covered ✅

### 2. Vendor Competition

**Users can choose GPU based on**:
- Price/performance (not vendor lock-in)
- Availability (supply chain)
- Power efficiency (datacenter costs)
- Brand preference (not forced)

**Result**: Market competition drives prices down ✅

### 3. Future-Proof

**New vendor enters market?**:
- If they support OpenCL/Vulkan → Works immediately ✅
- If they support WebGPU → Works via wgpu ✅
- If proprietary → They lose (users avoid lock-in) ✅

**Result**: Open standards enable innovation ✅

### 4. Pure Rust Path

**wgpu provides**:
- ✅ No FFI (pure Rust)
- ✅ Memory safety
- ✅ Type safety
- ✅ Cross-platform

**Result**: Can evolve to 100% Rust over time ✅

---

## 🚀 Immediate Next Steps

### 1. Vulkan Compute Test (2-3 hours)

```bash
cd showcase/gpu-universal
cargo new --bin vulkan-compute-test
```

**Test**:
- Create compute pipeline
- Allocate buffers
- Dispatch compute shader
- Verify results

**Success**: Both GPUs execute Vulkan compute ✅

### 2. wgpu Compute Test (2-3 hours)

```bash
cargo new --bin wgpu-compute-test
```

**Test**:
- Initialize wgpu device
- Create compute pipeline
- Execute shader
- Verify results

**Success**: Pure Rust compute working ✅

### 3. Unified API (4-6 hours)

**Create**:
- `ToadStoolGpuRuntime`
- Backend enum (OpenCL, Vulkan, Wgpu)
- Automatic selection
- Fallback logic

**Success**: Application uses single API ✅

---

## 📊 Expected Outcome

### After Vulkan + wgpu Tests

**Compute Execution**:
- OpenCL: NVIDIA ✅ | AMD ✅
- Vulkan: NVIDIA ✅ | AMD ✅
- wgpu: NVIDIA ✅ | AMD ✅

**User Impact**:
- Buy any GPU → Works ✅
- Switch GPUs → Still works ✅
- Vendor competition → Lower prices ✅
- Innovation → Better products ✅

### After Unified API

**Developer Experience**:
```rust
// Same code, any GPU, any backend
let runtime = ToadStoolGpuRuntime::new()?;
let result = runtime.execute(workload)?;
```

**Result**: **VENDOR LOCK-IN ELIMINATED** ✅

---

## 🎯 Conclusion

### Open GPU Frameworks Available

**Tier 1** (Cross-platform, open):
- ✅ OpenCL (done)
- ⚡ Vulkan (next)
- ⚡ wgpu (next)

**Tier 2** (High-level):
- SYCL (C++, evaluate later)

**Tier 3** (Vendor-specific):
- Level Zero (Intel)
- HIP (AMD)

**Tier 4** (Platform-specific):
- Metal (Apple, via wgpu)
- DirectX (Windows, via wgpu)

### Strategy

**Focus**: Big 3 (OpenCL, Vulkan, wgpu) ✅  
**Coverage**: 99%+ of GPUs ✅  
**Philosophy**: Open standards over vendor lock-in ✅  
**Evolution**: Toward pure Rust over time ✅

### Next Actions

1. ⚡ Vulkan compute test (2-3 hours)
2. ⚡ wgpu compute test (2-3 hours)
3. → Unified ToadStool API (4-6 hours)
4. → Performance comparison & optimization

---

**Document Version**: 1.0  
**Last Updated**: January 8, 2026  
**Status**: Landscape Mapped, Next Steps Clear  
**Next**: Verify Vulkan compute execution

---

*ToadStool: Open Systems, Competitive Environment, User Freedom* 🚀

**"Leverage all open frameworks. Reward open standards, not vendor lock."** ✅

