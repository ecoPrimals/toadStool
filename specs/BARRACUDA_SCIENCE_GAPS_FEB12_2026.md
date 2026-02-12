# BarraCUDA Universal Shader Math Library — Evolution Roadmap

**Date**: February 12, 2026
**From**: ecoPrimals Control Team (Eastgate) — hotSpring validation findings
**To**: ToadStool / BarraCUDA Team
**Priority**: HIGH — Foundational for all scientific workloads
**Depends On**: BARRACUDA_LIBRARY_VALIDATION_FEB12_2026.md (today's validation)
**Scope**: Build a robust, GPU-accelerated math library that eliminates the need
for scipy, numpy, LAPACK, or any external math dependency in Rust workloads

---

## Executive Summary

BarraCUDA already has **378 WGSL shaders** across 21 categories — an extraordinary
foundation. The ML/DL layer is mature. What's missing is the **scientific computing
middleware** that bridges raw shaders to real-world physics, engineering, and
optimization workloads.

Today's hotSpring L1+L2 validation exposed the exact gaps. Every function below
was either:
- **Written inline** in our L1/L2 binaries (needs extraction into library)
- **Delegated to nalgebra** (needs barracuda-native implementation)
- **Missing entirely** (needs new shader + Rust wrapper)

The goal: **any scientific workload should be expressible using only barracuda
modules** — no external math libraries, no inline reimplementations.

---

## Part 1: Current State — What's Solid

### 1.1 WGSL Shader Inventory (378 shaders, 21 categories)

| Category | Count | Maturity | Notes |
|----------|-------|----------|-------|
| `math/` | 68 | ✅ Solid | Trig, exp, log, pow, elementwise, matmul, erf |
| `linalg/` | 11 | ✅ Solid | Cholesky, eigh, linsolve, triangular, inverse, trace, diag |
| `special/` | 5 | 🟡 Partial | Bessel J₀/J₁/I₀/K₀, spherical harmonics. Missing: gamma, laguerre, hermite |
| `interpolation/` | 2 | 🟡 Partial | rbf_kernel, loo_cv. Missing: spline, polyfit |
| `misc/` | 56 | ✅ Solid | cdist, pdist, sort, topk, einsum, cross_product, etc. |
| `reduce/` | 14 | ✅ Solid | sum, mean, max, min, cumsum, variance, std |
| `activation/` | 37 | ✅ Solid | All standard ML activations |
| `attention/` | 8 | ✅ Solid | Flash attention, GQA, MHA, rotary |
| `norm/` | 27 | ✅ Solid | BatchNorm, LayerNorm, RMSNorm, GroupNorm |
| `loss/` | 31 | ✅ Solid | Full loss function suite |
| `conv/` | 11 | ✅ Solid | 1D/2D/3D, depthwise, deformable, dilated |
| `optimizer/` | 13 | ✅ Solid | Adam, SGD, LAMB, etc. |
| `pooling/` | 17 | ✅ Solid | Max/Avg/Adaptive/Global/RoI |
| `tensor/` | 41 | ✅ Solid | Reshape, permute, gather, scatter, pad |
| `audio/` | 9 | ✅ Solid | STFT, MFCC, Griffin-Lim |
| `gnn/` | 6 | ✅ Solid | GCN, GAT, GIN, SAGE |
| `rnn/` | 4 | ✅ Solid | LSTM, GRU, bi-LSTM |
| `detection/` | 5 | ✅ Solid | NMS, IoU, anchors |
| `augmentation/` | 10 | ✅ Solid | CutMix, MixUp, elastic, mosaic |
| `dropout/` | 2 | ✅ Solid | Standard + spatial |
| `gradient/` | 1 | ✅ Solid | clip_grad_norm |

### 1.2 Rust-Side Scientific Modules (Validated ✅ in hotSpring)

| Module | Functions | Status |
|--------|-----------|--------|
| `barracuda::linalg` | `solve_f64` | ✅ Working |
| `barracuda::numerical` | `gradient_1d`, `trapz`, `trapz_product` | ✅ Working |
| `barracuda::optimize` | `nelder_mead`, `bisect`, `multi_start_nelder_mead`, `EvaluationCache` | ✅ Working |
| `barracuda::sample` | `latin_hypercube`, `random_uniform`, `sparsity_sampler` | ✅ Working (accuracy tuning needed) |
| `barracuda::special` | `gamma`, `factorial`, `laguerre`, `laguerre_all`, `laguerre_simple` | ✅ Working |
| `barracuda::surrogate` | `RBFSurrogate` (train/predict), `RBFKernel` (6 variants), `AdaptiveConfig` | ✅ Working |

---

## Part 2: What's Missing — The Science Gap

### 2.1 Special Functions (Priority: HIGH)

The `special/` shader directory has Bessel functions and spherical harmonics.
The Rust-side has gamma, factorial, and Laguerre. But scientific computing
needs the full special function suite:

| Function | Shader | Rust | Status | Needed By |
|----------|--------|------|--------|-----------|
| Gamma function Γ(x) | ❌ `lgamma.wgsl` exists (log only) | ✅ `gamma.rs` | Bridge gap | HFB wavefunctions, statistical distributions |
| Digamma ψ(x) | ❌ | ❌ | **NEW** | Gradient of log-likelihood, Bayesian methods |
| Beta function B(a,b) | ❌ | ❌ | **NEW** | Bayesian statistics, F-distributions |
| Incomplete gamma γ(a,x) | ❌ | ❌ | **NEW** | Chi-squared distribution, Poisson CDF |
| Error function erf(x) | ✅ `erf.wgsl` | ❌ Rust wrapper | Bridge gap | Normal distribution CDF, diffusion |
| Complementary erfc(x) | ✅ `erfc.wgsl` | ❌ Rust wrapper | Bridge gap | Tail probabilities |
| Bessel J₀, J₁ | ✅ `bessel_j0.wgsl`, `bessel_j1.wgsl` | ❌ Rust wrapper | Bridge gap | Cylindrical physics, TTM hydro |
| Bessel Jₙ (arbitrary order) | ❌ | ❌ | **NEW** | Nuclear wavefunctions, acoustics |
| Modified Bessel I₀, K₀ | ✅ `bessel_i0.wgsl`, `bessel_k0.wgsl` | ❌ Rust wrapper | Bridge gap | Heat kernels, signal processing |
| Modified Bessel Iₙ, Kₙ | ❌ | ❌ | **NEW** | Nuclear density functionals |
| Spherical Bessel jₙ, yₙ | ❌ | ❌ | **NEW** | Scattering theory, L3 deformed HFB |
| Laguerre L_n^α(x) | ❌ | ✅ `laguerre.rs` | Add shader | HO wavefunctions (GPU batch) |
| Hermite H_n(x) | ❌ | ❌ | **NEW** | 1D HO wavefunctions, Gaussian quadrature |
| Legendre P_n(x) | ❌ | ❌ | **NEW** | Angular momentum, multipole expansion |
| Associated Legendre P_n^m | ❌ | ❌ | **NEW** | Spherical harmonics (full), L3 |
| Spherical harmonics Y_lm | ✅ `spherical_harmonics.wgsl` | ❌ Rust wrapper | Bridge gap | Deformed HFB, angular decomposition |
| Airy functions Ai, Bi | ❌ | ❌ | **NEW** | Quantum tunneling, WKB approximation |
| Hypergeometric ₁F₁, ₂F₁ | ❌ | ❌ | **NEW** | Coulomb wavefunctions, exact solutions |
| Elliptic integrals K, E | ❌ | ❌ | **NEW** | Deformed nuclear shapes, pendulum |

### 2.2 Linear Algebra (Priority: HIGH)

| Operation | Shader | Rust | Status | Needed By |
|-----------|--------|------|--------|-----------|
| LU decomposition | ❌ (linsolve uses Gaussian) | ❌ | **NEW** | General linear systems, determinant |
| QR decomposition | ❌ | ❌ | **NEW** | Least squares, eigenvalue prep |
| SVD | ❌ | ❌ | **NEW** | PCA, low-rank approximation, condition number |
| Symmetric eigendecomposition | ✅ `eigh.wgsl` (Jacobi) | ❌ (uses nalgebra) | Bridge gap | HFB, normal modes, PCA |
| Cholesky decomposition | ✅ `cholesky.wgsl` | ❌ Rust wrapper | Bridge gap | RBF training, positive-definite systems |
| Triangular solve | ✅ `triangular_solve.wgsl` | ❌ Rust wrapper | Bridge gap | Cholesky back-substitution |
| Matrix inverse | ✅ `inverse.wgsl` | ❌ Rust wrapper | Bridge gap | Small matrix inversion |
| Determinant | ✅ `determinant.wgsl` | ❌ Rust wrapper | Bridge gap | Jacobians |
| Band-diagonal solver | ❌ | ❌ | **NEW** | 1D PDE discretization (TTM, diffusion) |
| Sparse matrix × vector | ✅ `sparse_matvec.wgsl` | ❌ Rust wrapper | Bridge gap | Large sparse systems |
| Kronecker product | ✅ `kron.wgsl` | ❌ Rust wrapper | Bridge gap | Tensor product spaces |
| Generalized eigenvalue | ❌ | ❌ | **NEW** | Ax = λBx (HFB overlap matrix) |

### 2.3 Numerical Methods (Priority: HIGH)

| Method | Shader | Rust | Status | Needed By |
|--------|--------|------|--------|-----------|
| Trapezoidal integration | ❌ | ✅ `trapz` | Add shader | Batch integration on GPU |
| Simpson's rule | ❌ | ❌ | **NEW** | Higher-accuracy 1D integrals |
| Gauss-Legendre quadrature | ❌ | ❌ | **NEW** | High-accuracy integrals, FEM |
| Finite difference gradient | ❌ | ✅ `gradient_1d` | Add shader | Batch gradients on GPU |
| ODE solver (RK4) | ❌ | ❌ | **NEW** | TTM local, time evolution, dynamics |
| ODE solver (adaptive RK45) | ❌ | ❌ | **NEW** | Stiff ODEs, real-time simulation |
| PDE solver (Crank-Nicolson) | ❌ | ❌ | **NEW** | TTM hydro, diffusion equation |
| Root-finding (Brent) | ❌ | ✅ `bisect` | Upgrade | Faster convergence than bisection |
| Root-finding (Newton-Raphson) | ❌ | ❌ | **NEW** | Nonlinear equations, implicit methods |
| Interpolation (cubic spline) | ❌ | ❌ | **NEW** | Smooth data interpolation, EOS tables |
| Polynomial fitting | ❌ | ❌ | **NEW** | Curve fitting, trend analysis |
| FFT (radix-2) | ❌ | ❌ | **NEW** (STFT exists but not general FFT) | Spectral methods, convolution, DSF |
| Inverse FFT | ❌ | ❌ | **NEW** | Signal reconstruction, PDE spectral |

### 2.4 Optimization (Priority: HIGH)

| Method | Shader | Rust | Status | Needed By |
|--------|--------|------|--------|-----------|
| Nelder-Mead | ❌ | ✅ `nelder_mead` | Done | Derivative-free optimization |
| Bisection | ❌ | ✅ `bisect` | Done | 1D root-finding |
| Multi-start NM | ❌ | ✅ `multi_start_nelder_mead` | Done | Global optimization |
| BFGS / L-BFGS | ❌ | ❌ | **NEW** | Gradient-based optimization |
| Conjugate gradient | ❌ | ❌ | **NEW** | Large sparse linear systems |
| Trust-region | ❌ | ❌ | **NEW** | Constrained optimization |
| Differential evolution | ❌ | ❌ | **NEW** | Global optimization without gradients |
| Simulated annealing | ❌ | ❌ | **NEW** | Combinatorial optimization |
| Bayesian optimization | ❌ | ❌ | **NEW** | Expensive-function optimization |
| Constrained NM (box bounds) | ❌ | ✅ (bounds in NM) | Done | Bounded parameter search |

### 2.5 Sampling & Statistics (Priority: MEDIUM)

| Method | Shader | Rust | Status | Needed By |
|--------|--------|------|--------|-----------|
| Latin Hypercube | ❌ | ✅ `latin_hypercube` | Done | Initial exploration |
| Sobol sequences | ❌ | ❌ | **NEW** | Quasi-random sampling, sensitivity |
| Halton sequences | ❌ | ❌ | **NEW** | Low-discrepancy sampling |
| PRNG (xoshiro) | ✅ `prng_xoshiro.wgsl` | ❌ Rust wrapper | Bridge gap | GPU-parallel random numbers |
| Normal distribution | ❌ | ❌ | **NEW** | Statistical simulation, Monte Carlo |
| Chi-squared test | ❌ | ❌ | **NEW** | Goodness of fit |
| KDE (kernel density est.) | ❌ | ❌ | **NEW** | Distribution estimation |
| Histogram | ✅ `histc.wgsl` | ❌ Rust wrapper | Bridge gap | Distribution visualization |
| Correlation matrix | ❌ | ❌ | **NEW** | Parameter sensitivity |
| Covariance matrix | ❌ | ❌ | **NEW** | Multivariate statistics, GP |

### 2.6 Transforms (Priority: MEDIUM)

| Method | Shader | Rust | Status | Needed By |
|--------|--------|------|--------|-----------|
| Log-transform log(1+x) | ❌ | ❌ | **NEW** (trivial) | Surrogate learning |
| Box-Cox transform | ❌ | ❌ | **NEW** | Normalization of skewed data |
| Z-score normalization | ❌ | ❌ | **NEW** | Feature scaling |
| Min-max scaling | ❌ | ❌ | **NEW** | Feature scaling |
| Wavelet transform | ❌ | ❌ | **NEW** | Multi-scale analysis |

---

## Part 3: Architecture — The Bridge Pattern

The key architectural pattern is **Shader ↔ Rust Bridge**. Many shaders exist
but lack Rust wrappers. Many Rust functions exist but lack GPU-accelerated shaders.
The universal math library needs both sides.

### 3.1 The Three Execution Tiers

```
┌─────────────────────────────────────────────────┐
│              barracuda::math (Public API)         │
│    All functions available as Rust fn calls       │
│    Auto-dispatch to optimal tier                  │
├─────────────────────────────────────────────────┤
│  Tier 1: WGSL Shader (GPU)                       │
│    - Large batch operations (cdist, matmul)       │
│    - Element-wise on tensors (sin, exp, erf)      │
│    - Reduction operations (sum, mean, max)        │
│    Pro: Massive parallelism                       │
│    Con: Dispatch overhead for small inputs         │
├─────────────────────────────────────────────────┤
│  Tier 2: Rust CPU (f64)                           │
│    - Single-value operations (gamma, laguerre)    │
│    - Sequential algorithms (NM, bisect, ODE)      │
│    - Small matrix operations (eigh < 100×100)     │
│    Pro: Full f64 precision, zero dispatch cost     │
│    Con: No parallelism for large batches           │
├─────────────────────────────────────────────────┤
│  Tier 3: Auto-Dispatch (WorkloadHint)             │
│    - Small input → CPU fast path                  │
│    - Large batch → GPU shader                     │
│    - Threshold determined by profiling             │
│    Pro: Optimal for all sizes                      │
│    Con: Complexity                                 │
└─────────────────────────────────────────────────┘
```

### 3.2 The hotSpring Lesson: GPU Dispatch Overhead

During L1 validation, we discovered that using GPU shaders for **single-point**
surrogate predictions in the Nelder-Mead inner loop caused a **90× slowdown**.
The fix was a CPU-only fast path (`predict_single` on CPU).

**Design principle**: Every shader-backed function MUST have:
1. A Rust CPU fallback for small inputs
2. An auto-dispatch threshold
3. A `_cpu()` variant for explicit CPU execution

```rust
// Pattern for every math function:
pub fn erf(x: &[f64]) -> Vec<f64> {
    if x.len() < AUTO_DISPATCH_THRESHOLD {
        erf_cpu(x)           // Tier 2: direct CPU
    } else {
        erf_gpu(x)           // Tier 1: WGSL shader
    }
}

pub fn erf_cpu(x: &[f64]) -> Vec<f64> { ... }
pub fn erf_gpu(x: &[f64]) -> Vec<f64> { ... }
```

### 3.3 Precision Strategy

| Tier | Precision | When |
|------|-----------|------|
| WGSL shaders | f32 | Element-wise math, distance, convolution |
| WGSL shaders | f64 emulated | `matmul_fp64.wgsl` pattern — split into hi/lo |
| Rust CPU | f64 native | Linear algebra, root-finding, physics |

**Future (Titan V)**: Direct f64 on GPU via compute shader with f64 extensions.
BarraCUDA should have a `precision` config that selects f32/f64 shaders.

---

## Part 4: Phased Roadmap

### Phase 1: Bridge Existing Shaders (1-2 weeks)

Write Rust wrappers for shaders that already exist but lack a Rust API:

| Shader | Rust wrapper needed |
|--------|-------------------|
| `linalg/cholesky.wgsl` | `barracuda::linalg::cholesky` |
| `linalg/eigh.wgsl` | `barracuda::linalg::symmetric_eigen` |
| `linalg/triangular_solve.wgsl` | `barracuda::linalg::triangular_solve` |
| `linalg/inverse.wgsl` | `barracuda::linalg::inverse` |
| `linalg/determinant.wgsl` | `barracuda::linalg::determinant` |
| `math/erf.wgsl` | `barracuda::special::erf` |
| `math/erfc.wgsl` | `barracuda::special::erfc` |
| `math/lgamma.wgsl` | `barracuda::special::lgamma` |
| `special/bessel_j0.wgsl` | `barracuda::special::bessel_j0` |
| `special/bessel_j1.wgsl` | `barracuda::special::bessel_j1` |
| `special/bessel_i0.wgsl` | `barracuda::special::bessel_i0` |
| `special/bessel_k0.wgsl` | `barracuda::special::bessel_k0` |
| `special/spherical_harmonics.wgsl` | `barracuda::special::spherical_harmonics` |
| `interpolation/rbf_kernel.wgsl` | Already in `surrogate::RBFSurrogate` — verify dispatch |
| `interpolation/loo_cv.wgsl` | `barracuda::surrogate::loo_cv` |
| `misc/prng_xoshiro.wgsl` | `barracuda::random::xoshiro` |
| `misc/histc.wgsl` | `barracuda::stats::histogram` |
| `misc/sparse_matvec.wgsl` | `barracuda::linalg::sparse_matvec` |
| `misc/kron.wgsl` | `barracuda::linalg::kron` |

**Deliverable**: Every existing shader has a `pub fn` in `barracuda::*` with
CPU fallback and auto-dispatch.

### Phase 2: Critical New Functions (2-3 weeks)

New shaders + Rust implementations for hotSpring L3 and other scientific workloads:

| Function | Where | Why |
|----------|-------|-----|
| `special/laguerre.wgsl` | Batch HO wavefunctions | L2/L3: evaluate on entire radial grid |
| `special/hermite.wgsl` | 1D quantum mechanics | L3: deformed HFB |
| `special/legendre.wgsl` | Angular decomposition | L3: multipole expansion |
| `special/assoc_legendre.wgsl` | Full spherical harmonics | L3: deformed shapes |
| `special/bessel_jn.wgsl` | Arbitrary-order Bessel | Scattering, waveguides |
| `special/digamma.wgsl` | Log-gamma derivative | Bayesian, gradient of Γ |
| `special/beta.wgsl` | Beta function | Statistics |
| `numerical/rk4.wgsl` | ODE integration (batch) | TTM, dynamics on GPU |
| `numerical/simpson.wgsl` | Better integration | Higher accuracy than trapz |
| `numerical/fft_radix2.wgsl` | General FFT | Spectral methods, DSF |
| `numerical/ifft.wgsl` | Inverse FFT | Signal reconstruction |
| `linalg/lu.wgsl` | LU decomposition | General linear systems |
| `linalg/qr.wgsl` | QR decomposition | Least squares, eigenprep |
| `linalg/svd.wgsl` | SVD | PCA, condition number |
| `linalg/band_solve.wgsl` | Banded system solver | 1D PDE discretization |
| `optimize/bfgs.wgsl` | Gradient-based optimization | When gradients available |
| `sample/sobol.wgsl` | Quasi-random sequences | Sensitivity analysis |
| `stats/normal_cdf.wgsl` | Normal distribution CDF | Statistical tests |
| `stats/correlation.wgsl` | Correlation matrix | Parameter analysis |

### Phase 3: Advanced & Domain-Specific (ongoing)

| Function | Where | Why |
|----------|-------|-----|
| `special/airy.wgsl` | Airy functions | Quantum tunneling |
| `special/hypergeometric.wgsl` | Confluent hypergeometric | Coulomb wavefunctions |
| `special/elliptic.wgsl` | Elliptic integrals | Deformed nuclear shapes |
| `special/spherical_bessel.wgsl` | Spherical Bessel | Scattering theory |
| `numerical/ode_adaptive.wgsl` | Adaptive RK45 | Stiff ODEs |
| `numerical/pde_cn.wgsl` | Crank-Nicolson | Diffusion PDE |
| `numerical/gauss_legendre.wgsl` | GL quadrature | High-accuracy integrals |
| `numerical/cubic_spline.wgsl` | Cubic spline interpolation | EOS tables |
| `optimize/cg.wgsl` | Conjugate gradient | Large sparse systems |
| `optimize/trust_region.wgsl` | Trust region | Constrained optimization |
| `optimize/differential_evolution.wgsl` | DE | Global search |
| `sample/halton.wgsl` | Halton sequence | Low-discrepancy |
| `stats/kde.wgsl` | Kernel density estimation | Distribution fitting |
| `stats/covariance.wgsl` | Covariance matrix | Multivariate statistics |

---

## Part 5: SparsitySampler Accuracy Fix (Priority: CRITICAL)

Separate from the math library, but the #1 accuracy bottleneck from today's validation.

### Problem

The current SparsitySampler runs NM on the **surrogate** and evaluates only 1 true
point per solver. This gives ~20 true evaluations per iteration. Python's mystic
does ~200 true evaluations per round.

### Solution: Hybrid Evaluation Strategy

```rust
pub struct SparsitySamplerConfig {
    // ... existing fields ...

    /// NEW: Number of solvers running NM on TRUE objective (exploration)
    pub n_direct_solvers: usize,       // default: n_solvers / 2

    /// NEW: Evaluation budget for direct solvers
    pub direct_eval_budget: usize,     // default: 30

    /// NEW: Number of pure exploration LHS points per iteration
    pub n_explore_per_iter: usize,     // default: n_solvers / 4

    /// NEW: Early stopping patience (iterations without improvement)
    pub early_stopping_patience: Option<usize>,  // default: Some(5)
}
```

The iteration loop becomes:

```
Per iteration:
  1. Train surrogate on ALL evaluations
  2. Run n_surrogate_solvers × NM on surrogate → evaluate 1 true point each
  3. Run n_direct_solvers × NM on TRUE objective → evaluate direct_eval_budget each  ← NEW
  4. Sample n_explore_per_iter × LHS exploration points → evaluate true objective
  5. Add all to cache, check early stopping
```

**Expected impact**: With `n_direct_solvers=8, direct_eval_budget=30`, each iteration
adds ~260 true evaluations (vs current ~20). Over 5 iterations that's ~1300 total
(comparable to Python's ~1000). The direct NM solvers provide the aggressive
true-objective exploration that the Python mystic version achieves.

---

## Part 6: Testing Strategy

### Unit Tests per Function

Every new function needs:
1. **Known-value test**: Compare against published tables (A&S, DLMF)
2. **Edge case test**: Zero, infinity, NaN, negative, very large
3. **Precision test**: CPU (f64) vs shader (f32) — document acceptable epsilon
4. **Round-trip test**: Where applicable (FFT → IFFT = identity)

### Integration Tests against Python

```bash
# Pattern: generate reference data with scipy, validate barracuda matches
python3 -c "
import scipy.special as sp
import json
data = {'gamma': [(0.5, sp.gamma(0.5)), (1.0, 1.0), (5.0, 24.0), (0.1, sp.gamma(0.1))]}
json.dump(data, open('test_vectors.json', 'w'))
"

cargo test --test special_functions_vs_scipy
```

### Benchmark Tests

Every function with both CPU and GPU paths needs:
```rust
#[bench]
fn bench_erf_cpu_1000() { ... }

#[bench]
fn bench_erf_gpu_1000() { ... }

#[bench]
fn bench_erf_auto_1000() { ... }  // should pick CPU
fn bench_erf_auto_100000() { ... } // should pick GPU
```

This establishes the auto-dispatch threshold empirically.

---

## Part 7: Module Layout

```
barracuda/src/
├── special/
│   ├── mod.rs          ← re-exports all
│   ├── gamma.rs        ← Γ(x), lgamma, digamma, beta
│   ├── factorial.rs    ← n!, double factorial
│   ├── bessel.rs       ← J0, J1, Jn, I0, K0, In, Kn, spherical
│   ├── laguerre.rs     ← Laguerre L_n^α(x)
│   ├── hermite.rs      ← Hermite H_n(x)
│   ├── legendre.rs     ← Legendre P_n(x), P_n^m(x)
│   ├── erf.rs          ← erf, erfc, erfcx
│   ├── airy.rs         ← Ai, Bi
│   ├── elliptic.rs     ← K(m), E(m)
│   └── spherical_harmonics.rs ← Y_lm
│
├── linalg/
│   ├── mod.rs
│   ├── solve.rs        ← solve_f64 (existing)
│   ├── cholesky.rs     ← Cholesky with GPU shader
│   ├── eigen.rs        ← symmetric_eigen (wrap eigh.wgsl + nalgebra fallback)
│   ├── lu.rs           ← LU decomposition
│   ├── qr.rs           ← QR decomposition
│   ├── svd.rs          ← SVD
│   ├── sparse.rs       ← sparse matvec, sparse solve
│   ├── band.rs         ← banded system solver
│   └── triangular.rs   ← forward/back substitution
│
├── numerical/
│   ├── mod.rs
│   ├── integrate.rs    ← trapz, simpson, gauss_legendre (existing trapz)
│   ├── gradient.rs     ← gradient_1d (existing), gradient_nd
│   ├── ode.rs          ← rk4, rk45_adaptive, euler
│   ├── pde.rs          ← crank_nicolson_1d
│   ├── fft.rs          ← fft, ifft, rfft
│   ├── interpolate.rs  ← cubic_spline, polyfit, interp1d
│   └── roots.rs        ← brent, newton_raphson, secant
│
├── optimize/
│   ├── mod.rs
│   ├── nelder_mead.rs  ← (existing)
│   ├── bisect.rs       ← (existing)
│   ├── multi_start.rs  ← (existing)
│   ├── bfgs.rs         ← L-BFGS, BFGS
│   ├── cg.rs           ← conjugate gradient
│   ├── trust_region.rs
│   ├── de.rs           ← differential evolution
│   ├── eval_record.rs  ← (existing)
│   └── solver_state.rs ← (existing)
│
├── sample/
│   ├── mod.rs
│   ├── lhs.rs          ← (existing)
│   ├── sparsity.rs     ← (existing — needs hybrid eval)
│   ├── maximin.rs      ← (existing)
│   ├── sobol.rs        ← Sobol quasi-random
│   └── halton.rs       ← Halton sequence
│
├── stats/
│   ├── mod.rs          ← NEW module
│   ├── distributions.rs ← Normal, Chi2, F, t-distribution
│   ├── descriptive.rs  ← mean, var, std, skew, kurtosis
│   ├── correlation.rs  ← Pearson, Spearman, correlation matrix
│   ├── hypothesis.rs   ← Chi2 test, t-test, F-test
│   └── kde.rs          ← Kernel density estimation
│
├── surrogate/
│   ├── mod.rs
│   ├── rbf.rs          ← (existing)
│   ├── kernels.rs      ← (existing)
│   ├── adaptive.rs     ← (existing)
│   └── gaussian_process.rs ← GP regression (future)
│
└── transform/
    ├── mod.rs          ← NEW module
    ├── log.rs          ← log1p, expm1, box_cox
    ├── scale.rs        ← z_score, min_max, robust
    └── wavelet.rs      ← Haar, Daubechies (future)
```

---

## Part 8: Prioritized Action Items

| # | Task | Priority | Effort | Unlocks |
|---|------|----------|--------|---------|
| 1 | **SparsitySampler hybrid eval** | 🔴 CRITICAL | 2-3 days | L2 accuracy parity |
| 2 | **Bridge: linalg shaders → Rust** (cholesky, eigh, triangular, inverse) | 🔴 HIGH | 3-4 days | Remove nalgebra dependency from downstream |
| 3 | **Bridge: special shaders → Rust** (bessel, erf, spherical_harmonics) | 🔴 HIGH | 2-3 days | Complete special function suite |
| 4 | **New shaders: laguerre, hermite, legendre** | 🔴 HIGH | 2-3 days | GPU-batch wavefunction evaluation for L3 |
| 5 | **New: ODE solver** (rk4 + rk45_adaptive) | 🟡 MEDIUM | 3-4 days | TTM, dynamics workloads |
| 6 | **New: FFT** (radix-2 + inverse) | 🟡 MEDIUM | 3-4 days | Spectral methods, DSF, convolution |
| 7 | **New: BFGS optimizer** | 🟡 MEDIUM | 2-3 days | Gradient-based optimization |
| 8 | **New: stats module** (Normal CDF, chi2, correlation) | 🟡 MEDIUM | 2-3 days | Statistical validation |
| 9 | **New: LU, QR, SVD** | 🟡 MEDIUM | 4-5 days | General linear algebra |
| 10 | **New: Sobol/Halton** quasi-random | 🟢 LOW | 1-2 days | Better space-filling |
| 11 | **New: cubic spline** | 🟢 LOW | 1-2 days | EOS table interpolation |
| 12 | **LOO-CV for RBFSurrogate** | 🟢 LOW | 1 day | Surrogate quality metric |
| 13 | **Auto-dispatch thresholds** (benchmark suite) | 🟢 LOW | 2-3 days | CPU/GPU routing |
| 14 | **New: Airy, hypergeometric, elliptic** | 🟢 LOW | 3-4 days | L3, advanced physics |

### Critical Path (3-4 weeks)

```
Week 1: Items 1-3 (SparsitySampler fix + bridge existing shaders)
Week 2: Items 4-6 (new special functions + ODE + FFT)
Week 3: Items 7-9 (BFGS + stats + LA decompositions)
Week 4: Items 10-14 (quasi-random, spline, advanced special functions)
```

---

## Part 9: Reference Implementation Quality Standards

### What "done" looks like for each function:

```rust
/// Compute the error function erf(x).
///
/// Uses GPU shader for batches > 1024 elements, CPU for smaller.
///
/// # Precision
/// - CPU path: f64 (Abramowitz & Stegun 7.1.26, |ε| < 1.5e-7)
/// - GPU path: f32 (polynomial approximation, |ε| < 1e-5)
///
/// # Examples
/// ```
/// use barracuda::special::erf;
/// assert!((erf(&[0.0])[0] - 0.0).abs() < 1e-14);
/// assert!((erf(&[1.0])[0] - 0.8427007929).abs() < 1e-7);
/// ```
///
/// # References
/// - Abramowitz & Stegun, §7.1
/// - DLMF 7.2(i): <https://dlmf.nist.gov/7.2.i>
pub fn erf(x: &[f64]) -> Vec<f64> { ... }
pub fn erf_cpu(x: &[f64]) -> Vec<f64> { ... }
pub fn erf_gpu(x: &[f32]) -> Vec<f32> { ... }
```

Every function:
- Has CPU + GPU paths
- Documents precision for each path
- Links to reference (A&S, DLMF, or paper)
- Has unit tests with known values
- Has integration test vs scipy
- Has benchmark to determine dispatch threshold

---

## Summary

BarraCUDA has the GPU infrastructure. The 378 shaders are an extraordinary
foundation. What's missing is the **math middleware** — the Rust-side API that
makes these shaders usable by scientific workloads without any external dependencies.

The roadmap above turns BarraCUDA from a **tensor compute engine** into a
**self-contained scientific computing platform**. Once complete, any primal can
express `scipy.special.gamma(x)` as `barracuda::special::gamma(x)`, with automatic
CPU/GPU dispatch, full f64 precision, and zero Python dependency.

The SparsitySampler hybrid-eval fix is the #1 immediate priority for accuracy.
The shader bridging work (Phase 1) is the #1 immediate priority for completeness.
Together they make BarraCUDA the drop-in replacement for scipy + numpy + PyTorch
that the ecoPrimals ecosystem needs.
