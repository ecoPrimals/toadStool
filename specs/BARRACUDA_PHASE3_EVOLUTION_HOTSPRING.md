# BarraCUDA & ToadStool — Phase 3 Evolution Roadmap

**Date**: February 12, 2026  
**Source**: ecoPrimals Control Team (Eastgate) — hotSpring validation & evolution analysis  
**Status**: ACTIVE — Foundational for L3 nuclear physics and all scientific workloads  
**Validation**: 121/121 library tests passed against analytical references and scipy controls

---

## Executive Summary

BarraCUDA has evolved into an extraordinary platform: **436 WGSL shaders**, **540 Rust modules**,
**1,658 tests**, and **~16,800 lines** of scientific computing middleware. The shader-first
architecture is sound. The ML/DL layer is production-grade. The scientific computing modules
work correctly — all validation tests pass.

The evolution now shifts from **breadth** (more shaders) to **depth**:

1. **f64 CPU bridges** for existing GPU linalg ops
2. **Auto-dispatch** so users never worry about CPU vs GPU routing
3. **Heterogeneous pipeline orchestration** for multi-device workflows
4. **Missing math** only as workloads demand it

---

## Validated — No Changes Needed

These modules passed 121/121 tests against analytical references and Python scipy controls:

| Category | Tests | Status |
|----------|-------|--------|
| Special Functions | 69/69 | ✅ Production-ready |
| Linear Algebra | 10/10 | ✅ Production-ready |
| Optimizers & Numerical | 22/22 | ✅ Production-ready |
| MD Forces & Integrators | 20/20 | ✅ Production-ready |

---

## Priority #1: f64 Linalg Bridge

### Problem

`barracuda::linalg` exports only `solve_f64`. The GPU ops (`cholesky`, `eigh`, `LU`,
`QR`, `SVD`, `tridiagonal`) exist in `ops/linalg/` as shader wrappers but are **f32-only**
and aren't accessible as CPU f64 functions through the public API.

### Target API

```rust
// barracuda::linalg — Target API
pub use solve::solve_f64;           // ✅ exists
pub use cholesky::cholesky_f64;     // NEW
pub use eigh::eigh_f64;             // NEW — replaces nalgebra::SymmetricEigen
pub use lu::{lu_decompose_f64, lu_solve_f64, lu_det_f64};  // NEW
pub use qr::{qr_decompose_f64, qr_least_squares_f64};      // NEW
pub use svd::{svd_decompose_f64, svd_pinv_f64};            // NEW
pub use tridiagonal::tridiagonal_solve_f64;                // NEW
pub use inverse::inverse_f64;       // NEW
pub use determinant::determinant_f64;  // NEW
pub use triangular::triangular_solve_f64;  // NEW
```

### Effort: 3-5 days

---

## Priority #2: Auto-Dispatch System

### Problem

GPU dispatch for **single-point** surrogate predictions in Nelder-Mead inner loops
caused a **90× slowdown**. Every function needs intelligent routing.

### Pattern

```rust
pub fn erf(x: &[f64]) -> Vec<f64> {
    erf_cpu(x)  // always f64 CPU for scalar/small
}

pub fn erf_batch(x: &[f32], device: &WgpuDevice) -> Vec<f32> {
    if x.len() < ERF_GPU_THRESHOLD {
        x.iter().map(|&v| erf_cpu(&[v as f64])[0] as f32).collect()
    } else {
        erf_gpu(x, device)  // WGSL shader
    }
}

const ERF_GPU_THRESHOLD: usize = 512;  // determined by benchmarking
```

### Effort: 2-3 days

---

## Priority #3: EvaluationCache Persistence

### Problem

The `EvaluationCache` is in-memory only. Between runs, all data is lost.
L1 data should inform L2 classifier training (warm-starting).

### Target API

```rust
impl EvaluationCache {
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn load(path: &Path) -> Result<Self>;
    pub fn merge(&mut self, other: &EvaluationCache);
    pub fn to_training_data(&self) -> (Vec<Vec<f64>>, Vec<f64>);
}
```

### Effort: 1 day

---

## Priority #4: Missing Scientific Functions

### HIGH Priority (L3 nuclear physics + general science)

| Function | Module | Use Case | Effort |
|----------|--------|----------|--------|
| Generalized eigenvalue Ax = λBx | `linalg::gen_eigh` | HFB overlap matrix | 3-4 days |
| Incomplete gamma γ(a,x) | `special::inc_gamma` | Chi-squared CDF | 1-2 days |
| Newton-Raphson root-finding | `optimize::newton` | Nonlinear equations | 1 day |
| Brent's method | `optimize::brent` | Faster 1D root-finding | 1 day |
| Cubic spline interpolation | `numerical::spline` | EOS tables | 2 days |

### MEDIUM Priority (scientific completeness)

| Function | Module | Use Case | Effort |
|----------|--------|----------|--------|
| Arbitrary-order Bessel Jₙ | `special::bessel_jn` | Nuclear wavefunctions | 1-2 days |
| Gauss-Legendre quadrature | `numerical::gauss_legendre` | High-accuracy integrals | 1-2 days |
| Conjugate gradient | `optimize::cg` | Large sparse systems | 2-3 days |
| Chi-squared distribution | `stats::chi2` | Goodness-of-fit | 1 day |

---

## Priority #5: Heterogeneous Pipeline Orchestration

### What hotSpring Taught Us

The heterogeneous L2 pipeline manually orchestrates four tiers:

```
Tier 1: NMP pre-screen (CPU, ~1μs/candidate)     → 79% rejection
Tier 2: SEMF proxy    (CPU, ~0.1ms/candidate)    → 13% rejection
Tier 3: Classifier    (CPU/NPU, ~10μs/candidate) → 0% (low recall)
Tier 4: Full HFB      (CPU parallel, ~0.2s/eval) → 8% pass rate
```

This achieved 7.2× speedup over plain SparsitySampler.

### Target API

```rust
let pipeline = Pipeline::builder()
    .filter("nmp_screen", WorkloadHint::SmallWorkload, |params| { ... })
    .filter("semf_proxy", WorkloadHint::SmallWorkload, |params| { ... })
    .evaluate("hfb", WorkloadHint::LinearSolve, |params| { ... })
    .surrogate(RBFKernel::ThinPlateSpline)
    .optimizer(MultiStartNM::new(8, 100, 1e-8))
    .cache(EvaluationCache::load_or_new("cache.json"))
    .build();
```

### Effort: 5-7 days

---

## Priority #6: LOO-CV for Surrogate Quality

### Target API

```rust
impl RBFSurrogate {
    pub fn loo_cv_rmse(&self) -> f64;
    pub fn loo_cv_errors(&self) -> Vec<f64>;
}
```

The `loo_cv.wgsl` shader already exists — needs wiring.

### Effort: 1 day

---

## Hardware Evolution (When Titan V Arrives)

| Task | Priority | Effort |
|------|----------|--------|
| f64 WGSL shaders (port matmul_fp64 pattern) | HIGH | 2-3 weeks |
| Multi-GPU DevicePool (RTX 4070 f32, Titan V f64) | MEDIUM | 1-2 weeks |
| f64 Tensor type | MEDIUM | 1-2 weeks |
| GPU precision config (`Precision::F64`) | MEDIUM | 1 week |

### Precision Mode Strategy

```rust
pub enum PrecisionMode {
    F32,                    // Standard: all shaders use f32
    F64Emulated,            // No f64 hardware: split hi/lo emulation
    F64Native,              // Titan V / datacenter: native f64 in WGSL
    Mixed { threshold },    // Auto: f64 for small (CPU), f32 for large (GPU)
}
```

---

## Phased Roadmap

### Phase A — Bridge & Polish (1-2 weeks)

| Task | Priority | Effort |
|------|----------|--------|
| f64 linalg bridges | 🔴 HIGH | 3-5 days |
| Auto-dispatch benchmarks | 🔴 HIGH | 2-3 days |
| EvaluationCache serialization | 🔴 HIGH | 1 day |
| LOO-CV wiring | 🟡 MEDIUM | 1 day |

### Phase B — Scientific Depth (2-3 weeks)

| Task | Priority | Effort |
|------|----------|--------|
| Incomplete gamma + chi² | 🟡 MEDIUM | 1-2 days |
| Newton-Raphson + Brent | 🟡 MEDIUM | 1-2 days |
| Cubic spline | 🟡 MEDIUM | 2 days |
| Generalized eigenvalue | 🟡 MEDIUM | 3-4 days |

### Phase C — Hardware Exploitation (when Titan V arrives)

| Task | Priority | Effort |
|------|----------|--------|
| f64 Tensor type | 🟡 MEDIUM | 1-2 weeks |
| f64 WGSL shader variants | 🟡 MEDIUM | 2-3 weeks |
| Multi-GPU DevicePool | 🟡 MEDIUM | 1-2 weeks |

---

## Key Lessons from hotSpring

1. **GPU Dispatch Overhead Matters** — Single-point predictions must use CPU
2. **Surrogate Accuracy Gap Is Algorithmic** — 121/121 tests pass; remaining gap is application tuning
3. **Pre-Screening Cascades Are Powerful** — 91.9% rejection before expensive HFB
4. **f64 vs f32 Trade-offs Are Workload-Specific** — Dual-precision architecture is correct
5. **NMP-Aware Surrogates Improve Pass Rates** — Physics-informed surrogate is best practice

---

## Validation Strategy

Every new function validated in hotSpring:

```
Python control (scipy/numpy)  ←→  BarraCUDA (Rust/WGSL)
         ↓                              ↓
    reference results              candidate results
         ↓                              ↓
                 comparison.json
                      ↓
            accuracy + speedup metrics
```

---

**Source**: hotSpring heterogeneous L2 pipeline validation  
**Validation Data**: `hotSpring/control/surrogate/nuclear-eos/results/`
