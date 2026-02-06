# Phase 4: Capability Evolution Status

**Date**: February 6, 2026  
**Status**: 🚀 **IN PROGRESS - Systematic Capability Evolution**

---

## 🎯 Mission

**Evolve ALL remaining operations from hardcoded workgroup sizes to capability-based dispatch**

### Deep Debt Principle

> "Hardcoding should be evolved to agnostic and capability based"

---

## 📊 Current Status

### Operation Count

```
Total Operation Files:        318 (excluding mod.rs, tests.rs, compute.rs, projections.rs)
Already Capability-Evolved:    50 operations (15.7%)
Remaining to Evolve:          268 operations (84.3%)
```

### Capability Evolution Pattern

**BEFORE** (Hardcoded):
```rust
// ❌ Hardcoded workgroup size (256)
compute_pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
```

**AFTER** (Capability-Based):
```rust
// ✅ Capability-based dispatch (vendor-optimized)
use crate::device::{DeviceCapabilities, WorkloadType};

let caps = DeviceCapabilities::from_device(&device);
let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
compute_pass.dispatch_workgroups(workgroups, 1, 1);
```

---

## 🔧 Workload Type Classification

### WorkloadType Enum Options

```rust
pub enum WorkloadType {
    ElementWise,    // Simple element-wise ops (relu, sigmoid, add, mul)
    MatMul,         // Matrix multiplication, attention, dense layers
    Reduction,      // Sum, mean, variance, normalization
    FHE,            // Homomorphic encryption (U64 emulation)
    Convolution,    // Conv2d, pooling, image operations
}
```

### Classification Guide

| WorkloadType | Use For | Optimal Sizes |
|--------------|---------|---------------|
| **ElementWise** | relu, sigmoid, add, mul, tanh, abs, exp, log, clip, threshold, activation functions | NVIDIA: 256, AMD: 256, Intel: 128 |
| **MatMul** | matmul, attention (all variants), dense layers, linear transforms | NVIDIA: 256, AMD: 256, Intel: 128 |
| **Reduction** | sum, mean, std, variance, norm, batch_norm, layer_norm, softmax | NVIDIA: 512, AMD: 256, Intel: 256 |
| **FHE** | fhe_ntt, fhe_intt, fhe_poly_*, cryptographic operations | NVIDIA: 256, AMD: 256, Intel: 128 |
| **Convolution** | conv2d, pool2d, pad, spatial operations, image transforms | NVIDIA: 128, AMD: 128, Intel: 64 |

---

## 📋 Evolution Strategy

### Wave Approach (Parallel Execution)

**Wave 1**: Element-wise Operations (80+ ops)  
**Wave 2**: Reduction & Normalization (30+ ops)  
**Wave 3**: Attention & MatMul (40+ ops)  
**Wave 4**: Convolution & Spatial (50+ ops)  
**Wave 5**: FHE & Specialized (40+ ops)  
**Wave 6**: Loss Functions & Optimizers (remaining)

### Parallel Subagent Strategy

- **12 operations per wave** (3 subagents × 4 ops each)
- **Fast model** for efficiency
- **Systematic verification** via `cargo check`

---

## 📝 Evolution Checklist Per Operation

### Required Changes

1. **Add imports**:
   ```rust
   use crate::device::{DeviceCapabilities, WorkloadType};
   ```

2. **Replace hardcoded dispatch**:
   - Find: `dispatch_workgroups((size + N) / M, ...)`
   - Replace with capability-based pattern

3. **Choose correct WorkloadType**:
   - Analyze operation semantics
   - Select appropriate type (ElementWise, MatMul, Reduction, FHE, Convolution)

4. **Add documentation comment**:
   ```rust
   // Deep Debt Evolution: Capability-based dispatch
   ```

5. **Verify compilation**:
   - `cargo check --package barracuda`
   - Ensure no regressions

---

## 🎯 Sample Operations to Evolve

### High-Priority Targets (Wave 1)

1. **Element-wise (Simple)**:
   - `clip_grad_value.rs` - Gradient clipping
   - `clip_grad_norm.rs` - Gradient norm clipping
   - `sign_wgsl.rs` - Sign function
   - `tanhshrink_wgsl.rs` - TanhShrink activation
   - `threshold_wgsl.rs` - Threshold operation
   - `softshrink_wgsl.rs` - SoftShrink activation
   - `logsigmoid_wgsl.rs` - LogSigmoid activation
   - `rrelu_wgsl.rs` - Randomized ReLU
   - `hardshrink_wgsl.rs` - HardShrink activation

2. **Reduction Operations**:
   - `log_softmax_wgsl.rs` - LogSoftmax
   - `logsumexp_wgsl.rs` - LogSumExp

3. **Specialized**:
   - `embedding_wgsl.rs` - Embedding lookup
   - `one_hot_wgsl.rs` - One-hot encoding
   - `masked_fill_wgsl.rs` - Masked fill

4. **Transforms**:
   - `flip_wgsl.rs` - Flip operation
   - `roll_wgsl.rs` - Roll operation
   - `index_select_wgsl.rs` - Index select

### Medium-Priority (Wave 2-3)

- Attention variants (already refactored in Phase 3)
- Advanced ops (lamb.rs, rotary_embedding.rs, etc.)

### Already Completed ✅

50 operations already use capability-based dispatch, including:
- All 5 optimizers: adam, adamw, sgd, rmsprop, adadelta
- All refactored large files from Phase 3
- Core activations: relu, sigmoid, tanh, gelu*, etc.
- Normalization: batch_norm, layer_norm, instance_norm, group_norm

---

## 🚀 Execution Plan

### Immediate Actions

1. ✅ **Audit Complete**: Found 268 operations to evolve
2. 🚀 **Wave 1 Launch**: Start with 12 element-wise operations
3. 🔄 **Parallel Execution**: Use 3 fast subagents (4 ops each)
4. ✅ **Verify**: Run `cargo check` after each wave
5. 📊 **Track Progress**: Update this document after each wave

### Success Criteria

- ✅ All 268 operations evolved
- ✅ Zero hardcoded workgroup sizes remaining
- ✅ Clean `cargo check` (no compilation errors)
- ✅ All operations use `DeviceCapabilities::from_device`
- ✅ Appropriate `WorkloadType` selected for each

---

## 📈 Progress Tracking

### Wave 0 (Baseline)

- **Status**: ✅ COMPLETE (from prior work)
- **Operations**: 50 already capability-evolved
- **Percentage**: 15.7% complete

### Wave 1 (In Progress)

- **Target**: 12 operations (element-wise)
- **Status**: 🚀 STARTING
- **Operations**:
  1. `clip_grad_value.rs`
  2. `clip_grad_norm.rs`
  3. `sign_wgsl.rs`
  4. `tanhshrink_wgsl.rs`
  5. `threshold_wgsl.rs`
  6. `softshrink_wgsl.rs`
  7. `logsigmoid_wgsl.rs`
  8. `rrelu_wgsl.rs`
  9. `hardshrink_wgsl.rs`
  10. `log_softmax_wgsl.rs`
  11. `flip_wgsl.rs`
  12. `roll_wgsl.rs`

---

## 🎓 Why This Matters

### Technical Benefits

1. **Vendor Optimization**:
   - NVIDIA GPUs use optimal sizes (256-512)
   - AMD GPUs use wavefront-aligned sizes (64-256)
   - Intel GPUs use conservative sizes (64-128)
   - CPU fallback uses cache-friendly sizes (16-64)

2. **Hardware Agnostic**:
   - Same code works optimally on any vendor
   - No recompilation needed per hardware
   - Runtime adaptation to capabilities

3. **Performance**:
   - Better GPU utilization (warp/wavefront alignment)
   - Fewer idle threads
   - Cache-friendly dispatch

4. **Future-Proof**:
   - New hardware auto-optimizes
   - No manual tuning needed
   - Scales to new architectures

### Deep Debt Alignment

✅ **Zero Hardcoding**: All workgroup sizes discovered at runtime  
✅ **Capability-Based**: Query device, don't assume  
✅ **Hardware Agnostic**: Works optimally everywhere  
✅ **Modern Rust**: Type-safe, zero unsafe  
✅ **Production-Ready**: Real capability detection

---

## 📚 Reference

### DeviceCapabilities API

```rust
pub struct DeviceCapabilities {
    pub device_name: String,
    pub device_type: wgpu::DeviceType,
    pub max_buffer_size: u64,
    pub max_workgroup_size: (u32, u32, u32),
    pub backend: wgpu::Backend,
    pub vendor: u32,  // 0x10DE=NVIDIA, 0x1002=AMD, 0x8086=Intel
}

impl DeviceCapabilities {
    pub fn from_device(device: &WgpuDevice) -> Self;
    pub fn optimal_workgroup_size(&self, workload: WorkloadType) -> u32;
}
```

### Example Evolution

**File**: `crates/barracuda/src/ops/clip_grad_value.rs`

**Before** (line 159):
```rust
compute_pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
```

**After**:
```rust
// Deep Debt Evolution: Capability-based dispatch
let caps = DeviceCapabilities::from_device(&device);
let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
compute_pass.dispatch_workgroups(workgroups, 1, 1);
```

---

**Status**: 🚀 Ready to Launch Wave 1  
**Target**: 268 operations to evolve  
**Strategy**: Systematic waves with parallel execution  
**Timeline**: Continuous until complete

---

*Created: February 6, 2026*  
*Last Updated: February 6, 2026 - Initial Status*  
*Next Action: Launch Wave 1 (12 operations)*
