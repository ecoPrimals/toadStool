# BarraCUDA Shader Inventory for hotSpring

**Date**: February 12, 2026  
**Total Shaders**: 400  
**Architecture**: SHADER-FIRST (WGSL primary, ToadStool dispatches)

---

## hotSpring Domain Requirements

### 1. Molecular Dynamics (MD)

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| **Force Kernels** | | | |
| Lennard-Jones | ✅ | `ops/md/forces/lennard_jones.rs` | Ready |
| Coulomb (electrostatic) | ✅ | `ops/md/forces/coulomb.rs` | Ready |
| Morse (bonded) | ✅ | `ops/md/forces/morse.rs` | Ready |
| Born-Mayer (repulsion) | ✅ | `ops/md/forces/born_mayer.rs` | Ready |
| Yukawa (screened) | ✅ | `ops/md/forces/yukawa.rs` | Ready |
| **Time Integration** | | | |
| Velocity Verlet | ✅ | `ops/md/integrators/velocity_verlet.rs` | Ready |
| RK4 | ✅ | `ops/md/integrators/rk4.rs` | Ready |
| RK45 adaptive | ✅ | `shaders/numerical/rk_stage.wgsl` | Ready |
| Laplacian (PDE) | ✅ | `ops/md/integrators/laplacian.rs` | Ready |
| **Periodic Boundaries** | | | |
| PBC distance | ✅ | `ops/md/pbc.rs` | Ready |
| Minimum image convention | ✅ | (in pbc.rs) | Ready |

### 2. Two-Temperature Model (TTM)

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| Crank-Nicolson PDE | ✅ | `shaders/pde/crank_nicolson.wgsl` | Ready |
| Tridiagonal solve | ✅ | `shaders/linalg/cyclic_reduction.wgsl` | Ready (parallel O(log n)) |
| Heat equation RHS | ✅ | `shaders/pde/crank_nicolson.wgsl` | Ready |
| Electron-phonon coupling | ✅ | (custom objective) | Use with optimizer |

### 3. Equation of State (EOS) Fitting

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| **Optimization** | | | |
| BFGS | ✅ | `shaders/optimizer/bfgs_update.wgsl` | Ready |
| Nelder-Mead | ✅ | `shaders/optimizer/simplex_ops.wgsl` | Ready |
| Numerical gradient | ✅ | `shaders/optimizer/batch_gradient.wgsl` | Ready |
| **Linear Algebra** | | | |
| LU decomposition | ✅ | `shaders/linalg/lu_decomp.wgsl` | Ready |
| QR decomposition | ✅ | `shaders/linalg/qr_decomp.wgsl` | Ready |
| SVD | ✅ | `shaders/linalg/svd.wgsl` | Ready |
| Cholesky | ✅ | `shaders/linalg/cholesky.wgsl` | Ready |
| Matrix inverse | ✅ | `shaders/linalg/inverse.wgsl` | Ready |
| Linear solve | ✅ | `shaders/linalg/linsolve.wgsl` | Ready |
| Eigendecomposition | ✅ | `shaders/linalg/eigh.wgsl` | Ready |

### 4. Special Functions (Physics)

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| **Polynomials** | | | |
| Hermite Hₙ(x) | ✅ | `shaders/special/hermite.wgsl` | Ready — HO wavefunctions |
| Legendre Pₙ(x) | ✅ | `shaders/special/legendre.wgsl` | Ready — Angular momentum |
| Associated Legendre Pₙᵐ | ✅ | `shaders/special/legendre.wgsl` | Ready — Full Yₗᵐ |
| Laguerre Lₙ^α(x) | ✅ | `shaders/special/laguerre.wgsl` | Ready — Radial wavefunctions |
| **Bessel Functions** | | | |
| J₀(x) | ✅ | `shaders/special/bessel_j0.wgsl` | Ready |
| J₁(x) | ✅ | `shaders/special/bessel_j1.wgsl` | Ready |
| I₀(x) | ✅ | `shaders/special/bessel_i0.wgsl` | Ready |
| K₀(x) | ✅ | `shaders/special/bessel_k0.wgsl` | Ready |
| **Other** | | | |
| Spherical harmonics Yₗᵐ | ✅ | `shaders/special/spherical_harmonics.wgsl` | Ready |
| Error function erf(x) | ✅ | `shaders/math/erf.wgsl` | Ready |
| Complementary erfc(x) | ✅ | `shaders/math/erfc.wgsl` | Ready |
| Log-gamma lgamma(x) | ✅ | `shaders/math/lgamma.wgsl` | Ready |
| Digamma ψ(x) | ✅ | `shaders/special/digamma.wgsl` | Ready |
| Beta B(a,b) | ✅ | `shaders/special/beta.wgsl` | Ready |

### 5. Statistics & Sampling

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| Normal CDF Φ(x) | ✅ | `shaders/special/norm_cdf.wgsl` | Ready |
| Normal PDF φ(x) | ✅ | `shaders/special/norm_cdf.wgsl` | Ready |
| Inverse Normal Φ⁻¹(p) | ✅ | `shaders/special/norm_ppf.wgsl` | Ready — Acklam algorithm |
| Pearson correlation | ✅ | `shaders/special/correlation.wgsl` | Ready |
| Covariance | ✅ | `shaders/special/covariance.wgsl` | Ready |
| Variance/Std | ✅ | `shaders/special/variance.wgsl` | Ready |
| Sobol sequences | ✅ | `shaders/sample/sobol.wgsl` | Ready — Quasi-random |
| Latin Hypercube | ✅ | `shaders/sample/lhs.wgsl` | Ready — Space-filling |
| Uniform random | ✅ | `shaders/sample/random_uniform.wgsl` | Ready |
| xoshiro128** PRNG | ✅ | `shaders/misc/prng_xoshiro.wgsl` | Ready — Monte Carlo |

### 6. FFT & Signal Processing

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| FFT 1D | ✅ | `ops/fft/fft_1d.rs` | Ready |
| FFT 2D | ✅ | `ops/fft/fft_2d.rs` | Ready |
| FFT 3D | ✅ | `ops/fft/fft_3d.rs` | Ready |
| IFFT | ✅ | `ops/fft/ifft_1d.rs` | Ready |
| RFFT (real) | ✅ | `ops/fft/rfft.rs` | Ready |
| **Complex Arithmetic** | | | |
| Complex add/sub/mul/div | ✅ | `ops/complex/*.rs` | Ready |
| Complex exp/log/pow | ✅ | `ops/complex/*.rs` | Ready |
| Complex sqrt/abs/conj | ✅ | `ops/complex/*.rs` | Ready |

### 7. Surrogate Modeling (RBF)

| Requirement | Shader | Location | Status |
|-------------|--------|----------|--------|
| RBF kernel evaluation | ✅ | `shaders/interpolation/rbf_kernel.wgsl` | Ready |
| Pairwise distance | ✅ | `shaders/misc/cdist.wgsl` | Ready |
| Leave-one-out CV | ✅ | `shaders/interpolation/loo_cv.wgsl` | Ready |

---

## Shader Categories Summary

| Category | Count | Purpose |
|----------|-------|---------|
| activation/ | 37 | Neural network activations |
| attention/ | 8 | Transformer attention mechanisms |
| audio/ | 9 | Signal processing (STFT, MFCC) |
| augmentation/ | 10 | Data augmentation |
| conv/ | 11 | Convolution operations |
| detection/ | 5 | Object detection (NMS, IoU) |
| dropout/ | 2 | Regularization |
| gnn/ | 6 | Graph neural networks |
| gradient/ | 1 | Gradient clipping |
| interpolation/ | 2 | RBF, LOO-CV |
| linalg/ | 14 | Linear algebra (LU, QR, SVD, Cholesky, etc.) |
| loss/ | 31 | Loss functions |
| math/ | 68 | Core math (trig, exp, matmul, etc.) |
| misc/ | 56 | Utilities (sort, unique, quantize) |
| norm/ | 27 | Normalization layers |
| numerical/ | 1 | RK45 ODE integration |
| optimizer/ | 16 | SGD, Adam, BFGS, Nelder-Mead |
| pde/ | 1 | Crank-Nicolson |
| pooling/ | 17 | Pooling operations |
| reduce/ | 14 | Reduction operations |
| rnn/ | 4 | Recurrent networks |
| sample/ | 3 | Sampling (Sobol, LHS, uniform) |
| special/ | 15 | Special functions (Bessel, Hermite, etc.) |
| tensor/ | 41 | Tensor manipulation |
| **Total** | **400** | |

---

## hotSpring Validation Checklist

### Molecular Dynamics Validation

```
[ ] Lennard-Jones: 2-body noble gas simulation
    - Expected: Energy conservation with Velocity Verlet
    - Validate: Compare with LAMMPS reference

[ ] Coulomb: Charged particle system
    - Expected: Correct 1/r² force law
    - Validate: Compare with analytical Madelung constant

[ ] PBC: Periodic box simulation
    - Expected: Minimum image convention correct
    - Validate: No artifacts at box boundaries

[ ] Multi-species: Mixed LJ + Coulomb
    - Expected: Proper force accumulation
    - Validate: NaCl crystal stability
```

### TTM Validation

```
[ ] Heat diffusion: 1D bar
    - Expected: Exponential decay to equilibrium
    - Validate: Analytical solution comparison

[ ] Crank-Nicolson stability: Large timesteps
    - Expected: Unconditionally stable
    - Validate: No oscillations

[ ] Electron-phonon: Two-temperature
    - Expected: Equilibration timescale correct
    - Validate: Literature values
```

### EOS Fitting Validation

```
[ ] Birch-Murnaghan: Pressure-volume curve
    - Expected: Fit matches experimental data
    - Validate: K₀, K₀' within uncertainty

[ ] BFGS convergence: Rosenbrock function
    - Expected: Converge to (1,1) in ~50 iterations
    - Validate: Gradient norm < 1e-6
```

---

## Ready for Validation

**All required shaders exist for hotSpring molecular dynamics validation.**

The hotSpring team can proceed with:
1. Lennard-Jones noble gas simulation
2. Coulomb charged particle system
3. TTM heat diffusion
4. EOS parameter fitting

ToadStool dispatches to GPU by default. For f64 precision, ToadStool routes to CPU.

---

**Last Updated**: February 12, 2026
