# BarraCUDA & ToadStool — Phase 3 Evolution Roadmap

**Date**: February 12, 2026 (Updated)  
**Source**: ecoPrimals Control Team (Eastgate) — hotSpring validation & evolution analysis  
**Status**: ✅ **Phase A & Phase B COMPLETE** — Phase C awaiting hardware  
**Validation**: 121/121 library tests passed against analytical references and scipy controls

---

## Progress Update (February 12, 2026)

### Phase A — Bridge & Polish ✅ COMPLETE

| Task | Status | Implementation |
|------|--------|----------------|
| f64 linalg bridges | ✅ DONE | `linalg::cholesky_f64`, `linalg::eigh_f64`, re-exports for LU/QR/SVD/Tridiagonal |
| Auto-dispatch system | ✅ DONE | `dispatch` module with `DispatchConfig`, `DispatchTarget`, per-op thresholds |
| EvaluationCache serialization | ✅ DONE | `save()`, `load()`, `load_or_new()`, `from_training_data()` via serde_json |
| LOO-CV wiring | ✅ DONE | `RBFSurrogate::loo_cv_rmse()`, `loo_cv_errors()` |

### Phase B — Scientific Depth ✅ COMPLETE

| Task | Status | Implementation |
|------|--------|----------------|
| Incomplete gamma + chi² | ✅ DONE | `special::gamma` (regularized P/Q), `special::chi_squared` (CDF/quantile/test) |
| Newton-Raphson + Brent | ✅ DONE | `optimize::newton`, `optimize::brent`, `optimize::secant` |
| Cubic spline | ✅ DONE | `interpolate::CubicSpline` with natural/clamped/not-a-knot boundaries |
| Generalized eigenvalue | ✅ DONE | `linalg::gen_eigh_f64` via Cholesky reduction |

### Deep Debt Review ✅ COMPLETE

| Item | Status | Finding |
|------|--------|---------|
| Unsafe code in linalg | ✅ VERIFIED | No unsafe blocks - all pure safe Rust |
| Mock isolation | ✅ VERIFIED | Mocks feature-gated (`#[cfg(feature = "mock-tpu")]`) or in test modules |

### Remaining Work

- **Phase C hardware exploitation** — Waiting for Titan V arrival (f64 WGSL, multi-GPU DevicePool)

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

## Priority #1: f64 Linalg Bridge — ✅ COMPLETE

### Solution Implemented

Full f64 CPU implementations for all core linear algebra operations:

```rust
// barracuda::linalg — Current API (all working)
pub use solve::solve_f64;           // ✅ 
pub use cholesky::cholesky_f64;     // ✅ Cholesky-Banachiewicz algorithm
pub use eigh::eigh_f64;             // ✅ Jacobi eigenvalue algorithm
pub use lu_decompose;               // ✅ Re-exported from ops::linalg
pub use lu_solve;                   // ✅ Re-exported
pub use lu_det;                     // ✅ Re-exported
pub use lu_inverse;                 // ✅ Re-exported
pub use qr_decompose;               // ✅ Re-exported
pub use qr_least_squares;           // ✅ Re-exported
pub use svd_decompose;              // ✅ Re-exported
pub use svd_pinv;                   // ✅ Re-exported
pub use tridiagonal_solve_f64;      // ✅ Re-exported (Thomas algorithm)
```

### Files Added
- `crates/barracuda/src/linalg/cholesky.rs` — Full Cholesky with solve/det/log_det/inverse
- `crates/barracuda/src/linalg/eigh.rs` — Jacobi algorithm with eigenvector/sort/reconstruct

---

## Priority #2: Auto-Dispatch System — ✅ COMPLETE

### Solution Implemented

Centralized dispatch system with configurable per-operation thresholds:

```rust
use barracuda::dispatch::{dispatch_for, DispatchConfig, DispatchTarget};

// Global automatic dispatch
let target = dispatch_for("erf", input_size);  // Returns Cpu or Gpu

// Custom configuration
let config = DispatchConfig::new()
    .with_threshold("matmul", 4096)
    .force_cpu();  // or .force_gpu()

// Per-operation thresholds (empirically determined)
// - erf/erfc: 512
// - bessel: 1024
// - matmul: 4096
// - convolution: 8192
// - rbf_kernel: 200
```

### File Added
- `crates/barracuda/src/dispatch.rs` — Full dispatch module with:
  - `DispatchConfig` struct with per-op thresholds
  - GPU availability detection via wgpu
  - Global `OnceLock` singleton configuration
  - `dispatch_for()` and `dispatch_with_config()` functions

---

## Priority #3: EvaluationCache Persistence — ✅ COMPLETE

### Solution Implemented

Full serde-based persistence for warm-starting across runs:

```rust
use barracuda::optimize::EvaluationCache;

// Save/load to JSON
cache.save("cache.json")?;
let cache = EvaluationCache::load("cache.json")?;

// Graceful fallback if file doesn't exist
let cache = EvaluationCache::load_or_new("cache.json");

// Create from existing training data
let cache = EvaluationCache::from_training_data(x_data, y_data);

// Export for surrogate training
let (x_train, y_train) = cache.training_data();
```

### File Modified
- `crates/barracuda/src/optimize/eval_record.rs` — Added:
  - `#[derive(Serialize, Deserialize)]` on `EvaluationRecord` and `EvaluationCache`
  - `save()`, `load()`, `load_or_new()`, `from_training_data()` methods
  - `#[serde(skip)]` on `best_idx` with auto-recomputation on load

---

## Priority #4: Missing Scientific Functions — ✅ COMPLETE

### HIGH Priority — All Done

| Function | Module | Status | Implementation |
|----------|--------|--------|----------------|
| Generalized eigenvalue Ax = λBx | `linalg::gen_eigh` | ✅ DONE | Cholesky-based reduction to standard form |
| Incomplete gamma γ(a,x) | `special::gamma` | ✅ DONE | `regularized_gamma_p/q`, `lower_incomplete_gamma` |
| Newton-Raphson root-finding | `optimize::newton` | ✅ DONE | `newton()`, `newton_numerical()` |
| Secant method | `optimize::newton` | ✅ DONE | `secant()` |
| Brent's method | `optimize::brent` | ✅ DONE | `brent()`, `brent_minimize()` |
| Cubic spline interpolation | `interpolate::cubic_spline` | ✅ DONE | `CubicSpline::natural/clamped()` |
| Chi-squared distribution | `special::chi_squared` | ✅ DONE | `chi_squared_cdf/pdf/quantile/test()` |

### Files Added

- `crates/barracuda/src/special/gamma.rs` — Incomplete gamma functions (series + CF)
- `crates/barracuda/src/special/chi_squared.rs` — Full chi-squared distribution
- `crates/barracuda/src/optimize/newton.rs` — Newton-Raphson + Secant methods
- `crates/barracuda/src/optimize/brent.rs` — Brent root-finding + minimization
- `crates/barracuda/src/interpolate/cubic_spline.rs` — Full cubic spline with derivatives/integration
- `crates/barracuda/src/linalg/gen_eigh.rs` — Generalized eigenvalue via Cholesky reduction

### MEDIUM Priority — Remaining

| Function | Module | Use Case | Effort |
|----------|--------|----------|--------|
| Arbitrary-order Bessel Jₙ | `special::bessel_jn` | Nuclear wavefunctions | 1-2 days |
| Gauss-Legendre quadrature | `numerical::gauss_legendre` | High-accuracy integrals | 1-2 days |
| Conjugate gradient | `optimize::cg` | Large sparse systems | 2-3 days |

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

## Priority #6: LOO-CV for Surrogate Quality — ✅ COMPLETE

### Solution Implemented

```rust
use barracuda::surrogate::RBFSurrogate;

let surrogate = RBFSurrogate::train(&x_data, &y_data, RBFKernel::Gaussian, 0.0)?;

// Leave-one-out cross-validation
let rmse = surrogate.loo_cv_rmse()?;        // Overall quality metric
let errors = surrogate.loo_cv_errors()?;    // Per-point residuals

// Accessor methods
let n = surrogate.n_train();
let dim = surrogate.n_dim();
```

### File Modified
- `crates/barracuda/src/surrogate/rbf.rs` — Added:
  - `train_y` field to store training targets
  - `loo_cv_rmse()` — RMSE from LOO residuals
  - `loo_cv_errors()` — Per-point LOO errors
  - `compute_hat_diagonal()` — Hat matrix diagonal computation
  - `n_train()`, `n_dim()` accessor methods

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

### Phase A — Bridge & Polish ✅ COMPLETE

| Task | Priority | Status |
|------|----------|--------|
| f64 linalg bridges | 🔴 HIGH | ✅ DONE — cholesky, eigh, LU, QR, SVD, tridiagonal |
| Auto-dispatch system | 🔴 HIGH | ✅ DONE — `dispatch` module with per-op thresholds |
| EvaluationCache serialization | 🔴 HIGH | ✅ DONE — save/load/merge via serde_json |
| LOO-CV wiring | 🟡 MEDIUM | ✅ DONE — `loo_cv_rmse()`, `loo_cv_errors()` |

### Phase B — Scientific Depth ✅ COMPLETE

| Task | Priority | Status |
|------|----------|--------|
| Incomplete gamma + chi² | 🟡 MEDIUM | ✅ DONE — `special::gamma`, `special::chi_squared` |
| Newton-Raphson + Brent | 🟡 MEDIUM | ✅ DONE — `optimize::newton`, `optimize::brent` |
| Cubic spline | 🟡 MEDIUM | ✅ DONE — `interpolate::CubicSpline` |
| Generalized eigenvalue | 🟡 MEDIUM | ✅ DONE — `linalg::gen_eigh_f64` |

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
