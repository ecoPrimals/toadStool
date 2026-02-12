# BarraCUDA Science Gaps — Audit Response

**Date**: February 12, 2026
**From**: ToadStool Team
**Re**: BARRACUDA_SCIENCE_GAPS_FEB12_2026.md audit

---

## Executive Summary

The hotSpring team's gap analysis identified many items as "missing" that **already exist**.
This audit corrects the record and identifies the **actual gaps** that need work.

**Key Corrections**:
- Shader count: **391** (updated Feb 12)
- Many "missing" Rust wrappers already exist
- FFT suite is complete (fft_1d, fft_2d, fft_3d, ifft, rfft)
- RK4 ODE solver exists in `md/integrators/`
- All Bessel wrappers exist (j0, j1, i0, k0)

**Shader-First Architecture** (Feb 12, 2026):
- ALL math is now shader-first (WGSL primary)
- ToadStool dispatches to GPU/CPU based on hardware
- When fp64 GPUs available, seamless transition
- 18 special function shaders, 3 sampling shaders

---

## Corrections: What Already Exists

### ✅ Shaders (391 total)

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

### ✅ Special Functions (HIGH Priority Complete)

| Function | Shader | Rust | Priority | Status |
|----------|--------|------|----------|--------|
| Digamma ψ(x) | ✅ | ✅ | HIGH — Bayesian methods | **SHADER Feb 12** |
| Beta B(a,b) | ✅ | ✅ | HIGH — Statistics | **SHADER Feb 12** |
| Hermite Hₙ(x) | ✅ | ✅ | HIGH — HO wavefunctions | **SHADER Feb 12** |
| Legendre Pₙ(x) | ✅ | ✅ | HIGH — Angular momentum | **SHADER Feb 12** |
| Associated Legendre Pₙᵐ | ✅ | ✅ | HIGH — Full Y_lm | **SHADER Feb 12** |
| Normal CDF Φ(x) | ✅ | ✅ | HIGH — Statistics | **SHADER Feb 12** |
| Normal PDF φ(x) | ✅ | ✅ | HIGH — Statistics | **SHADER Feb 12** |
| Incomplete gamma γ(a,x) | ❌ | ❌ | MEDIUM — Chi² CDF | |
| Bessel Jₙ (arbitrary n) | ❌ | ❌ | MEDIUM — Nuclear | |
| Modified Bessel Iₙ, Kₙ | ❌ | ❌ | MEDIUM — Nuclear | |
| Spherical Bessel jₙ, yₙ | ❌ | ❌ | LOW — Scattering | |
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

### 🟢 Sampling/Stats (HIGH Priority Complete)

| Method | Priority | Status |
|--------|----------|--------|
| Normal distribution CDF | HIGH | ✅ **SHADER Feb 12** |
| Normal distribution PDF | HIGH | ✅ **SHADER Feb 12** |
| Sobol sequences | MEDIUM | ✅ DONE Feb 12 |
| Correlation matrix | MEDIUM | ✅ DONE Feb 12 |
| Covariance matrix | MEDIUM | ✅ DONE Feb 12 |
| xoshiro Rust wrapper | LOW | |
| Halton sequences | LOW | |
| Chi² test | MEDIUM | |
| KDE | LOW | |

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
5. ~~Band-diagonal + Crank-Nicolson~~ — ✅ **DONE**: tridiagonal.rs, crank_nicolson.rs
6. ~~WGSL Shaders for special functions~~ — ✅ **DONE Feb 12**: Shader-first architecture

---

## Shader-First Architecture (Feb 12, 2026)

BarraCUDA follows **shader-first** principles:
- WGSL shaders are the primary implementation
- ToadStool dispatches to GPU or CPU based on hardware
- All math must be runnable anywhere (GPU, CPU, NPU)

### Special Function Shaders (10 total)

| Shader | File | Operations |
|--------|------|------------|
| bessel_j0.wgsl | shaders/special/ | J₀(x) |
| bessel_j1.wgsl | shaders/special/ | J₁(x) |
| bessel_i0.wgsl | shaders/special/ | I₀(x) |
| bessel_k0.wgsl | shaders/special/ | K₀(x) |
| spherical_harmonics.wgsl | shaders/special/ | Yₗᵐ(θ,φ) |
| **hermite.wgsl** | shaders/special/ | Hₙ(x) — **NEW** |
| **legendre.wgsl** | shaders/special/ | Pₙ(x), Pₙᵐ(x) — **NEW** |
| **digamma.wgsl** | shaders/special/ | ψ(x) — **NEW** |
| **beta.wgsl** | shaders/special/ | B(a,b) — **NEW** |
| **norm_cdf.wgsl** | shaders/special/ | Φ(x), φ(x) — **NEW** |

---

## Summary

| Category | hotSpring Claimed Missing | Actually Missing |
|----------|--------------------------|------------------|
| Linalg wrappers | 6 | 0 |
| Special wrappers | 8 | 0 |
| Special shaders | 5 | 0 ✅ **All HIGH done** |
| FFT | 2 | 0 |
| ODE solver | 1 | 0 (RK4 exists) |
| **Total "bridge" items** | **17** | **0** |

**All HIGH priority hotSpring gaps resolved.** Remaining gaps are MEDIUM/LOW priority
(incomplete gamma, arbitrary-order Bessel, Airy, hypergeometric, etc.).
