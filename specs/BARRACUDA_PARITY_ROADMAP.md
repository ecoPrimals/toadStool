# BarraCUDA Performance Parity Roadmap

**Date**: February 13, 2026  
**Status**: PROFILED — Optimization targets identified  
**Goal**: Achieve vendor-free CUDA/ROCm parity

---

## 1. Current Performance Gap

### Benchmark Results (16M elements, vector operations)

| Backend | Device | Add (μs) | Mul (μs) | Bandwidth (GB/s) | Gap vs CUDA |
|---------|--------|----------|----------|------------------|-------------|
| **CUDA (native)** | RTX 3090 | 232 | 229 | 826-839 | Baseline |
| BarraCUDA (wgpu) | RTX 3090 | 3097 | 2767 | 62-69 | **13x slower** |
| BarraCUDA (wgpu) | RX 6950 XT | 1449 | 812 | 132-236 | **4-6x slower** |

### Size-Dependent Scaling (validated Feb 13, 2026)

| Size | CUDA | BC/NVIDIA | BC/AMD | NVIDIA Gap | AMD Gap |
|------|------|-----------|--------|------------|---------|
| 1M | 16μs | 421μs | 230μs | 27x | 14x |
| 4M | 60μs | 438μs | 273μs | 7.3x | 4.5x |
| 16M | 232μs | 3097μs | 1449μs | 13x | 6.2x |

### Key Insight
AMD RX 6950 XT performs **2-3x better** than NVIDIA RTX 3090 via wgpu/Vulkan!
- RADV (Mesa) driver has lower Vulkan compute overhead
- NVIDIA proprietary driver optimized for CUDA, not Vulkan compute
- At 16M mul: AMD achieves 236 GB/s vs NVIDIA's 69 GB/s via wgpu

---

## 2. Bottleneck Analysis

### 2.1 API Overhead (Primary)
```
CUDA workflow:
  1. cuLaunchKernel() → 15μs
  
wgpu/Vulkan workflow:
  1. Create command encoder → ~50μs
  2. Set bind groups → ~20μs  
  3. Set pipeline → ~30μs
  4. Dispatch → ~10μs
  5. Submit → ~100μs
  6. Wait for fence → ~100μs
  Total: ~300μs overhead per operation
```

### 2.2 Pipeline State Switching
- Each operation creates new pipeline state
- Vulkan validation overhead (debug builds)
- Bind group allocation per dispatch

### 2.3 Memory Bandwidth Utilization
- CUDA: 763-828 GB/s (95% of theoretical 936 GB/s)
- BarraCUDA: 62-187 GB/s (7-20% of theoretical)
- Bottleneck: CPU-side command buffer setup, not GPU execution

---

## 3. Optimization Strategies

### Tier 1: Quick Wins (Expected: 3-5x improvement)

#### 1.1 Pre-compiled Pipeline Cache [IN PROGRESS]

**Status**: Infrastructure created (`device/pipeline_cache.rs`), needs per-device isolation fix.

The challenge: wgpu objects (BindGroupLayout, Pipeline) are tied to specific Device instances.
A global cache must key by device ID, but concurrent multi-device usage caused validation errors.

```rust
// Current: Pipeline compiled per dispatch
let pipeline = device.create_compute_pipeline(&desc);

// Target: Per-device pipeline cache
impl WgpuDevice {
    fn get_or_create_pipeline(&self, key: PipelineKey) -> Arc<ComputePipeline>;
}
```

**Next steps**:
1. Move pipeline cache into `WgpuDevice` struct (not global)
2. Pre-compile common pipelines at device creation
3. Warm cache on first use of each shader

#### 1.2 Persistent Command Buffers
```rust
// Current: New encoder per operation
let encoder = device.create_command_encoder(&desc);

// Optimized: Reuse encoders for repeated operations
struct PersistentEncoder {
    encoder: CommandEncoder,
    operations: Vec<QueuedOp>,
}
```

#### 1.3 Batch Multiple Operations
```rust
// Current: Submit after each op
tensor.add(&other)?; // submit
tensor.mul(&scalar)?; // submit

// Optimized: Batch before submit
graph.add(&tensor, &other);
graph.mul(&result, &scalar);
graph.execute(); // single submit
```

### Tier 2: Architecture (Expected: 2-3x improvement)

#### 2.1 Compute Graph / Lazy Execution
```rust
// Defer execution until result needed
let result = tensor.add(&a)?.mul(&b)?.sum();
// All ops fused into single dispatch
```

#### 2.2 Fused Kernels (FMA, etc.)
```wgsl
// Instead of separate add + mul
// c = a + b * alpha
@compute @workgroup_size(256)
fn fma(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    out[idx] = fma(alpha, a[idx], b[idx]);
}
```

#### 2.3 Async Compute Queue
```rust
// Use dedicated compute queue on AMD
// Parallel with graphics queue
let compute_queue = device.get_compute_queue();
```

### Tier 3: Advanced (Expected: 1.5-2x improvement)

#### 3.1 Cooperative Groups (wgpu extensions)
- Use subgroup operations for reductions
- Minimize shared memory bank conflicts

#### 3.2 Memory Layout Optimization
- Ensure coalesced memory access
- Use staging buffers for uploads

#### 3.3 Shader Occupancy Tuning
- Profile with nsight-compute / rocprof
- Tune workgroup sizes per GPU

---

## 4. Target Milestones

| Milestone | Target Gap | Strategy |
|-----------|------------|----------|
| **M1** | 5x slower | Pipeline cache + batch submit |
| **M2** | 3x slower | Fused kernels + compute graph |
| **M3** | 2x slower | Async queues + occupancy tuning |
| **M4** | 1.5x slower | Subgroup ops + memory optimization |
| **Parity** | 1.0-1.2x | Vendor-specific shader variants |

---

## 5. Validation Plan

### Benchmark Suite
```bash
# Run parity benchmark
cargo run -p cross-platform-showcase --bin parity_benchmark --release --features cuda

# Profile with nsight (NVIDIA)
nsight-compute --target-processes all ./target/release/parity_benchmark

# Profile with rocprof (AMD)
rocprof --stats ./target/release/parity_benchmark
```

### Parity Criteria
- [ ] Vector ops: <2x CUDA latency
- [ ] Matrix ops: <1.5x CUDA latency
- [ ] Memory bandwidth: >50% theoretical
- [ ] Cross-vendor: Same code, <20% performance variance

---

## 6. Reference Benchmarks

### Mature Projects to Compare Against

| Project | Domain | CUDA Baseline | Notes |
|---------|--------|---------------|-------|
| cuBLAS | Linear algebra | Best-in-class | GEMM reference |
| HOOMD-blue | Molecular dynamics | Production MD | Force kernels |
| LAMMPS | Molecular dynamics | HPC standard | Neighbor lists |
| PyTorch | Deep learning | Industry standard | Tensor ops |
| Thrust | Parallel algorithms | CUDA STL | Reductions |

### hotSpring Validation
```
# scipy-scale validation with BarraCUDA
cd ../hotSpring
cargo run --bin validate_hfb -- --backend barracuda
```

---

## 7. Success Criteria

**Phase 1 Complete** when:
1. BarraCUDA achieves <3x CUDA latency on vector ops
2. Same code runs on both NVIDIA and AMD
3. AMD performance within 20% of NVIDIA

**Parity Achieved** when:
1. BarraCUDA achieves <1.5x CUDA latency
2. Memory bandwidth >70% theoretical
3. hotSpring validates with <1% scipy deviation

---

## 8. Current Findings Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│  BARRACUDA PARITY PROFILE (Feb 13, 2026)                            │
├─────────────────────────────────────────────────────────────────────┤
│  Gap vs CUDA:        12x (NVIDIA), 5x (AMD)                         │
│  Primary bottleneck: wgpu command submission overhead               │
│  Best performer:     AMD RX 6950 XT (RADV driver)                   │
│  Bandwidth achieved: 7-20% of theoretical (vs CUDA's 95%)           │
│                                                                     │
│  OPTIMIZATION PATH:                                                 │
│  1. Pipeline cache + batch submit → 3-5x improvement               │
│  2. Fused kernels + compute graph → 2-3x improvement               │
│  3. Async queues + tuning → 1.5-2x improvement                     │
└─────────────────────────────────────────────────────────────────────┘
```
