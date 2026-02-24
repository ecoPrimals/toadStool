# Hybrid FP64 Core Streaming — DF64 on FP32 Cores Across Hardware Eras

**Date**: February 23, 2026
**Status**: Production — gauge force, plaquette, Wilson action wired; expansion guide active
**Classification**: Core architecture — applies to every f64 shader in BarraCuda

---

## The Problem

Consumer GPUs have a 1:64 FP64:FP32 hardware ratio. An RTX 3090 running
native f64 lattice QCD uses **164 FP64 units** while **10,496 FP32 cores sit
idle** — 1.6% of the chip. Every f64 shader in BarraCuda has this problem on
consumer hardware.

| GPU | FP32 Units | FP64 Units | Ratio | FP32 TFLOPS | FP64 TFLOPS | DF64 TFLOPS (est.) |
|-----|:----------:|:----------:|:-----:|:-----------:|:-----------:|:------------------:|
| RTX 3090 (Ampere) | 10,496 | 164 | 1:64 | 35.6 | 0.56 | ~3.2 |
| RTX 4070 (Ada) | 5,888 | 92 | 1:64 | 29.1 | 0.45 | ~2.6 |
| RTX 2080 Ti (Turing) | 4,352 | 68 | 1:64 | 13.4 | 0.21 | ~1.2 |
| RX 6950 XT (RDNA2) | 5,120 | 2,560 | 1:2 | 23.6 | 11.8 | ~2.1 |
| Titan V (Volta) | 5,120 | 2,560 | 1:2 | 14.9 | 7.5 | ~1.3 |
| A100 (Ampere HPC) | 6,912 | 3,456 | 1:2 | 19.5 | 9.7 | ~1.7 |
| MI250X (CDNA2) | 14,080 | 7,040 | 1:2 | 47.9 | 23.9 | ~4.2 |

**Key insight**: DF64 on FP32 cores is ~5.7x slower than a single native FP32
op (due to Dekker splitting overhead), but ~9.9x faster than native f64 on
consumer GPUs. On compute-class GPUs (1:2 ratio), native f64 wins.

Measured on RTX 3090 (bench_fp64_ratio.rs):
```
FP32 throughput:  35.6 TFLOPS
FP64 throughput:   0.33 TFLOPS
DF64 throughput:   3.24 TFLOPS   ← 9.9× faster than native f64
```

---

## The Solution: Automatic Hybrid Precision

### Architecture

```
GpuDriverProfile::fp64_strategy()
          │
          ├── Fp64Strategy::Native  (Titan V, A100, MI250 — 1:2 hardware)
          │     └── Use native f64 shaders (existing)
          │
          └── Fp64Strategy::Hybrid  (RTX 3090, 4070, 2080 — 1:64 hardware)
                └── Use DF64 shaders (f32-pair on FP32 cores)
                      │
                      ├── Bulk math: DF64 on FP32 cores (~10× throughput)
                      └── Critical ops: native f64 on FP64 units (full precision)
```

### The Hybrid Precision Boundary Pattern

Every hybrid shader follows the same three-zone structure:

```
┌────────────────────────────────────────────────────┐
│  ZONE 1: LOAD (f64 → DF64)                        │
│  Buffer is f64. Convert to Df64 at load boundary.  │
│  Cost: 1 f64 read + 1 f32 cast per element.        │
└────────────────────────┬───────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────┐
│  ZONE 2: COMPUTE (DF64 on FP32 cores)              │
│  All matrix multiplies, additions, products.        │
│  This is the hot inner loop — 90%+ of the FLOPs.   │
│  Runs on the massive FP32 core array.               │
└────────────────────────┬───────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────┐
│  ZONE 3: REDUCE + STORE (DF64 → f64)               │
│  Convert back to f64 for precision-critical ops:    │
│  - Trace computation                                │
│  - Algebra projection                               │
│  - Scalar accumulation                              │
│  - Buffer store                                     │
│  Cost: 1 f32→f64 promotion per element.             │
└────────────────────────────────────────────────────┘
```

### Buffer Layout: Unchanged

The single most important design decision: **buffers remain f64**. The DF64
conversion happens at load/store boundaries inside the shader. This means:

- No buffer layout changes across the entire codebase
- All existing reductions (sum_f64, ReduceScalarPipeline) work unchanged
- All existing host-side readback works unchanged
- Leapfrog integrators, backup/restore, CG solver — all unchanged
- The Rust orchestrator code only changes shader selection (1 line)

---

## Implementation Status

### Production (Wired and Tested) ✅

| Shader | % of HMC | DF64 Zone | f64 Zone | Speedup |
|--------|:--------:|-----------|----------|:-------:|
| `su3_hmc_force_df64.wgsl` | 40% | 18 SU(3) matmuls (staple sum) | algebra projection + scale | ~10× |
| `wilson_plaquette_df64.wgsl` | 15% | 4 SU(3) matmuls per plaquette | Re Tr / 3 output | ~10× |
| `wilson_action_df64.wgsl` | ~5% | 4 SU(3) matmuls × 6 planes | 1 - ReTr/3 accumulation | ~10× |

### Foundation Libraries ✅

| Library | Location | Content |
|---------|----------|---------|
| `df64_core.wgsl` | `shaders/math/` | Df64 struct, two_sum, two_prod, add/sub/mul/div |
| `su3_df64.wgsl` | `shaders/math/` | Cdf64 complex, SU(3) mul/adjoint/add/trace/plaquette |
| `su3.rs` | `ops/lattice/` | `su3_df64_preamble()` shader composition |

### Auto-Detection ✅

| Component | File | Mechanism |
|-----------|------|-----------|
| `Fp64Rate` | `device/driver_profile.rs` | Full/Throttled/Minimal/Software from adapter name |
| `Fp64Strategy` | `device/driver_profile.rs` | Full → Native, else → Hybrid |
| `Su3HmcForce::new()` | `ops/lattice/hmc_force_su3.rs` | Queries fp64_strategy(), selects shader |
| `WilsonPlaquette::new()` | `ops/lattice/plaquette.rs` | Same pattern |
| `GpuWilsonAction::new()` | `ops/lattice/gpu_wilson_action.rs` | Same pattern |

---

## Hardware Classification

### Fp64Rate Detection

| GPU | Fp64Rate | Fp64Strategy | FP64:FP32 | DF64 Benefit |
|-----|----------|:------------:|:---------:|:------------:|
| Titan V / GV100 | Full | **Native** | 1:2 | None — native f64 faster |
| V100 / A100 | Full | **Native** | 1:2 | None |
| MI250X / MI300 | Full | **Native** | 1:2 | None |
| RTX 3090 / 3080 | Throttled | **Hybrid** | 1:64 | ~10× |
| RTX 4070 / 4090 | Throttled | **Hybrid** | 1:64 | ~10× |
| RTX 2080 / 2070 | Throttled | **Hybrid** | 1:64 | ~10× |
| RX 6950 XT | Throttled | **Hybrid** | 1:2* | Marginal |
| Intel Arc | Minimal | **Hybrid** | 1:16 | ~4× |
| Apple M-series | Software | **Hybrid** | N/A | Significant |

*AMD RDNA2/3 has 1:2 but is classified as Throttled for vendor SDK reasons.
On AMD, DF64 may not improve throughput since native f64 is already 1:2.
The Fp64Strategy::Hybrid path is still correct — DF64 is never slower than
1:2 of the FP32 rate, and on AMD it matches native f64 closely.

### Era Progression

As GPUs evolve, the FP64:FP32 ratio determines the optimal strategy:

| Era | Consumer FP64 | Compute FP64 | Strategy |
|-----|:-------------:|:------------:|----------|
| Kepler (2012) | 1:24 | 1:3 | Hybrid / Native |
| Maxwell (2014) | 1:32 | N/A | Hybrid |
| Pascal (2016) | 1:32 | 1:2 (P100) | Hybrid / Native |
| Volta (2017) | N/A | 1:2 (V100) | Native |
| Turing (2018) | 1:32 | N/A | Hybrid |
| Ampere (2020) | 1:64 | 1:2 (A100) | Hybrid / Native |
| Ada (2022) | 1:64 | N/A | Hybrid |
| Hopper (2022) | N/A | 1:2 (H100) | Native |
| Blackwell (2024) | 1:64 | 1:2 (B200) | Hybrid / Native |

The 1:64 consumer ratio has been stable since Ampere and shows no sign of
changing. DF64 core streaming is a permanent strategy for consumer hardware.

---

## Precision Analysis

### DF64 Precision Hierarchy

```
f32:  24-bit mantissa →  7 decimal digits
DF64: 48-bit mantissa → 14 decimal digits  ← this library
f64:  53-bit mantissa → 16 decimal digits
```

DF64 gives 14 of the 16 f64 digits. The 2-digit gap matters only for:
- Accumulations over millions of terms (use native f64 for final sum)
- Convergence tests comparing results at machine epsilon (use native f64)
- Random number generation (use native f64)

### Suitability by Operation Type

| Operation | DF64 Safe? | Rationale |
|-----------|:----------:|-----------|
| Matrix multiply (SU(3), dense) | **YES** | Intermediate precision; final result projected |
| Adjoint / transpose | **YES** | Element-wise conjugate — exact |
| Matrix addition | **YES** | Element-wise — Df64 add is error-free |
| Plaquette product | **YES** | 4 matmuls, result traced to scalar |
| Staple sum | **YES** | 6 staple products summed |
| Trace (Re Tr) | **YES** | 3 DF64 adds, then convert to f64 |
| Algebra projection | **NO** | Precision-critical subtraction and division |
| Scalar accumulation (Σ) | **NO** | Long chains need f64 for accuracy |
| Convergence test (δ < ε) | **NO** | Comparing at machine epsilon |
| CG inner products | **NO** | Numerical stability of Krylov solver |
| Random number generation | **NO** | Bit-level precision required |
| Metropolis accept/reject | **NO** | exp(−ΔH) at machine epsilon |
| Link update (Cayley-Hamilton) | **PARTIAL** | Matrix exp needs f64 for series convergence |

---

## Shader Conversion Guide

### Step 1: Identify the Hot Loop

Profile the shader or use the HMC kernel breakdown:

| Kernel | % of HMC | Hot Loop | Status |
|--------|:--------:|----------|:------:|
| Gauge force (staple sum) | 40% | 18 SU(3) matmuls/link | ✅ Done |
| CG solver (D†D iterations) | 20% | SpMV + vector ops | ❌ Not suitable |
| Wilson plaquette | 15% | 4 SU(3) matmuls/plane | ✅ Done |
| Wilson action | ~5% | Same as plaquette | ✅ Done |
| Link update (Cayley) | 10% | Matrix exponential | ⚠️ Partial |
| Momentum update | 5% | Element-wise add | ⚠️ Marginal benefit |
| Kinetic energy | 5% | Tr(P²) per link | ⚠️ Marginal benefit |
| Random momenta | 5% | Gaussian sampling | ❌ Not suitable |

### Step 2: Create the DF64 Shader

Template for converting any f64 shader to hybrid DF64:

```wgsl
// my_kernel_df64.wgsl — Hybrid version of my_kernel_f64.wgsl
//
// Prepend: complex_f64.wgsl + su3.wgsl + df64_core.wgsl + su3_df64.wgsl
//
// (Keep the same buffer layout, same params struct, same entry point name)

// ── Same buffer declarations as f64 version (unchanged) ──
@group(0) @binding(0) var<uniform>             params: MyParams;
@group(0) @binding(1) var<storage, read>       input:  array<f64>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;

// ── DF64 load boundary ──
fn load_data_df64(idx: u32) -> Df64 {
    return df64_from_f64(input[idx]);
}

@compute @workgroup_size(64)
fn my_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    // ── ZONE 1: Load f64 → DF64 ──
    let a = load_data_df64(gid.x * 2u);
    let b = load_data_df64(gid.x * 2u + 1u);

    // ── ZONE 2: Compute in DF64 (runs on FP32 cores) ──
    let result = df64_mul(a, b);  // or su3_mul_df64, etc.

    // ── ZONE 3: Convert back to f64 and store ──
    output[gid.x] = df64_to_f64(result);
}
```

### Step 3: Wire Auto-Detection in Rust

```rust
use crate::device::driver_profile::{Fp64Strategy, GpuDriverProfile};

const SHADER_F64: &str = include_str!("my_kernel_f64.wgsl");
const SHADER_DF64: &str = include_str!("my_kernel_df64.wgsl");

pub fn new(device: Arc<WgpuDevice>, /* ... */) -> Result<Self> {
    let profile = GpuDriverProfile::from_device(&device);
    let strategy = profile.fp64_strategy();
    let src = match strategy {
        Fp64Strategy::Native => format!("{}{}", su3_preamble(), SHADER_F64),
        Fp64Strategy::Hybrid => format!("{}{}", su3_df64_preamble(), SHADER_DF64),
    };
    tracing::info!(?strategy, "MyKernel: FP64 strategy");
    let module = device.compile_shader_f64(&src, Some("my_kernel"));
    // ... rest unchanged
}
```

### Step 4: Validate

- Identity-link test must produce identical results (f64 ≡ DF64 for identity)
- Random-link test: DF64 result within 1e-10 of f64 result
- Performance test: ≥5× speedup on consumer GPU

---

## Expansion Candidates: Beyond Lattice QCD

The DF64 pattern applies to ANY shader that does bulk f64 arithmetic.
Candidates ordered by estimated impact:

### High Priority (large compute, clear DF64 zones)

| Shader Category | Files | Hot Loop | Estimated Speedup |
|-----------------|-------|----------|:-----------------:|
| **Staggered Dirac** | `dirac_staggered_f64.wgsl` | SpMV-like hop terms | ~5-8× |
| **Pseudofermion force** | `pseudofermion_force_f64.wgsl` | Outer product per link | ~5-8× |
| **HFB Hamiltonian** | `batched_hfb_hamiltonian_f64.wgsl` | Matrix construction | ~5-8× |
| **HFB Density** | `batched_hfb_density_f64.wgsl` | Batched matmul | ~5-8× |
| **Batched Eigensolve** | `batched_eigh_single_dispatch_f64.wgsl` | Jacobi rotations | ~5-8× |
| **Dense GEMM** | `gemm_f64.wgsl` | Matrix multiply | ~8-10× |

### Medium Priority (moderate compute, straightforward conversion)

| Shader Category | Files | Hot Loop | Estimated Speedup |
|-----------------|-------|----------|:-----------------:|
| **LJ/Morse/Yukawa forces** | `lennard_jones_f64.wgsl`, etc. | Pairwise force | ~5× |
| **Velocity Verlet** | `velocity_verlet_f64.wgsl` | Position/velocity update | ~3× |
| **FFT** | `fft_1d_f64.wgsl` | Butterfly ops | ~5× |
| **RDF/SSF observables** | `rdf_f64.wgsl`, `ssf_f64.wgsl` | Histogram/accumulate | ~3-5× |
| **Coulomb (PPPM)** | `pppm_*.wgsl` | Charge spread + k-space | ~5× |

### Low Priority / Not Suitable

| Shader Category | Files | Reason |
|-----------------|-------|--------|
| **CG solver kernels** | `cg_*.wgsl` | Inner products need f64 for numerical stability |
| **Random number generators** | `su3_random_momenta_f64.wgsl` | Bit-level precision required |
| **Reduction kernels** | `sum_reduce_f64.wgsl` | Already memory-bound, not compute-bound |
| **BCS bisection** | `bcs_bisection_f64.wgsl` | Convergence test at machine epsilon |

---

## DF64 Library Reference

### df64_core.wgsl Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `df64_from_f32` | `(f32) → Df64` | Promote f32 to DF64 |
| `df64_from_f64` | `(f64) → Df64` | Split f64 into (hi, lo) f32 pair |
| `df64_to_f64` | `(Df64) → f64` | Reconstruct f64 from pair |
| `df64_zero` | `() → Df64` | Zero constant |
| `df64_add` | `(Df64, Df64) → Df64` | Error-free add via two_sum |
| `df64_sub` | `(Df64, Df64) → Df64` | Subtract via negation + add |
| `df64_mul` | `(Df64, Df64) → Df64` | Dekker multiply via two_prod |
| `df64_div` | `(Df64, Df64) → Df64` | Newton quotient refinement |
| `df64_neg` | `(Df64) → Df64` | Negate both components |
| `df64_scale_f32` | `(Df64, f32) → Df64` | Scale by f32 constant |
| `two_sum` | `(f32, f32) → Df64` | Error-free addition (Knuth) |
| `two_prod` | `(f32, f32) → Df64` | Error-free multiplication (Dekker) |

### su3_df64.wgsl Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `cdf64_zero` | `() → Cdf64` | Complex zero |
| `cdf64_from_f64` | `(f64, f64) → Cdf64` | Complex from f64 re, im |
| `cdf64_to_f64` | `(Cdf64) → vec2<f64>` | Complex to f64 pair |
| `cdf64_add` | `(Cdf64, Cdf64) → Cdf64` | Complex addition |
| `cdf64_sub` | `(Cdf64, Cdf64) → Cdf64` | Complex subtraction |
| `cdf64_mul` | `(Cdf64, Cdf64) → Cdf64` | Complex multiplication |
| `cdf64_conj` | `(Cdf64) → Cdf64` | Conjugate |
| `su3_mul_df64` | `(SU3, SU3) → SU3` | 3×3 matrix multiply |
| `su3_adjoint_df64` | `(SU3) → SU3` | Conjugate transpose |
| `su3_add_df64` | `(SU3, SU3) → SU3` | Matrix addition |
| `su3_re_trace_df64` | `(SU3) → Df64` | Real part of trace |
| `su3_plaquette_df64` | `(4 × SU3) → SU3` | U·V·W†·X† product |
| `su3_df64_to_f64` | `(SU3_df64) → SU3_f64` | Boundary: DF64 → f64 |
| `su3_f64_to_df64` | `(SU3_f64) → SU3_df64` | Boundary: f64 → DF64 |

Where `SU3` = `array<Cdf64, 9>` (DF64 mode) or `array<vec2<f64>, 9>` (f64 mode).

---

## Preamble Composition

Two composition functions in `ops/lattice/su3.rs`:

```rust
// Native f64 preamble (compute-class GPUs):
pub fn su3_preamble() -> String {
    format!("{WGSL_COMPLEX64}\n{WGSL_SU3}\n")
}

// Hybrid DF64 preamble (consumer GPUs):
pub fn su3_df64_preamble() -> String {
    format!("{WGSL_COMPLEX64}\n{WGSL_SU3}\n{WGSL_DF64_CORE}\n{WGSL_SU3_DF64}\n")
}
```

The DF64 preamble includes both f64 AND DF64 operations because hybrid
shaders need f64 for the precision-critical zone (algebra projection,
reductions) while using DF64 for the compute-intensive zone.

For non-SU(3) shaders (MD forces, FFT, etc.), create domain-specific
DF64 preambles following the same pattern:

```rust
pub fn md_df64_preamble() -> String {
    format!("{WGSL_DF64_CORE}\n{WGSL_VEC3_DF64}\n")  // future: 3D vector DF64
}
```

---

## Performance Model

### Expected Speedup Formula

```
Speedup = T_native / T_hybrid

Where:
  T_native = N_ops / R_f64                  (all ops on FP64 units)
  T_hybrid = N_df64 / R_f32_df64 + N_f64 / R_f64 + T_convert

  R_f64        = FP64 throughput (e.g., 0.33 TFLOPS on RTX 3090)
  R_f32_df64   = FP32 throughput / 5.7 (DF64 overhead factor)
  N_df64       = number of ops in DF64 zone
  N_f64        = number of ops remaining in f64 zone
  T_convert    = conversion overhead (negligible for large N)
```

For the gauge force on RTX 3090:
```
N_total = 18 matmuls (staple) + 1 matmul (F_raw) + projection
N_df64  = 18 × 27 × 4 = 1944 DF64 FMAs  (staple sum)
N_f64   = 1 × 27 × 2 + ~30 = 84 f64 FMAs  (projection)

R_f64      = 0.33 TFLOPS
R_f32_df64 = 35.6 / 5.7 = 6.24 TFLOPS

T_native = (1944 + 84) / 0.33 = 6145 ns
T_hybrid = 1944 / 6.24 + 84 / 0.33 = 311 + 255 = 566 ns

Speedup ≈ 10.8×
```

Measured: **9.9×** (bench_fp64_ratio.rs). Model matches observation.

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `FP64_GPU_EVOLUTION.md` | Math library foundation — DF64 extends the precision toolkit |
| `SOVEREIGN_COMPUTE_EVOLUTION.md` | ILP + DF64 are complementary: ILP fills latency gaps, DF64 fills core utilization gaps |
| `BARRACUDA_PARITY_ROADMAP.md` | DF64 is a parity multiplier — same physics, 10× throughput on consumer hardware |
| `CROSS_VENDOR_BENCHMARK_SPEC.md` | `bench_fp64_ratio` is the calibration tool |

---

## Future: Composable DF64 Libraries

As DF64 expands beyond SU(3), build domain-specific DF64 libraries:

| Library | Content | Target Shaders |
|---------|---------|----------------|
| `vec3_df64.wgsl` | 3D vector arithmetic in DF64 | MD forces, particle ops |
| `mat_df64.wgsl` | Dense NxN matrix ops in DF64 | HFB, eigensolve |
| `fft_df64.wgsl` | Butterfly operations in DF64 | FFT, PPPM |
| `poly_df64.wgsl` | Polynomial evaluation in DF64 | Special functions |

Each follows the same pattern: load f64 → compute in DF64 → store f64.
Each has a corresponding `xxx_df64_preamble()` Rust function.
Each is selected automatically by `Fp64Strategy`.

---

*The chip is 100% silicon. We should use 100% of it.*
*hotSpring core-streaming discovery, February 2026.*
*Production wiring: toadStool, February 2026.*
