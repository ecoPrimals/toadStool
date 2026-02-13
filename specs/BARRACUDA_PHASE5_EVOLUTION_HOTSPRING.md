# BarraCUDA Phase 5 Evolution — hotSpring Validation Response

**Date**: February 13, 2026  
**Status**: TIER 1 + TIER 2 + TIER 3 COMPLETE  
**Reference**: hotSpring Phase 5 Handoff (Feb 13, 2026)

---

## Executive Summary

This document tracks BarraCUDA's response to the Phase 5 hotSpring validation handoff. The hotSpring team validated BarraCUDA against Python/scipy controls using nuclear EOS workloads, identifying both victories and critical gaps.

### Headline Results (from hotSpring)

| Pipeline | BarraCUDA | Python/scipy | Verdict |
|----------|-----------|-------------|---------|
| **L1 (SEMF)** | χ²/datum = 1.19 | χ²/datum = 6.62 | **✅ BarraCUDA WINS by 82%** |
| **Validation Suite** | **129/129** tests | — | **✅ COMPLETE** |

---

## Tier 1 Fixes — IMPLEMENTED ✅

### 1. LOO-CV Hat Matrix Bug Fixed

**Bug**: `RBFSurrogate::compute_hat_diagonal()` used K_smooth (kernel + regularization) for both the system matrix AND the right-hand side, causing H_ii = 1.0 always.

**Fix**: Separated K_raw (pure kernel) from K_smooth (kernel + λI):
- System matrix: K_smooth
- Right-hand side: standard basis vector e_i
- H_ii = K_raw[i,:] · (K_smooth⁻¹ · e_i)

**Location**: `barracuda::surrogate::rbf::compute_hat_diagonal()`

**Tests**: `test_loo_cv_hat_diagonal_correct`, `test_loo_cv_smoothing_effect`

---

### 2. Auto-Smoothing via LOO-CV Grid Search

**Problem**: Default smoothing (1e-12) causes exact interpolation → overfitting on rugged landscapes.

**Solution**: Added `loo_cv_optimal_smoothing()` function and `SparsitySamplerConfig::auto_smoothing` flag.

**API**:
```rust
// Standalone function
let (opt_s, opt_rmse, results) = loo_cv_optimal_smoothing(
    &x_data, &y_data, kernel, None  // None = default grid
)?;

// Config builder
let config = SparsitySamplerConfig::new(10, 42)
    .with_auto_smoothing(true);
```

**Location**: `barracuda::surrogate::rbf::loo_cv_optimal_smoothing()`

---

### 3. Penalty Filtering for Surrogate Training

**Problem**: Large penalty values (from infeasible regions) corrupt RBF surrogate approximation.

**Solution**: Added `PenaltyFilter` enum and `SparsitySamplerConfig::penalty_filter` field.

**API**:
```rust
pub enum PenaltyFilter {
    None,                    // Default: no filtering
    Threshold(f64),          // Remove y > threshold
    Quantile(f64),           // Remove top q% outliers
    AdaptiveMAD(f64),        // Median + k×MAD
}

let config = SparsitySamplerConfig::new(10, 42)
    .with_penalty_filter(PenaltyFilter::Threshold(12.0));
```

**Location**: `barracuda::sample::sparsity::filter_training_data()`

---

### 4. Warm-Start API for L1→L2 Seeding

**Problem**: Random LHS starts in 10D L2 space almost never hit NMP-valid regions.

**Solution**: Added `SparsitySamplerConfig::warm_start_seeds` field.

**API**:
```rust
// Best solutions from L1 optimization
let l1_best = vec![
    vec![0.1, 0.2, 0.3, ...],
    vec![0.15, 0.25, 0.35, ...],
];

let config = SparsitySamplerConfig::new(10, 42)
    .with_warm_start(l1_best);
```

**Location**: `barracuda::sample::sparsity::SparsitySamplerConfig`

---

### 5. digamma() and beta() CPU f64 Functions

**Problem**: Functions were removed as standalone CPU f64, only existed as GPU tensor ops.

**Solution**: Implemented CPU f64 versions using:
- `digamma(x)`: Recurrence + asymptotic expansion
- `beta(a, b)`: exp(ln_gamma(a) + ln_gamma(b) - ln_gamma(a+b))
- `ln_beta(a, b)`: Overflow-safe log-beta

**API**:
```rust
use barracuda::special::{digamma, beta, ln_beta};

let psi = digamma(1.0)?;     // ≈ -0.5772... (Euler-Mascheroni)
let b = beta(2.0, 3.0)?;     // = 1/12
let lb = ln_beta(100.0, 100.0)?;  // Overflow-safe
```

**Location**: `barracuda::special::gamma`

**Tests**: 8 new tests for digamma/beta correctness

---

## Tier 2 Implementations — COMPLETE ✅

### 1. Direct Sampler (`barracuda::sample::direct`)

Round-based Nelder-Mead on the true objective function (not surrogate-guided). Surrogate is trained for monitoring only.

**API**:
```rust
let config = DirectSamplerConfig::new(42)
    .with_rounds(5)
    .with_solvers(8)
    .with_patience(2)
    .with_warm_start(l1_seeds);

let result = direct_sampler(objective, &bounds, &config)?;
// result.x_best, result.f_best, result.cache, result.rounds
```

**Reference**: `surrogate.rs::round_based_direct_optimization()` achieving χ²/datum = 1.19

---

### 2. Chi-Squared Decomposition (`barracuda::stats::chi2`)

Per-datum analysis with residuals, pulls (standardized residuals), and contributions.

**API**:
```rust
let result = chi2_decomposed(&observed, &expected, n_params)?;
println!("{}", result.summary());
let worst = result.worst_n(3);  // Top 3 outliers
```

**Features**:
- `chi2_decomposed()` — Poisson-like (σ = √E)
- `chi2_decomposed_weighted()` — With known uncertainties
- `Chi2Decomposed::worst_n()` — Identify N worst points
- `Chi2Decomposed::summary()` — Human-readable report

---

### 3. Bootstrap Confidence Intervals (`barracuda::stats::bootstrap`)

Non-parametric CI for any statistic via resampling.

**API**:
```rust
let ci = bootstrap_ci(&data, |s| s.iter().sum::<f64>() / s.len() as f64, 1000, 0.95, 42)?;
println!("{}", ci.summary());  // "5.50 (95% CI: [4.80, 6.20], SE: 0.35)"

// Convenience functions
let mean_ci = bootstrap_mean(&data, 1000, 0.95, 42)?;
let median_ci = bootstrap_median(&data, 1000, 0.95, 42)?;
```

---

### 4. Convergence Diagnostics (`barracuda::optimize::diagnostics`)

Detect stagnation, oscillation, and divergence in optimization trajectories.

**API**:
```rust
let diag = convergence_diagnostics(&history, window, threshold, patience)?;
match diag.state {
    ConvergenceState::Improving => { /* continue */ }
    ConvergenceState::Stagnant => { /* consider stopping */ }
    ConvergenceState::Diverging => { /* abort */ }
    _ => {}
}

// Simple helper
if should_stop_early(&history, 0.01, 3) {
    break;
}
```

---

### 5. Adaptive Penalty (`barracuda::optimize::penalty`)

Data-driven penalty from observed feasible values.

**API**:
```rust
let penalty = adaptive_penalty(&feasible_values, PenaltyConfig::default())?;
let penalized_value = penalty.apply(constraint_violation);

// Robust MAD-based variant
let penalty = adaptive_penalty_mad(&all_values, config, 5.0)?;
```

---

## Tier 3 Implementations — COMPLETE ✅

### 1. Auto-Dispatch Benchmark Suite ✅ (`barracuda::dispatch::benchmark`)

Empirically determine optimal CPU/GPU thresholds for each operation.

**API**:
```rust
let suite = BenchmarkSuite::new(BenchmarkConfig::default());
let results = suite.run_all()?;
println!("{}", results.summary());

// Get optimal thresholds
let thresholds = results.optimal_thresholds();
```

**Features**:
- `BenchmarkConfig::quick()` / `default()` / `thorough()` presets
- Per-operation timing with warmup and multiple iterations
- Crossover detection with configurable speedup threshold
- Safety margin for threshold recommendations

---

### 2. Pipeline Orchestration API ✅ (`barracuda::pipeline`)

Declarative heterogeneous compute pipelines following the hotSpring cascade pattern.

**API**:
```rust
let cascade = Cascade::<Point, f64>::builder()
    .filter("nmp_check", |x| check_nmp_constraints(x))
    .filter("proxy_filter", |x| semf_proxy(x) < threshold)
    .transform("full_eval", |x| hfb_objective(x))
    .build();

let result = cascade.run(&candidates);
println!("{}", result.summary());
// Input: 6000 → Evaluated: 488 → Savings: 91.9%
```

**Features**:
- `Cascade` — Multi-stage filtering pipeline
- `Stage` — Filter and/or transform with target device
- `Target::Cpu`, `CpuParallel`, `Gpu`, `Npu`, `Auto`
- Per-stage statistics and overall savings metrics

---

### 3. Sparse Linear Algebra ✅ (`barracuda::linalg::sparse`)

Sparse matrix representations and iterative solvers for large-scale problems.

**API**:
```rust
use barracuda::linalg::sparse::{CsrMatrix, cg_solve, bicgstab_solve, SolverConfig};

// Build sparse SPD matrix (tridiagonal Laplacian)
let n = 1000;
let mut triplets = Vec::new();
for i in 0..n {
    triplets.push((i, i, 4.0));
    if i > 0 { triplets.push((i, i-1, -1.0)); }
    if i < n-1 { triplets.push((i, i+1, -1.0)); }
}
let a = CsrMatrix::from_triplets(n, n, &triplets);

// Solve with Conjugate Gradient
let b: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
let result = cg_solve(&a, &b, 1e-10, 500)?;
assert!(result.converged);

// For non-symmetric systems, use BiCGSTAB
let result = bicgstab_solve(&a, &b, 1e-10, 500)?;
```

**Storage Formats**:
- `CsrMatrix` — Compressed Sparse Row: O(nnz) SpMV, efficient row access
- `CooMatrix` — Coordinate: Easy construction, convert to CSR

**Solvers**:
- `cg_solve()` — Preconditioned Conjugate Gradient (SPD matrices)
- `bicgstab_solve()` — BiCGSTAB (general non-symmetric)
- `jacobi_solve()` — Jacobi iteration (diagonally dominant)

**Features**:
- Diagonal preconditioning (Jacobi) for faster convergence
- `SolverConfig` for tolerance, max iterations, verbosity
- Factory methods: `identity()`, `from_diagonal()`, `tridiagonal()`
- Sparsity metrics and symmetry checking
- Dense conversion for debugging small matrices

---

## Remaining Work

### Tier 4: Hardware (when Titan V arrives)

| Task | Status | Notes |
|------|--------|-------|
| f64 WGSL shader variants | PENDING | Full GPU f64 |
| Multi-GPU DevicePool | PENDING | f32→RTX, f64→Titan V routing |
| VFIO NPU driver | STARTED | Pure Rust backend implemented; awaiting hardware validation |

---

## API Changes Documentation

### Breaking Changes (from Phase 3/4)

1. **`gamma(x)` now returns `Result<f64>`** instead of `f64`
   - Migration: Add `.unwrap()` or `?` to all call sites
   
2. **`lgamma` renamed to `ln_gamma`**
   - Migration: Find/replace `lgamma` → `ln_gamma`

### New Exports

```rust
// barracuda::special (Tier 1)
pub use gamma::{digamma, beta, ln_beta};

// barracuda::surrogate (Tier 1)
pub use rbf::{loo_cv_optimal_smoothing, LooSmoothing};

// barracuda::sample (Tier 1 + 2)
pub use sparsity::{PenaltyFilter, SparsitySamplerConfig};
pub use direct::{direct_sampler, DirectSamplerConfig, DirectSamplerResult};

// barracuda::stats (Tier 2)
pub use chi2::{chi2_decomposed, chi2_decomposed_weighted, Chi2Decomposed};
pub use bootstrap::{bootstrap_ci, bootstrap_mean, bootstrap_median, bootstrap_std, BootstrapCI};

// barracuda::optimize (Tier 2)
pub use diagnostics::{convergence_diagnostics, should_stop_early, ConvergenceDiagnostics, ConvergenceState};
pub use penalty::{adaptive_penalty, adaptive_penalty_mad, AdaptivePenalty, PenaltyConfig};

// barracuda::dispatch (Tier 3)
pub use benchmark::{BenchmarkConfig, BenchmarkSuite, BenchmarkResult, ThresholdResult};

// barracuda::pipeline (Tier 3)
pub use cascade::{Cascade, CascadeBuilder, CascadeResult};
pub use stage::{Stage, StageConfig, Target};

// barracuda::linalg::sparse (Tier 3)
pub use csr::{CsrMatrix, CooMatrix};
pub use solvers::{cg_solve, bicgstab_solve, jacobi_solve, SolverConfig, SolverResult};
```

---

## Test Coverage

| Module | Tests | Status |
|--------|-------|--------|
| `special::gamma` (digamma, beta) | 8 new | ✅ PASS |
| `surrogate::rbf` (LOO-CV) | 3 new | ✅ PASS |
| `sample::sparsity` | 10 existing | ✅ PASS |
| `sample::direct` (direct_sampler) | 4 new | ✅ PASS |
| `stats::chi2` (chi2_decomposed) | 7 new | ✅ PASS |
| `stats::bootstrap` (bootstrap_ci) | 7 new | ✅ PASS |
| `optimize::diagnostics` (convergence) | 5 new | ✅ PASS |
| `optimize::penalty` (adaptive_penalty) | 6 new | ✅ PASS |
| `dispatch::benchmark` | 6 new | ✅ PASS |
| `dispatch::config` | 6 existing | ✅ PASS |
| `pipeline::stage` | 5 new | ✅ PASS |
| `pipeline::cascade` | 5 new | ✅ PASS |

Total new tests (Tier 1 + 2 + 3): 62

---

## Architecture Patterns Validated

### 1. Dual-Precision Strategy ✅

```
Distance computation: f32 GPU shader → Fast
Kernel evaluation:    f64 CPU → Accuracy
Linear algebra:       f64 CPU → Accuracy
Optimization:         f64 CPU → Accuracy
```

### 2. L1-Seeded L2 Pattern ✅

```
Phase 1: Cheap L1 (SEMF) on 5000 LHS points [0.5s]
Phase 2: Sort by L1, take top-K seeds
Phase 3: Expensive L2 (HFB) on K seeds [743s]
Result: 2× better than random, physically-valid starts
```

### 3. Round-Based Direct Optimization ✅

```
FOR round in 0..max_rounds:
  1. Generate starting points
  2. Run multi-start NM on TRUE objective
  3. Add all evaluations to cache
  4. Train surrogate (monitoring only)
  5. Compute LOO-CV RMSE
  6. Early stop if no improvement
```

---

## References

- **hotSpring Validation**: `hotSpring/control/surrogate/nuclear-eos/results/`
- **Reference Implementations**: `hotSpring/barracuda/src/{surrogate,stats}.rs`
- **Previous Handoffs**: `wateringHole/handoffs/BARRACUDA_PHASE{3,4}_*.md`
- **Phase 3 Spec**: `specs/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md`
