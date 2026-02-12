# Generic Precision Evolution — Investigation

**Date**: February 12, 2026  
**Status**: INVESTIGATION — Architectural decision pending

---

## Question

Can we evolve BarraCUDA to "any fp" instead of hardcoded f32/f64?

Could embedded systems use fp8/bf16/f16 while still using the same algorithms?

---

## Current State

### CPU Code
- Hardcoded `f64` for precision-critical paths (linalg, special functions)
- Hardcoded `f32` for GPU tensor operations
- No generic `Float` trait abstraction

### GPU WGSL
- Hardcoded `f32` (WGSL spec limitation)
- f64 emulation via hi/lo f32 pairs (`matmul_fp64.wgsl` pattern)
- No native f64 on consumer GPUs (1/32 rate vs f32)

### Dependencies
- No `num-traits` crate currently used
- `nalgebra` used downstream (has generic precision)

---

## Analysis

### Option 1: `num-traits::Float` for CPU

**Approach**: Use `num-traits::Float` trait for generic CPU implementations.

```rust
use num_traits::Float;

pub fn solve<F: Float>(a: &[F], b: &[F], n: usize) -> Result<Vec<F>> {
    // Works with f32, f64, or any type implementing Float
}
```

**Pros**:
- Clean abstraction for CPU code
- Works with f32/f64 without code duplication
- Pure Rust, no FFI

**Cons**:
- Doesn't help GPU (WGSL is always f32)
- Need precision-specific tolerances (1e-14 for f64 vs 1e-6 for f32)
- Performance overhead for trait dispatch (negligible but present)

### Option 2: Compile-Time Feature Flags

**Approach**: Use Cargo features to select precision at compile time.

```toml
[features]
f32 = []
f64 = []
default = ["f64"]
```

```rust
#[cfg(feature = "f64")]
type Scalar = f64;

#[cfg(feature = "f32")]
type Scalar = f32;
```

**Pros**:
- Zero runtime overhead
- Single binary per precision target
- Clear separation of precision-sensitive code

**Cons**:
- Need separate builds for f32 vs f64
- WGSL still can't be generic
- Doesn't help embedded (fp8/bf16/f16)

### Option 3: Runtime Precision Mode

**Approach**: Select precision at runtime based on hardware and workload.

```rust
pub enum PrecisionMode {
    F32,                    // Standard GPU (all consumer hardware)
    F64Emulated,            // Split hi/lo f32 pairs (matmul_fp64.wgsl)
    F64Native,              // Titan V / datacenter GPUs with fp64 extensions
    Mixed { threshold },    // f64 CPU for small, f32 GPU for large
    Auto,                   // Let system decide based on hardware caps
}
```

**Pros**:
- Flexible, hardware-adaptive
- Single binary works everywhere
- Matches hotSpring's dual-precision pattern

**Cons**:
- Runtime dispatch overhead
- Complex code paths
- WGSL still fundamentally f32

### Option 4: Half-Precision / Low-Precision Specialization

**For embedded/edge**: fp8, bf16, f16 have fundamentally different characteristics:

| Type | Range | Precision | Use Case |
|------|-------|-----------|----------|
| fp8 (e4m3) | ~±240 | 3 mantissa bits | Inference, quantized models |
| bf16 | ~±3.4e38 | 7 mantissa bits | Training, TPU |
| f16 | ~±65504 | 10 mantissa bits | Graphics, inference |
| f32 | ~±3.4e38 | 23 mantissa bits | Standard compute |
| f64 | ~±1.8e308 | 52 mantissa bits | Scientific computing |

**Challenge**: Algorithms need different implementations for low precision:
- Different overflow/underflow handling
- Different tolerance thresholds
- Different accumulation strategies (Kahan summation for f16)
- Different numerical stability requirements

**Recommendation**: Keep low-precision as separate implementations, not generic.

---

## Recommended Approach

### Phase 1: Auto-Dispatch (Now)

```rust
pub struct DispatchConfig {
    pub cpu_threshold: usize,      // Size below which CPU is used
    pub gpu_available: bool,       // Hardware detection
    pub force_cpu: bool,           // User override for precision
    pub precision: PrecisionMode,  // F32, F64, Mixed, Auto
}
```

Each function follows the pattern:

```rust
pub fn erf(x: &[f64]) -> Vec<f64> {
    erf_cpu_f64(x)  // Always CPU for scalar/small + precision
}

pub fn erf_batch(x: &[f32], device: &WgpuDevice) -> Vec<f32> {
    if x.len() < ERF_GPU_THRESHOLD {
        x.iter().map(|&v| erf_cpu_f64(&[v as f64])[0] as f32).collect()
    } else {
        erf_gpu(x, device)  // WGSL shader
    }
}
```

### Phase 2: Generic CPU (When Titan V Arrives)

Add `num-traits` dependency and make CPU implementations generic:

```rust
use num_traits::Float;

pub fn solve<F: Float + Send + Sync>(a: &[F], b: &[F], n: usize) -> Result<Vec<F>>
where
    F: std::iter::Sum,
{
    // Generic implementation
}
```

### Phase 3: f64 WGSL (When Hardware Supports)

When WGSL/WebGPU adds f64 extensions (expected 2026-2027):

```wgsl
enable f64;

@compute @workgroup_size(256)
fn matmul_f64(@builtin(global_invocation_id) gid: vec3<u32>) {
    let a: f64 = input_a[gid.x];  // Native f64
    // ...
}
```

### Phase 4: Low-Precision Specialization (If Needed)

Separate modules for fp8/bf16/f16 with purpose-built implementations:

```
barracuda/
├── linalg/         # Standard f32/f64
├── linalg_fp16/    # Half-precision specialized
├── linalg_bf16/    # BFloat16 specialized
└── linalg_int8/    # Quantized specialized
```

---

## Decision

**Adopt the Mixed/Auto approach from Option 3** as the immediate path forward.

1. Add `PrecisionMode` enum to `barracuda::config`
2. Add auto-dispatch with size thresholds to each function
3. Keep WGSL at f32, add f64 CPU bridges
4. Defer generic `num-traits` until Titan V validates the pattern
5. Keep fp8/bf16/f16 as separate specializations (not generic)

This matches hotSpring's validated dual-precision architecture and doesn't require
rewriting the shader infrastructure.

---

## References

- hotSpring L2 heterogeneous pipeline: 7.2× speedup with precision-aware dispatch
- `matmul_fp64.wgsl`: existing f64 emulation pattern
- ADR-001: wgpu over CUDA/OpenCL (shader-first principle)
- WebGPU f64 extensions: https://github.com/gpuweb/gpuweb/issues/2805
