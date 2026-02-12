# BarraCUDA Science Gaps — Audit Response

**Date**: February 12, 2026
**From**: ToadStool Team
**Re**: BARRACUDA_SCIENCE_GAPS_FEB12_2026.md audit

---

## Executive Summary

The hotSpring team's gap analysis identified many items as "missing" that **already exist**.
This audit corrects the record and identifies the **actual gaps** that need work.

**Key Corrections**:
- Shader count: **414** (not 378)
- Many "missing" Rust wrappers already exist
- FFT suite is complete (fft_1d, fft_2d, fft_3d, ifft, rfft)
- RK4 ODE solver exists in `md/integrators/`
- All Bessel wrappers exist (j0, j1, i0, k0)

---

## Corrections: What Already Exists

### ✅ Shaders (414 total, not 378)

The hotSpring inventory undercounted by 36 shaders.

### ✅ Linalg Rust Wrappers (ALL exist)

| Item | hotSpring Said | Reality |
|------|---------------|---------|
| `cholesky` | ❌ Missing | ✅ `src/ops/linalg/cholesky.rs` |
| `eigh` | ❌ Missing | ✅ `src/ops/linalg/eigh.rs` |
| `triangular_solve` | ❌ Missing | ✅ `src/ops/linalg/triangular_solve.rs` |
| `inverse` | ❌ Missing | ✅ `src/ops/inverse_wgsl.rs` |
| `determinant` | ❌ Missing | ✅ `src/ops/determinant.rs` |
| `sparse_matvec` | ❌ Missing | ✅ `src/ops/sparse_matvec_wgsl.rs` |

### ✅ Special Functions Rust Wrappers (ALL exist)

| Item | hotSpring Said | Reality |
|------|---------------|---------|
| `erf` | ❌ Missing | ✅ `src/ops/erf_wgsl.rs` |
| `erfc` | ❌ Missing | ✅ `src/ops/erfc_wgsl.rs` |
| `lgamma` | ❌ Missing | ✅ `src/ops/lgamma_wgsl.rs` |
| `bessel_j0` | ❌ Missing | ✅ `src/ops/bessel_j0_wgsl.rs` |
| `bessel_j1` | ❌ Missing | ✅ `src/ops/bessel_j1_wgsl.rs` |
| `bessel_i0` | ❌ Missing | ✅ `src/ops/bessel_i0_wgsl.rs` |
| `bessel_k0` | ❌ Missing | ✅ `src/ops/bessel_k0_wgsl.rs` |
| `spherical_harmonics` | ❌ Missing | ✅ `src/ops/spherical_harmonics_wgsl.rs` |

### ✅ FFT Suite (COMPLETE)

| Item | hotSpring Said | Reality |
|------|---------------|---------|
| FFT radix-2 | ❌ Missing | ✅ `src/ops/fft/fft_1d.rs` + `fft_1d.wgsl` |
| FFT 2D | — | ✅ `src/ops/fft/fft_2d.rs` |
| FFT 3D | — | ✅ `src/ops/fft/fft_3d.rs` |
| IFFT | ❌ Missing | ✅ `src/ops/fft/ifft_1d.rs` |
| RFFT | — | ✅ `src/ops/fft/rfft.rs` |

### ✅ Numerical Methods (Partial)

| Item | hotSpring Said | Reality |
|------|---------------|---------|
| RK4 ODE solver | ❌ Missing | ✅ `src/ops/md/integrators/rk4.rs` |
| Velocity Verlet | — | ✅ `src/ops/md/integrators/velocity_verlet.rs` |
| Laplacian | — | ✅ `src/ops/md/integrators/laplacian.rs` |

### ✅ Other Wrappers

| Item | hotSpring Said | Reality |
|------|---------------|---------|
| `histc` | ❌ Missing | ✅ `src/ops/histc.rs` |
| `loo_cv` | ❌ Missing | ✅ `src/ops/loo_cv_wgsl.rs` |

---

## Actual Gaps — What's Really Missing

### 🔴 Special Functions (Actually Missing)

| Function | Shader | Rust | Priority | Status |
|----------|--------|------|----------|--------|
| Digamma ψ(x) | ❌ | ✅ | HIGH — Bayesian methods | **DONE Feb 12** |
| Beta B(a,b) | ❌ | ✅ | HIGH — Statistics | **DONE Feb 12** |
| Incomplete gamma γ(a,x) | ❌ | ❌ | MEDIUM — Chi² CDF | |
| Bessel Jₙ (arbitrary n) | ❌ | ❌ | MEDIUM — Nuclear | |
| Modified Bessel Iₙ, Kₙ | ❌ | ❌ | MEDIUM — Nuclear | |
| Spherical Bessel jₙ, yₙ | ❌ | ❌ | LOW — Scattering | |
| Hermite Hₙ(x) | ❌ | ✅ | HIGH — HO wavefunctions | **DONE Feb 12** |
| Legendre Pₙ(x) | ❌ | ✅ | HIGH — Angular momentum | **DONE Feb 12** |
| Associated Legendre Pₙᵐ | ❌ | ✅ | HIGH — Full Y_lm | **DONE Feb 12** |
| Airy Ai, Bi | ❌ | ❌ | LOW — Tunneling | |
| Hypergeometric ₁F₁, ₂F₁ | ❌ | ❌ | LOW — Coulomb | |
| Elliptic K, E | ❌ | ❌ | LOW — Deformed shapes | |

### 🔴 Linear Algebra (Actually Missing)

| Operation | Priority |
|-----------|----------|
| LU decomposition | MEDIUM |
| QR decomposition | MEDIUM |
| SVD | MEDIUM |
| Band-diagonal solver | HIGH — TTM PDE |
| Kronecker product wrapper | LOW |
| Generalized eigenvalue | LOW |

### 🔴 Numerical Methods (Actually Missing)

| Method | Priority |
|--------|----------|
| Simpson's rule | LOW |
| Gauss-Legendre quadrature | MEDIUM |
| Adaptive RK45 | MEDIUM |
| Crank-Nicolson PDE | HIGH — TTM |
| Brent root-finding | MEDIUM |
| Newton-Raphson | MEDIUM |
| Cubic spline | MEDIUM |
| Polynomial fitting | LOW |

### 🔴 Optimization (Actually Missing)

| Method | Priority |
|--------|----------|
| BFGS / L-BFGS | HIGH |
| Conjugate gradient | MEDIUM |
| Trust-region | LOW |
| Differential evolution | MEDIUM |
| Simulated annealing | LOW |

### 🔴 Sampling/Stats (Actually Missing)

| Method | Priority |
|--------|----------|
| xoshiro Rust wrapper | LOW |
| Sobol sequences | MEDIUM |
| Halton sequences | LOW |
| Normal distribution CDF | HIGH |
| Chi² test | MEDIUM |
| KDE | LOW |
| Correlation matrix | MEDIUM |
| Covariance matrix | MEDIUM |

### 🔴 Transforms (Actually Missing)

| Method | Priority |
|--------|----------|
| Box-Cox | LOW |
| Z-score normalization | LOW (trivial) |
| Min-max scaling | LOW (trivial) |
| Wavelet | LOW |

---

## Revised Priority List

Based on corrected gaps:

| # | Task | Priority | Status |
|---|------|----------|--------|
| 1 | ~~SparsitySampler hybrid eval~~ | ~~🔴 CRITICAL~~ | ✅ **DONE Feb 12** |
| 2 | ~~Bridge linalg shaders~~ | ~~HIGH~~ | ✅ Already done |
| 3 | ~~Bridge special shaders~~ | ~~HIGH~~ | ✅ **DONE Feb 12** — Re-exported GPU ops |
| 4 | ~~Hermite, Legendre polynomials~~ | ~~🔴 HIGH~~ | ✅ **DONE Feb 12** |
| 5 | ~~Digamma, Beta functions~~ | ~~🔴 HIGH~~ | ✅ **DONE Feb 12** |
| 6 | ~~Band-diagonal solver~~ | ~~🔴 HIGH~~ | ✅ **DONE Feb 12** — tridiagonal.rs |
| 7 | ~~Crank-Nicolson PDE~~ | ~~🔴 HIGH~~ | ✅ **DONE Feb 12** — crank_nicolson.rs |
| 8 | ~~BFGS optimizer~~ | ~~🟡 MEDIUM~~ | ✅ **DONE Feb 12** — bfgs.rs |
| 9 | ~~Normal CDF, correlation~~ | ~~🟡 MEDIUM~~ | ✅ **DONE Feb 12** — stats/ module |
| 10 | ~~New: FFT~~ | ~~MEDIUM~~ | ✅ Already complete |
| 11 | ~~New: RK4~~ | ~~MEDIUM~~ | ✅ Already exists |
| 12 | ~~Adaptive RK45~~ | ~~🟡 MEDIUM~~ | ✅ **DONE Feb 12** — rk45.rs |
| 13 | ~~Sobol sequences~~ | ~~🟡 MEDIUM~~ | ✅ **DONE Feb 12** — sobol.rs |
| 14 | ~~LU, QR, SVD~~ | ~~🟡 MEDIUM~~ | ✅ **DONE Feb 12** — lu.rs, qr.rs, svd.rs |

---

## Visibility Issue

The hotSpring team appears to be working from an older inventory or incomplete
`barracuda::` module visibility. Many Rust wrappers exist in `src/ops/` but may
not be re-exported through the main `barracuda::special::*` or `barracuda::linalg::*`
public API.

**Action**: Ensure all existing wrappers are properly exported in module `mod.rs`
files so they're discoverable via `use barracuda::*`.

---

## Recommended Immediate Actions

1. ~~Document existing wrappers~~ — ✅ **DONE**: `barracuda::special` expanded
2. ~~Re-export missing modules~~ — ✅ **DONE**: GPU ops re-exported (ErfGpu, BesselJ0Gpu, etc.)
3. ~~SparsitySampler hybrid eval~~ — ✅ **DONE**: `sparsity_sampler_gpu()` with cdist.wgsl
4. ~~Hermite/Legendre polynomials~~ — ✅ **DONE**: `hermite()`, `legendre()`, `assoc_legendre()`
5. **Band-diagonal + Crank-Nicolson** — Actual gap for TTM (next priority)

---

## Summary

| Category | hotSpring Claimed Missing | Actually Missing |
|----------|--------------------------|------------------|
| Linalg wrappers | 6 | 0 |
| Special wrappers | 8 | 0 |
| FFT | 2 | 0 |
| ODE solver | 1 | 0 (RK4 exists) |
| **Total "bridge" items** | **17** | **0** |

The "bridge gap" is largely a **visibility/documentation gap**, not an implementation gap.
The actual implementation gaps are in advanced special functions (Hermite, Legendre,
Digamma, Beta) and PDE solvers (band-diagonal, Crank-Nicolson).
