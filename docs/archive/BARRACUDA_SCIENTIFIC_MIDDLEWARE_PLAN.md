# BarraCuda Scientific Computing Middleware Evolution Plan

**Date**: February 11, 2026  
**Status**: INVESTIGATION & PLANNING  
**Priority**: HIGH  
**Based On**: hotSpring L1/L2 validation results  
**Goal**: Extract 600 lines of proven scientific computing code into reusable library

---

## Executive Summary

hotSpring validated that BarraCuda can run real nuclear physics pipelines with:
- **14× speedup** on L1 (SEMF) with better accuracy than Python
- **1.7× speedup** on L2 (HFB+BCS) with throughput parity
- Dual-precision architecture (GPU f32 cdist + CPU f64 linear algebra) proven

**The problem**: ~600 lines of general-purpose scientific computing code are
duplicated between `nuclear_eos_l1.rs` and `nuclear_eos_l2.rs`. This middleware
layer (linear solvers, RBF surrogates, optimizers, numerical methods) belongs
in the BarraCuda library.

**The opportunity**: Extract proven code into library modules that every future
scientific primal can reuse. No research needed — code exists and validates
against Python control.

---

## Part 1: Source Code Inventory

### What Exists (in hotSpring control, needs extraction)

| File | Lines | Location | Contains |
|------|-------|----------|----------|
| `nuclear_eos_l1.rs` | 953 | hotSpring control | L1 SEMF pipeline + RBF + Nelder-Mead |
| `nuclear_eos_l2.rs` | 1542 | hotSpring control | L2 HFB+BCS pipeline + RBF + Nelder-Mead |

### Duplicated Code (extract once, use everywhere)

| Function/Struct | L1 Lines | L2 Lines | Duplication | Priority |
|-----------------|----------|----------|-------------|----------|
| `solve_f64` (Gauss-Jordan) | 450-503 (54) | 1130-1161 (32) | ✅ Identical | CRITICAL |
| `BarracudaRBFSurrogate` | 297-446 (150) | 1036-1128 (93) | ✅ Identical | CRITICAL |
| `nelder_mead` | 510-627 (118) | 1167-1250 (84) | ✅ Identical | CRITICAL |
| `bisect` | — | 860-874 (15) | L2 only | HIGH |
| `gradient_1d` | — | 834-844 (11) | L2 only | MEDIUM |
| `trapz` variants | — | 848-857 (10) | L2 only | MEDIUM |
| `gamma_fn` (Lanczos) | — | 886-931 (46) | L2 only | MEDIUM |
| `factorial` | — | 877-882 (6) | L2 only | MEDIUM |

**Total duplicated**: ~600 lines across both files

---

## Part 2: Proposed Module Structure

```
crates/barracuda/src/
├── linalg/                  # Linear algebra (CPU f64 + optional GPU f64)
│   ├── mod.rs               # Re-exports
│   ├── solve.rs             # Gauss-Jordan, LU decomposition
│   ├── eigen.rs             # nalgebra SymmetricEigen wrapper + eigh()
│   ├── cholesky.rs          # CPU f64 Cholesky (WGSL f32 exists)
│   └── triangular.rs        # Forward/backward substitution
│
├── surrogate/               # Surrogate modeling
│   ├── mod.rs
│   ├── rbf.rs               # RBFSurrogate struct (train + predict)
│   ├── kernels.rs           # Kernel functions (TPS, Gaussian, Multiquadric, etc.)
│   └── validation.rs        # LOO-CV, cross-validation utilities
│
├── optimize/                # Optimization algorithms
│   ├── mod.rs
│   ├── nelder_mead.rs       # Bounded Nelder-Mead simplex
│   ├── bisect.rs            # Bisection root-finder
│   ├── brentq.rs            # Brent's method (future)
│   ├── latin_hypercube.rs   # Space-filling sampling
│   ├── sparsity_sampler.rs  # Maximin distance sampling (port from mystic)
│   └── multi_start.rs       # Parallel multi-start optimization
│
├── numerical/               # Numerical methods
│   ├── mod.rs
│   ├── gradient.rs          # Finite-difference gradients
│   ├── integrate.rs         # Trapezoidal, Simpson's, Romberg
│   └── differentiate.rs     # Numerical derivatives
│
├── special/                 # Special functions
│   ├── mod.rs
│   ├── gamma.rs             # Gamma function (Lanczos approximation)
│   ├── factorial.rs         # Factorial and binomial
│   ├── laguerre.rs          # Generalized Laguerre polynomials
│   └── wrappers.rs          # f64 wrappers for existing WGSL (erf, lgamma)
│
├── ops/                     # Existing (445 ops, untouched)
├── shaders/                 # Existing (414 shaders, untouched)
└── ...                      # Existing modules unchanged
```

---

## Part 3: Detailed API Design

### 3.1 `barracuda::linalg` Module

#### Core Linear Solvers

```rust
/// Solve Ax = b using Gauss-Jordan elimination with partial pivoting
/// 
/// # Arguments
/// * `a` - Coefficient matrix (row-major, n×n)
/// * `b` - Right-hand side vector (length n)
/// * `n` - Matrix dimension
/// 
/// # Returns
/// Solution vector x, or error if singular
/// 
/// # Precision
/// f64 on CPU. For f32 GPU version, use `linsolve.wgsl` shader.
/// 
/// # Example
/// ```
/// use barracuda::linalg::solve_f64;
/// let a = vec![2.0, 1.0, 1.0, 3.0]; // 2×2 matrix
/// let b = vec![5.0, 8.0];
/// let x = solve_f64(&a, &b, 2)?;
/// assert!((x[0] - 1.0).abs() < 1e-10);
/// assert!((x[1] - 3.0).abs() < 1e-10);
/// ```
pub fn solve_f64(a: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>, BarracudaError>;

/// Eigenvalue decomposition of symmetric matrix (nalgebra wrapper)
/// 
/// # Arguments
/// * `a` - Symmetric matrix (nalgebra DMatrix)
/// 
/// # Returns
/// (eigenvalues, eigenvectors) sorted by ascending eigenvalue
/// 
/// # Notes
/// - Matches numpy.linalg.eigh() behavior
/// - Uses nalgebra::SymmetricEigen under the hood
/// - CPU only (GPU Jacobi iteration TBD for large matrices)
/// 
/// # Example
/// ```
/// use barracuda::linalg::eigh;
/// use nalgebra::DMatrix;
/// 
/// let a = DMatrix::from_row_slice(3, 3, &[
///     4.0, 1.0, 0.0,
///     1.0, 4.0, 1.0,
///     0.0, 1.0, 4.0,
/// ]);
/// let (eigenvalues, eigenvectors) = eigh(a)?;
/// ```
pub fn eigh(a: DMatrix<f64>) -> Result<(DVector<f64>, DMatrix<f64>), BarracudaError>;

/// Cholesky decomposition A = L L^T
/// 
/// # Returns
/// Lower triangular matrix L, or error if not positive definite
pub fn cholesky_f64(a: &[f64], n: usize) -> Result<Vec<f64>, BarracudaError>;

/// Forward substitution: solve Lx = b where L is lower triangular
pub fn forward_substitution(l: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>, BarracudaError>;

/// Backward substitution: solve Ux = b where U is upper triangular
pub fn backward_substitution(u: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>, BarracudaError>;
```

#### Design Notes

- All functions return `Result<T, BarracudaError>` for error handling
- f64 precision by default (scientific computing standard)
- f32 GPU variants via WGSL shaders (separate API, already exist)
- Leverage `nalgebra` for complex operations (already a dependency)

---

### 3.2 `barracuda::surrogate` Module

#### RBF Surrogate (Core)

```rust
use std::sync::Arc;
use crate::device::WgpuDevice;

/// Radial Basis Function (RBF) surrogate model
/// 
/// Uses dual-precision architecture:
/// - GPU (f32): cdist shader for pairwise distances (O(n²) bottleneck)
/// - CPU (f64): Kernel evaluation + linear solve for weights
/// 
/// Matches scipy.interpolate.RBFInterpolator behavior.
pub struct RBFSurrogate {
    train_x: Vec<f64>,       // [n_train × n_dim] flattened
    weights: Vec<f64>,       // [n_train] RBF weights
    poly_coeffs: Vec<f64>,   // [n_dim + 1] polynomial tail
    n_train: usize,
    n_dim: usize,
    kernel: RBFKernel,       // TPS, Gaussian, Multiquadric, etc.
    device: Arc<WgpuDevice>, // For GPU cdist
}

impl RBFSurrogate {
    /// Train RBF surrogate with given kernel
    /// 
    /// # Arguments
    /// * `x_data` - Training inputs [[x1, x2, ...], ...] (n_train × n_dim)
    /// * `y_data` - Training outputs [y1, y2, ...] (n_train)
    /// * `kernel` - RBF kernel type (TPS, Gaussian, etc.)
    /// * `smoothing` - Regularization parameter (λ in (A + λI)⁻¹)
    /// * `device` - GPU device for cdist shader
    /// 
    /// # Returns
    /// Trained surrogate model
    /// 
    /// # Algorithm
    /// 1. GPU: Compute pairwise distances D = cdist(x_data, x_data) in f32
    /// 2. CPU: Promote D to f64, apply kernel φ(D), add polynomial terms
    /// 3. CPU: Solve (Φ + λI) w = y for weights w using Gauss-Jordan
    /// 
    /// # Example
    /// ```
    /// use barracuda::surrogate::{RBFSurrogate, RBFKernel};
    /// 
    /// let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
    /// let y_train = vec![0.0, 1.0, 4.0];
    /// 
    /// let surrogate = RBFSurrogate::train(
    ///     &x_train,
    ///     &y_train,
    ///     RBFKernel::ThinPlateSpline,
    ///     1e-12,  // smoothing
    ///     device,
    /// )?;
    /// ```
    pub fn train(
        x_data: &[Vec<f64>],
        y_data: &[f64],
        kernel: RBFKernel,
        smoothing: f64,
        device: Arc<WgpuDevice>,
    ) -> Result<Self, BarracudaError>;

    /// Predict at new points
    /// 
    /// # Arguments
    /// * `x_eval` - Evaluation points [[x1, x2, ...], ...] (n_eval × n_dim)
    /// 
    /// # Returns
    /// Predictions [y1, y2, ...] (n_eval)
    /// 
    /// # Algorithm
    /// 1. GPU: Compute distances D = cdist(x_eval, x_train) in f32
    /// 2. CPU: Promote D to f64, apply kernel φ(D)
    /// 3. CPU: Compute y = Φ·w + P·c (RBF + polynomial)
    /// 
    /// # Example
    /// ```
    /// let x_eval = vec![vec![0.5], vec![1.5]];
    /// let y_pred = surrogate.predict(&x_eval)?;
    /// ```
    pub fn predict(&self, x_eval: &[Vec<f64>]) -> Result<Vec<f64>, BarracudaError>;

    /// Leave-one-out cross-validation error
    /// 
    /// # Returns
    /// LOOCV RMSE without retraining (uses Sherman-Morrison formula)
    pub fn loocv_error(&self) -> Result<f64, BarracudaError>;
}

/// RBF kernel functions
#[derive(Debug, Clone, Copy)]
pub enum RBFKernel {
    /// Thin-plate spline: φ(r²) = 0.5·r²·ln(r²)
    /// - Default for scattered data interpolation
    /// - Matches Diaw et al. (2024) EOS surrogate
    ThinPlateSpline,

    /// Gaussian: φ(r) = exp(-ε²r²)
    /// - Parameter ε controls locality
    /// - Good for smooth functions
    Gaussian { epsilon: f64 },

    /// Multiquadric: φ(r) = √(1 + ε²r²)
    /// - Parameter ε controls shape
    /// - Good for scattered data
    Multiquadric { epsilon: f64 },

    /// Inverse multiquadric: φ(r) = 1/√(1 + ε²r²)
    /// - Smoother than multiquadric
    /// - Good for noisy data
    InverseMultiquadric { epsilon: f64 },

    /// Cubic: φ(r) = r³
    /// - Simple, no parameters
    /// - Good for 1D/2D interpolation
    Cubic,

    /// Quintic: φ(r) = r⁵
    /// - Higher-order smoothness
    Quintic,
}

impl RBFKernel {
    /// Evaluate kernel at distance r
    pub fn eval(&self, r: f64) -> f64;
    
    /// Evaluate kernel for array of distances (vectorized)
    pub fn eval_batch(&self, distances: &[f64]) -> Vec<f64>;
}
```

#### Design Rationale

**Dual-Precision Architecture**:
- Proven in L1/L2 validation
- GPU f32 cdist is 14× faster than CPU, accuracy loss is negligible for distances
- CPU f64 linear algebra maintains scientific precision where it matters
- Falls back gracefully if GPU unavailable (CPU cdist via nalgebra)

**Kernel Flexibility**:
- TPS is default (matches hotSpring validation)
- Gaussian/Multiquadric for different data characteristics
- Easy to add custom kernels

**API Matches scipy.interpolate**:
- Familiar to scientific Python users
- Validated against Python control
- Drop-in replacement for migration

---

### 3.3 `barracuda::optimize` Module

#### Nelder-Mead Optimizer

```rust
/// Bounded Nelder-Mead simplex optimizer
/// 
/// # Arguments
/// * `f` - Objective function (minimize)
/// * `x0` - Initial guess
/// * `bounds` - Parameter bounds [(min, max), ...]
/// * `max_iter` - Maximum iterations
/// * `tol` - Convergence tolerance (simplex diameter)
/// 
/// # Returns
/// (x_best, f_best, n_evals)
/// 
/// # Algorithm
/// Standard Nelder-Mead with reflections constrained to bounds.
/// Adaptive parameters: α=1.0, γ=2.0, ρ=0.5, σ=0.5
/// 
/// # Example
/// ```
/// use barracuda::optimize::nelder_mead;
/// 
/// let rosenbrock = |x: &[f64]| {
///     let (x0, x1) = (x[0], x[1]);
///     (1.0 - x0).powi(2) + 100.0 * (x1 - x0.powi(2)).powi(2)
/// };
/// 
/// let x0 = vec![0.0, 0.0];
/// let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
/// let (x_best, f_best, n_evals) = nelder_mead(
///     rosenbrock,
///     &x0,
///     &bounds,
///     1000,   // max_iter
///     1e-8,   // tol
/// )?;
/// 
/// assert!((x_best[0] - 1.0).abs() < 1e-4);
/// assert!((x_best[1] - 1.0).abs() < 1e-4);
/// ```
pub fn nelder_mead<F>(
    f: F,
    x0: &[f64],
    bounds: &[(f64, f64)],
    max_iter: usize,
    tol: f64,
) -> Result<(Vec<f64>, f64, usize), BarracudaError>
where
    F: Fn(&[f64]) -> f64;
```

#### Root-Finding

```rust
/// Bisection root-finder
/// 
/// Finds x in [a, b] where f(x) = 0.
/// Requires f(a) and f(b) have opposite signs.
/// 
/// # Example
/// ```
/// use barracuda::optimize::bisect;
/// 
/// let f = |x: f64| x.powi(2) - 2.0;  // Find √2
/// let root = bisect(f, 0.0, 2.0, 1e-10, 100)?;
/// assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
/// ```
pub fn bisect<F>(
    f: F,
    a: f64,
    b: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, BarracudaError>
where
    F: Fn(f64) -> f64;

/// Brent's method (future — faster than bisection)
pub fn brentq<F>(
    f: F,
    a: f64,
    b: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, BarracudaError>
where
    F: Fn(f64) -> f64;
```

#### Smart Sampling (THE PRIZE)

```rust
/// Latin Hypercube Sampling (space-filling)
/// 
/// Generates n_samples points in d-dimensional hypercube with
/// near-uniform coverage. Better than random for initial exploration.
/// 
/// # Arguments
/// * `bounds` - Parameter bounds [(min, max), ...]
/// * `n_samples` - Number of samples to generate
/// * `rng` - Optional RNG seed for reproducibility
/// 
/// # Returns
/// Vec of sample points, each Vec<f64> of length bounds.len()
/// 
/// # Example
/// ```
/// use barracuda::optimize::latin_hypercube;
/// 
/// let bounds = vec![(-1.0, 1.0), (-1.0, 1.0)];
/// let samples = latin_hypercube(&bounds, 100, None)?;
/// 
/// // Verify space-filling property
/// assert_eq!(samples.len(), 100);
/// assert_eq!(samples[0].len(), 2);
/// ```
pub fn latin_hypercube(
    bounds: &[(f64, f64)],
    n_samples: usize,
    rng: Option<u64>,
) -> Result<Vec<Vec<f64>>, BarracudaError>;

/// Sparsity-based sampling (maximin distance)
/// 
/// Port of mystic.SparsitySampler. Generates samples that maximize
/// minimum pairwise distance to existing points (gap-filling).
/// 
/// **This is the accuracy multiplier**: Python L2 reached χ²=1.93
/// with 3008 evals using this. BarraCuda's random sampling hit
/// χ²=87 with 1009 evals. Smart sampling + faster throughput = WIN.
/// 
/// # Arguments
/// * `bounds` - Parameter bounds
/// * `n_samples` - Number of samples to generate
/// * `existing_points` - Already-sampled points to avoid
/// * `min_distance` - Minimum acceptable distance to existing points
/// 
/// # Returns
/// Vec of sample points filling gaps in existing coverage
/// 
/// # Algorithm
/// 1. Candidate generation: Random samples in bounds
/// 2. Distance computation: cdist(candidates, existing) via GPU
/// 3. Selection: Pick candidate with maximum min-distance to existing
/// 4. Repeat until n_samples generated
/// 
/// # Example
/// ```
/// use barracuda::optimize::sparsity_sampler;
/// 
/// let bounds = vec![(-1.0, 1.0), (-1.0, 1.0)];
/// let existing = vec![vec![0.0, 0.0]];  // Already sampled center
/// 
/// let new_samples = sparsity_sampler(
///     &bounds,
///     10,              // Generate 10 more
///     &existing,
///     0.1,             // Min distance 0.1 from existing
///     device,
/// )?;
/// 
/// // New samples avoid center region
/// ```
pub fn sparsity_sampler(
    bounds: &[(f64, f64)],
    n_samples: usize,
    existing_points: &[Vec<f64>],
    min_distance: f64,
    device: Arc<WgpuDevice>,
) -> Result<Vec<Vec<f64>>, BarracudaError>;

/// Multi-start Nelder-Mead with parallel execution
/// 
/// Runs Nelder-Mead from N different starting points (generated
/// via latin_hypercube) in parallel using rayon. Returns global best.
/// 
/// # Example
/// ```
/// use barracuda::optimize::multi_start_nelder_mead;
/// 
/// let f = |x: &[f64]| /* complex objective */;
/// let bounds = vec![(-10.0, 10.0); 5];
/// 
/// let (x_best, f_best) = multi_start_nelder_mead(
///     f,
///     &bounds,
///     20,      // 20 parallel starts
///     1000,    // max_iter per start
///     1e-6,    // tol
/// )?;
/// ```
pub fn multi_start_nelder_mead<F>(
    f: F,
    bounds: &[(f64, f64)],
    n_starts: usize,
    max_iter: usize,
    tol: f64,
) -> Result<(Vec<f64>, f64), BarracudaError>
where
    F: Fn(&[f64]) -> f64 + Sync;
```

---

### 3.4 `barracuda::numerical` Module

```rust
/// Finite-difference numerical gradient (1D, 3-point stencil)
/// 
/// Matches numpy.gradient() behavior.
/// 
/// # Arguments
/// * `f` - Function values [f(x0), f(x1), ...]
/// * `dx` - Grid spacing (uniform)
/// 
/// # Returns
/// Gradient [df/dx(x0), df/dx(x1), ...]
/// 
/// # Stencil
/// - Interior: (f[i+1] - f[i-1]) / (2·dx)
/// - Boundaries: Forward/backward 1st-order
pub fn gradient_1d(f: &[f64], dx: f64) -> Vec<f64>;

/// Trapezoidal integration
/// 
/// ∫ y(x) dx ≈ Σ (y[i] + y[i+1])/2 · (x[i+1] - x[i])
/// 
/// # Arguments
/// * `y` - Function values
/// * `x` - Grid points (need not be uniform)
/// 
/// # Returns
/// Integral value
pub fn trapz(y: &[f64], x: &[f64]) -> Result<f64, BarracudaError>;

/// Weighted trapezoidal product integral
/// 
/// ∫ f(r) · g1(r) · g2(r) · w(r) dr
/// 
/// Used extensively in HFB matrix element calculations.
/// 
/// # Arguments
/// * `f` - First function
/// * `g1` - Second function
/// * `g2` - Third function
/// * `x` - Grid points
/// * `weights` - Quadrature weights
pub fn trapz_product(
    f: &[f64],
    g1: &[f64],
    g2: &[f64],
    x: &[f64],
    weights: &[f64],
) -> Result<f64, BarracudaError>;
```

---

### 3.5 `barracuda::special` Module

```rust
/// Gamma function Γ(x) via Lanczos approximation
/// 
/// Handles:
/// - Positive half-integers exactly (n+1/2 → √π · n!! / 2^n)
/// - General values via 9-term Lanczos series
/// - Negative values via reflection formula
/// 
/// Matches scipy.special.gamma() to f64 precision.
/// 
/// # Example
/// ```
/// use barracuda::special::gamma;
/// 
/// assert!((gamma(1.0) - 1.0).abs() < 1e-15);       // Γ(1) = 1
/// assert!((gamma(5.0) - 24.0).abs() < 1e-12);      // Γ(5) = 4!
/// assert!((gamma(0.5) - std::f64::consts::PI.sqrt()).abs() < 1e-12);  // Γ(1/2) = √π
/// ```
pub fn gamma(x: f64) -> f64;

/// Factorial n!
/// 
/// Exact for n ≤ 20, uses Stirling for n > 20.
pub fn factorial(n: usize) -> f64;

/// Generalized Laguerre polynomial L_n^α(x)
/// 
/// Used in harmonic oscillator radial wavefunctions.
/// 
/// # Arguments
/// * `n` - Polynomial order
/// * `alpha` - Generalization parameter
/// * `x` - Evaluation point
/// 
/// # Returns
/// L_n^α(x)
/// 
/// # Recursion
/// L_0^α(x) = 1
/// L_1^α(x) = 1 + α - x
/// (n+1)L_{n+1}^α = (2n + 1 + α - x)L_n^α - (n + α)L_{n-1}^α
pub fn laguerre(n: usize, alpha: f64, x: f64) -> f64;
```

---

## Part 4: Extraction Plan (3-Week Sprint)

### Week 1: Core Infrastructure (CRITICAL)

| Day | Task | Deliverable | Validation |
|-----|------|-------------|------------|
| 1 | Create module structure | 5 new `mod.rs` files | `cargo check` passes |
| 2 | Extract `solve_f64` → `linalg::solve_f64` | Working function + tests | Match numpy.linalg.solve |
| 3 | Extract `BarracudaRBFSurrogate` → `surrogate::RBFSurrogate` | Struct + train + predict | Match scipy.interpolate.RBFInterpolator |
| 4 | Extract `nelder_mead` → `optimize::nelder_mead` | Optimizer + tests | Rosenbrock, Rastrigin benchmarks |
| 5 | Integration: Update L1/L2 binaries to use library | Binaries use new modules | Same results as inline code |

**Success criteria**: L1/L2 binaries run with library modules, produce identical results.

### Week 2: Optimization & Sampling (HIGH)

| Day | Task | Deliverable | Validation |
|-----|------|-------------|------------|
| 6 | Extract `bisect` → `optimize::bisect` | Root-finder + tests | Find √2, roots of polynomials |
| 7 | Implement `latin_hypercube` | Sampler + tests | Space-filling properties |
| 8 | Port mystic SparsitySampler | `sparsity_sampler` + tests | Maximin distance verification |
| 9 | Implement `multi_start_nelder_mead` | Parallel optimizer | Rastrigin (many local minima) |
| 10 | Benchmark: SparsitySampler vs random | Convergence comparison | Target: 3× fewer evals for same χ² |

**Success criteria**: `sparsity_sampler` demonstrates faster convergence than random on test problems.

### Week 3: Numerical Methods & Special Functions (MEDIUM)

| Day | Task | Deliverable | Validation |
|-----|------|-------------|------------|
| 11 | Extract numerical methods | `gradient_1d`, `trapz`, etc. | Match numpy.gradient, numpy.trapz |
| 12 | Extract special functions | `gamma`, `factorial`, `laguerre` | Match scipy.special |
| 13 | Wrap nalgebra SymmetricEigen | `linalg::eigh` | Match numpy.linalg.eigh |
| 14 | Add RBF kernel variants | Gaussian, Multiquadric, etc. | Interpolation tests |
| 15 | Documentation + examples | README, API docs, tutorials | Review-ready |

**Success criteria**: All extracted modules documented and tested against Python equivalents.

---

## Part 5: Success Metrics

### Code Quality

| Metric | Target | Validation |
|--------|--------|------------|
| Test coverage | >90% | Each function has unit test |
| Documentation | 100% | Every pub fn has doc comment + example |
| Benchmarks | Python parity | Match scipy/numpy within 1% |
| Zero-copy | Where possible | Profile memory allocations |

### Scientific Accuracy

| Function | Python Equivalent | Tolerance |
|----------|------------------|-----------|
| `solve_f64` | `numpy.linalg.solve` | <1e-12 relative |
| `RBFSurrogate.predict` | `scipy.interpolate.RBFInterpolator` | <1e-10 |
| `nelder_mead` | `scipy.optimize.fmin` | Same convergence |
| `gamma` | `scipy.special.gamma` | <1e-14 |
| `trapz` | `numpy.trapz` | <1e-13 |

### Performance

| Workload | Python | BarraCuda (inline) | BarraCuda (library) |
|----------|--------|-------------------|---------------------|
| L1 (30 rounds) | 129.9s | 9.3s (14×) | **9.3s** (maintain) |
| L2 (1000 evals) | 3571s | 2055s (1.7×) | **2055s** (maintain) |
| RBF train (5k pts) | 12.4s | 0.86s (14.4×) | **0.86s** (maintain) |

**Goal**: Library extraction adds ZERO overhead. Same performance as inline code.

---

## Part 6: Future Evolution

### Short-Term (Next 3 Months)

1. **f64 WGSL shaders** (for datacenter GPUs with native f64)
   - `cdist_f64.wgsl`, `tps_kernel_f64.wgsl`, `cholesky_f64.wgsl`
   - Feature-gated behind `SHADER_F64` capability detection
   - Fallback to dual-precision CPU path

2. **Advanced optimizers**
   - L-BFGS-B (quasi-Newton with bounds)
   - CMA-ES (evolution strategy)
   - Differential Evolution

3. **Surrogate enhancements**
   - Kriging/Gaussian Process regression
   - Expected Improvement acquisition (for Bayesian optimization)
   - Multi-fidelity surrogates

### Long-Term (6-12 Months)

1. **Tensor64 type** for f64-native workflows
   - `Tensor<T: Scalar>` generic over f32/f64
   - Transparent CPU/GPU dispatch based on dtype

2. **Distributed optimization**
   - Parallel function evaluations across gates
   - MPI-style reduce for global best

3. **NPU pre-screening**
   - Train binary classifier on surrogate (physical/unphysical)
   - Run at 1000× lower power before expensive GPU eval

---

## Part 7: Dependencies

### New Dependencies (add to Cargo.toml)

```toml
[dependencies]
# Already present
nalgebra = "0.32"     # For SymmetricEigen, DMatrix
rayon = "1.8"         # For parallel multi-start
thiserror = "1.0"     # For BarracudaError

# New (if needed)
rand = "0.8"          # For latin_hypercube, sparsity_sampler
rand_distr = "0.4"    # For sampling distributions
```

All are lightweight, pure Rust, and already widely used in the ecosystem.

---

## Part 8: Risk Mitigation

### Risk: Performance Regression

**Mitigation**: Benchmark every extraction against inline code. Use `criterion.rs`.

### Risk: API Instability

**Mitigation**: Start with `pub(crate)` visibility, promote to `pub` only after L3 validation.

### Risk: f64 GPU Availability

**Mitigation**: Dual-precision architecture proven. CPU f64 fallback is the default.

### Risk: Portability (mystic SparsitySampler)

**Mitigation**: Port algorithm, not Python code. Use hotSpring results for validation.

---

## Part 9: Documentation Plan

### User-Facing Docs

1. **`crates/barracuda/src/SCIENTIFIC_COMPUTING.md`**
   - Overview of scientific middleware
   - When to use which module
   - Dual-precision architecture explained

2. **Module-level README**
   - `linalg/README.md`, `surrogate/README.md`, etc.
   - Detailed examples
   - Performance tips

3. **Tutorial: RBF Surrogate from Scratch**
   - Step-by-step walkthrough
   - Compare to scipy
   - GPU acceleration benefits

### Developer Docs

1. **`docs/EXTRACTION_GUIDE.md`**
   - How to extract from hotSpring binaries
   - Testing against Python control
   - Integration into library

2. **API reference** (rustdoc)
   - Every `pub fn` has example
   - Links to papers/references where applicable

---

## Part 10: Rollout Strategy

### Phase 1: Internal Validation (Week 1-3)

- Extract modules
- Test against Python control
- L1/L2 binaries use library
- Verify identical results

### Phase 2: L3 Integration (Week 4-5)

- L3 (deformed HFB) is first external consumer
- Validates API ergonomics
- Finds missing functionality

### Phase 3: Public Release (Week 6)

- Promote modules from `pub(crate)` to `pub`
- Update barracuda README
- Announce to primals team

### Phase 4: Continuous Improvement (Ongoing)

- Add kernels/optimizers as needed
- Port best practices from literature
- Maintain Python parity

---

## Summary

BarraCuda's 414 WGSL shaders proved they can run real physics. The missing piece
is the **scientific computing middleware** — the 600 lines of duplicated Rust code
that connects "GPU tensor ops" to "find optimal nuclear parameters."

**This is a 3-week extraction project**, not a research project. The code exists,
validates against Python, and achieves 14× speedups on real workloads.

**Extract it. Module it. Test it. Every future scientific primal benefits.**

---

**Status**: APPROVED FOR EXECUTION  
**Start Date**: TBD (after shader reorganization)  
**Owner**: ToadStool/BarraCuda Team  
**Stakeholder**: hotSpring (L3 blocked on this)

**Next Steps**:
1. Review this plan
2. Allocate 3-week sprint
3. Begin Week 1 extraction (core infrastructure)

---

**References**:
- hotSpring validation: `phase1/toadstool/workloads/hotspring/`
- Source binaries: hotSpring control repository
- Python control: `hotSpring/control/surrogate/nuclear-eos/`
- Validation results: `.json` files in control/surrogate/nuclear-eos/results/

**Related Handoffs**:
- TOADSTOOL_PHYSICS_SHADERS_FEB08_2026.md
- TOADSTOOL_PURE_RUST_HARDWARE_EVOLUTION_FEB10_2026.md
- TOADSTOOL_REAL_HARDWARE_WIRING_FEB07_2026.md

**License**: AGPL-3.0
