# BarraCUDA Performance Parity Roadmap

**Date**: February 13, 2026  
**Status**: PIPELINE CACHING + MULTI-GPU FIXED  
**Goal**: Achieve vendor-free CUDA/ROCm parity with self-optimizing runtime

---

## 0. Latest Update: Pipeline Caching Fix (Feb 13, 2026)

### Root Cause Analysis

Latency breakdown benchmark revealed where the ~1200μs per-op overhead came from:

| Component | Time | % of Total |
|-----------|------|------------|
| **Shader compilation** | 450-500 μs | 37-40% |
| **Pipeline creation** | 180-500 μs | 18-41% |
| Bind group creation | 50-140 μs | 5-13% |
| Command encoding | 110-1100 μs | 10-100% |
| Submit + GPU + Wait | 150-360 μs | 13-35% |

**Key insight**: We were recompiling shaders and recreating pipelines on EVERY operation!
CUDA/ROCm compile kernels once at load time.

### Fix: Pipeline Caching with DeviceFingerprint

1. **Global pipeline cache** (`GLOBAL_CACHE`) stores shaders, layouts, and pipelines
2. **DeviceFingerprint** keys by adapter name + backend (not `global_id()` which was broken)
3. First call compiles/caches, subsequent calls reuse

### Results After Fix

| GPU | Cold (First) | Warm (Cached) | Speedup |
|-----|--------------|---------------|---------|
| RTX 3090 | 4,951 μs | 288-555 μs | **8.9x** |
| RX 6950 XT | 7,141 μs | 320-446 μs | **16x** |

Cache stats: 2 shaders, 2 layouts, 2 pipelines (1 per GPU) ✓

### Multi-GPU Bug Fixed

**Bug**: `device.global_id()` was identical across different wgpu instances, causing
cache collisions that led to "Bind group layout is invalid" errors.

**Fix**: Introduced `DeviceFingerprint` that hashes adapter name + backend + device type
to create truly unique keys per physical GPU.

### Remaining Gap to CUDA

The ~300-500μs floor comes from irreducible per-call overhead:
- Bind group creation (~100-150μs) - can't cache, references specific buffers
- Command encoding (~100μs)
- Submit + GPU sync (~150-200μs)

**To reach CUDA parity: Use TensorSession for batching**, not just caching.

---

## 1. Validated Performance Profile (Auto-Tuning Results)

### IMPORTANT: Previous Numbers Were Wrong

The earlier "2000+ GB/s" numbers from batched tests were **measurement artifacts** - they
didn't include proper GPU synchronization. The auto-tuning benchmark now uses correct
methodology with validation.

### Validated Benchmark Results (Feb 13, 2026)

| GPU | Peak Bandwidth | % Theoretical | Single-Op Latency | Optimal WG |
|-----|----------------|---------------|-------------------|------------|
| RTX 3090 | **176 GB/s** | 19% of 936 | 243 μs | 64 |
| RX 6950 XT | **137 GB/s** | 24% of 576 | 335 μs | 256 |

### Comparison to CUDA Baseline

| Metric | CUDA (RTX 3090) | BarraCUDA (RTX 3090) | Gap |
|--------|-----------------|----------------------|-----|
| Bandwidth | ~800 GB/s | 176 GB/s | 4.5x |
| Efficiency | 85% theoretical | 19% theoretical | - |
| Latency | ~15-50 μs | 243 μs | 5-16x |

### Key Findings

1. **NVIDIA achieves higher raw bandwidth** than AMD via wgpu (176 vs 137 GB/s)
2. **AMD achieves better % of theoretical** (24% vs 19%)
3. **Single-op overhead is massive** (~200-350μs) - this is the primary bottleneck
4. **Workgroup size matters differently per vendor**:
   - NVIDIA optimal: WG=64
   - AMD optimal: WG=256 (contrary to earlier shader_optimization_bench which showed 128)

## 2. Auto-Tuning Architecture [IMPLEMENTED]

### Runtime Calibration System

BarraCUDA now discovers optimal parameters at runtime rather than hardcoding vendor assumptions:

```rust
// Auto-calibration on first use
let device = WgpuDevice::new().await?;
let cal = device.get_calibration();  // Cached after first run

println!("Optimal WG: {}", cal.optimal_workgroup_size);
println!("Peak BW: {} GB/s", cal.peak_bandwidth_gbps);
```

### Benefits

1. **Silicon lottery handled**: Discovers actual card performance, not theoretical
2. **Unknown hardware works**: New cards (Titan V, future gens) auto-calibrate
3. **Driver updates captured**: Re-calibrate to discover driver improvements
4. **Per-card optimization**: Each physical GPU gets its own profile

### Files Added

- `crates/barracuda/src/device/autotune.rs` - Core auto-tuning infrastructure
- `showcase/cross-platform/src/autotune_bench.rs` - Validation benchmark
- `crates/barracuda/src/compute_graph.rs` - Lazy execution for batching

---

## 3. Previous Performance Analysis

### Benchmark Results (16M elements, vector operations)

| Backend | Device | Add (μs) | Mul (μs) | Bandwidth (GB/s) | Gap vs CUDA |
|---------|--------|----------|----------|------------------|-------------|
| **CUDA (native)** | RTX 3090 | 232 | 229 | 826-839 | Baseline |
| BarraCUDA (wgpu) | RTX 3090 | 3097 | 2767 | 62-69 | **13x slower** |
| BarraCUDA (wgpu) | RX 6950 XT | 1449 | 812 | 132-236 | **4-6x slower** |

### Size-Dependent Scaling

| Size | CUDA | BC/NVIDIA | BC/AMD | NVIDIA Gap | AMD Gap |
|------|------|-----------|--------|------------|---------|
| 1M | 16μs | 421μs | 230μs | 27x | 14x |
| 4M | 60μs | 438μs | 273μs | 7.3x | 4.5x |
| 16M | 232μs | 3097μs | 1449μs | 13x | 6.2x |

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
│  VALIDATED PERFORMANCE (auto-tune benchmark):                       │
│  ├── RTX 3090:  176 GB/s (19% theoretical), 243μs latency          │
│  └── RX 6950:   137 GB/s (24% theoretical), 335μs latency          │
│                                                                     │
│  Gap vs CUDA:        4-5x bandwidth, 5-16x latency                  │
│  Primary bottleneck: wgpu per-op overhead (~200-350μs)              │
│  Bandwidth achieved: 19-24% of theoretical (vs CUDA's 85%)          │
│                                                                     │
│  SESSION BATCHING RESULTS (session_benchmark):                      │
│  ├── 10 ops:  1.2-1.4x speedup                                     │
│  ├── 50 ops:  1.6-2.1x speedup                                     │
│  └── 100 ops: 1.9-2.4x speedup                                     │
│                                                                     │
│  NEW ARCHITECTURE:                                                  │
│  ✅ Auto-tuning runtime (discovers optimal WG per GPU)              │
│  ✅ Compute graph for lazy execution / batching                     │
│  ✅ TensorSession for automatic operation batching                  │
│  ⏳ ToadStool intelligent runtime (next)                            │
│                                                                     │
│  OPTIMIZATION PATH:                                                 │
│  1. ✅ Compute graph + batch submit → 1.2-2.4x improvement         │
│  2. ⏳ Fused kernels + memory reuse → 2-3x improvement             │
│  3. ⏳ Async queues + tuning → 1.5-2x improvement                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 9. ToadStool Intelligence Layer (Next Phase)

The auto-tuning infrastructure is now ready. The next evolution is making ToadStool
the intelligent orchestration layer:

### Planned Features

1. **Workload Classification**
   - Analyze operation patterns
   - Choose optimal batch size per workload type
   - Route to best available hardware

2. **Predictive Batching**
   - Learn common operation sequences
   - Pre-batch based on historical patterns
   - Speculative execution for low-latency paths

3. **Cross-Device Orchestration**
   - Use calibration data for load balancing
   - Route large ops to fastest GPU
   - Fall back gracefully when GPUs unavailable

4. **Continuous Learning**
   - Track actual vs predicted performance
   - Re-calibrate when drift detected
   - Adapt to thermal throttling

### Example Future API

```rust
// ToadStool handles everything
let runtime = ToadStool::auto().await?;

// Operations go through intelligent layer
let result = runtime.execute(|ctx| {
    let a = ctx.tensor(&[1.0, 2.0, 3.0]);
    let b = ctx.tensor(&[4.0, 5.0, 6.0]);
    
    // ToadStool automatically:
    // - Batches these operations
    // - Routes to optimal GPU
    // - Uses calibrated workgroup sizes
    a.add(&b)?.mul(&b)?
})?;
```
