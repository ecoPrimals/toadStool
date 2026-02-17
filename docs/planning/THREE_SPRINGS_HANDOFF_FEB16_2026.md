# Handoff: ToadStool → hotSpring, wetSpring, airSpring

**Date:** February 16, 2026  
**From:** ToadStool/BarraCUDA core team  
**To:** hotSpring, wetSpring, airSpring validation teams  
**Commit:** `0c477306` — `git pull origin master` to receive  
**License:** AGPL-3.0-or-later

---

## Executive Summary

All three spring handoffs have been processed. ToadStool now includes:

- **Bug fixes** from wetSpring (log_f64) and hotSpring (target keyword, SHADER_F64)
- **Unified primitives** serving all three domains
- **37 new tests** validating cross-spring functionality
- **Math f64 precision fixes** across all transcendental functions

**Combined validation**: 313+ Rust acceptance checks (hotSpring 195 + wetSpring 48 + airSpring 70)

---

## What's New in ToadStool (Pull to Receive)

### New Shaders

| Shader | Purpose | Primary Consumer |
|--------|---------|------------------|
| `fused_map_reduce_f64.wgsl` | Single-dispatch map + reduce | wetSpring |
| `cosine_similarity_f64.wgsl` | All-pairs f64 similarity | wetSpring |
| `batched_elementwise_f64.wgsl` | FAO-56 ET₀, water balance | airSpring |
| `kriging_f64.wgsl` | Spatial interpolation (4 variograms) | airSpring, wetSpring |

### New Rust Orchestrators

| Module | API | Features |
|--------|-----|----------|
| `FusedMapReduceF64` | `shannon_entropy()`, `simpson_index()`, `sum()`, `max()`, `min()` | Smart CPU/GPU routing (CPU for n < 1024) |
| `KrigingF64` | `interpolate()`, `interpolate_simple()`, `fit_variogram()` | Spherical, Exponential, Gaussian, Linear variograms |

### Bug Fixes Applied

| Bug | Source | Fix |
|-----|--------|-----|
| `log_f64()` coefficients 2x too large | wetSpring | Halved coefficients (~1e-3 → ~1e-15 precision) |
| `target` reserved keyword | hotSpring | Renamed to `target_val` in BCS bisection |
| SHADER_F64 not requested | hotSpring | Device creation now requests f64 when available |

### Math f64 Precision Evolution

All functions now use `(zero + literal)` pattern for full f64 precision:

```wgsl
// WRONG (truncates through f32)
let c1 = f64(0.333333333333333);

// CORRECT (full f64 precision)
let zero = x - x;
let c1 = zero + 0.333333333333333;
```

**Functions updated**: `exp_f64`, `sin_f64`, `cos_f64`, `sinh_f64`, `cosh_f64`, `erf_f64`, `gamma_f64`, `lanczos_core_f64`, `bessel_j0_f64`

---

## Per-Team Status and Next Steps

---

## hotSpring Team

### ✅ Your Contributions Absorbed

| Item | Status |
|------|:------:|
| `target` → `target_val` keyword fix | ✅ |
| SHADER_F64 device creation fix | ✅ |
| Broyden mixer | ✅ Already in |
| FD gradients (1D, 2D, cylindrical) | ✅ Already in |

### ✅ v0.5.5 Quality Hardening Acknowledged (Feb 16, 2026 evening)

| Metric | Value |
|--------|:-----:|
| Unit tests | 182 (+24) |
| Line coverage | 39% |
| Inline magic numbers | **0** |
| WGSL shaders extracted | 8 |
| Clippy warnings | 0 |

### 🎯 Primitives Ready for Your Next Evolution

All three primitives you identified are available in ToadStool:

| Primitive | Location | API |
|-----------|----------|-----|
| **SumReduceF64** | `barracuda::ops::sum_reduce_f64` | `SumReduceF64::sum(device, data)` |
| **SpinOrbitGpu** | `barracuda::ops::grid::spin_orbit_f64` | `SpinOrbitGpu::compute(...)` |
| **FusedMapReduceF64** | `barracuda::ops::fused_map_reduce_f64` | `FusedMapReduceF64::execute(...)` |

#### Priority 1: Wire SumReduceF64 for HFB Energy

Replace CPU `trapz` with GPU reduction:

```rust
use barracuda::ops::sum_reduce_f64::SumReduceF64;

// Your batched_hfb_energy_f64.wgsl already computes integrands
// After shader execution, read back integrand buffer...

// BEFORE (CPU trapz):
// let energy = trapz(&integrands, dr);

// AFTER (GPU reduce):
let energy = SumReduceF64::sum(device.clone(), &integrands)? * dr;
```

#### Priority 3: Wire SpinOrbitGpu for HFB Hamiltonian

```rust
use barracuda::ops::grid::spin_orbit_f64::SpinOrbitGpu;

let so = SpinOrbitGpu::new(device.clone());

// With pre-computed density gradient
let h_so = so.compute(
    &wf_squared,     // [batch × n_states × n_grid]
    &drho_dr,        // [batch × n_grid]
    &r_grid,         // [n_grid]
    &ls_factors,     // [batch × n_states]
    dr,              // grid spacing
    w0,              // spin-orbit coupling (MeV·fm⁵)
)?;

// Or compute gradient internally
let h_so = so.compute_with_density(&wf_squared, &density, &r_grid, &ls_factors, dr, w0)?;
```
| Hermite/Laguerre f64 | ✅ Already in |
| BatchedEighGpu | ✅ Already in |
| GPU SSF | ✅ Already in |

### 🎯 What You Can Work On Now

#### Priority 1.1: Single-Dispatch Eigensolve (CRITICAL PATH)

**The Problem**: `BatchedEighGpu` issues a separate `queue.submit()` per Jacobi rotation. For 12×12 matrices needing ~100 rotations across 19 nuclei × 2 isospins × 200 iterations = **760,000 submissions**. This is why GPU-resident was 16× slower.

**The Solution**: A single-dispatch Jacobi kernel where ALL rotations for ALL matrices execute inside ONE shader invocation with workgroup barriers.

**Interface Sketch**:
```wgsl
@compute @workgroup_size(64)
fn batched_jacobi_full(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let matrix_idx = wg_id.x;
    // Each workgroup handles one matrix
    // Loop over rotations WITH workgroupBarrier() between
    for (var iter = 0u; iter < max_iterations; iter++) {
        // Find max off-diagonal (parallel reduction)
        // Apply Givens rotation
        workgroupBarrier();
    }
}
```

**Target**: B=40 matrices, n=12, total < 1ms (vs current ~100ms)

**Impact**: Estimated 3-5× speedup (poll time 1.45s → ~0s)

#### Priority 2.1: GPU Spin-Orbit (Low-Hanging Fruit)

The shader exists at `shaders/grid/spin_orbit_f64.wgsl`. Just needs:
1. Rust orchestrator in `ops/grid/`
2. Wire to HFB pipeline

#### Priority 2.2: `pow_f64()` Function

ToadStool needs a full `pow_f64(base, exp)` for arbitrary exponents. Can be built from `exp_f64` and `log_f64`:

```wgsl
fn pow_f64(base: f64, exp: f64) -> f64 {
    return exp_f64(exp * log_f64(base));
}
```

---

## wetSpring Team

### ✅ Your Contributions Absorbed

| Item | Status |
|------|:------:|
| `log_f64()` bug fix | ✅ Coefficients halved |
| `zero + literal` pattern | ✅ Documented and applied |
| Shannon map concept | ✅ Evolved to `FusedMapReduceF64` |
| Simpson map concept | ✅ Evolved to `FusedMapReduceF64` |
| **Bray-Curtis shader** | ✅ **ABSORBED** — `ops::bray_curtis_f64::BrayCurtisF64` |

### ✅ Bray-Curtis Shader Absorbed (Feb 16, 2026)

Your `bray_curtis_pairs_f64.wgsl` has been absorbed into ToadStool:

```rust
use barracuda::ops::bray_curtis_f64::BrayCurtisF64;

let bc = BrayCurtisF64::new(device.clone())?;

// 100 samples, 500 features each → condensed distance matrix
let distances = bc.condensed_distance_matrix(&samples, 100, 500)?;
// distances.len() = 100*99/2 = 4950

// Convert index to sample pair
let (i, j) = BrayCurtisF64::condensed_index_to_pair(idx);
```

### 🎯 What You Can Work On Now

#### Immediate: Use New Primitives

**Shannon Entropy** (replaces your map + CPU sum):
```rust
use barracuda::ops::fused_map_reduce_f64::FusedMapReduceF64;

let fmr = FusedMapReduceF64::new(device)?;
let counts = vec![10.0, 20.0, 30.0, 40.0];
let shannon = fmr.shannon_entropy(&counts)?;
// Returns 1.27985422... (validated to 1e-10 vs CPU)
```

**Simpson Index**:
```rust
let simpson = fmr.simpson_index(&counts)?;
```

**Spatial Interpolation** (for sampling sites):
```rust
use barracuda::ops::kriging_f64::{KrigingF64, VariogramModel};

let kriging = KrigingF64::new(device)?;
let known = vec![(0.0, 0.0, 1.5), (10.0, 0.0, 2.1), ...]; // (x, y, diversity)
let targets = vec![(5.0, 5.0), (2.5, 7.5)];
let model = VariogramModel::Spherical { nugget: 0.0, sill: 0.5, range: 15.0 };
let result = kriging.interpolate(&known, &targets, model)?;
// result.values = interpolated diversity
// result.variances = uncertainty at each point
```

#### Priority 1: PCoA on Bray-Curtis

Your Bray-Curtis distance matrix can now feed directly to `BatchedEighGpu` for Principal Coordinates Analysis:

```rust
// 1. Compute Bray-Curtis condensed distance matrix (your existing code)
// 2. Convert to full distance matrix
// 3. Double-center: B = -0.5 * H * D² * H where H = I - 1/n * 11'
// 4. Eigensolve: BatchedEighGpu::execute_f64(&centered_matrix)
// 5. Top k eigenvectors = ordination axes
```

#### Priority 2: Rarefaction

GPU random subsampling using existing `prng_xoshiro.wgsl`:
1. Generate random indices on GPU
2. Subsample counts
3. Compute diversity metrics
4. Repeat for confidence intervals

#### Priority 3: m/z Tolerance Search

Use `batched_bisection_f64.wgsl` for binary search on sorted m/z arrays:
- Input: query masses, sorted reference masses, tolerance
- Output: matching indices

---

## airSpring Team

### ✅ Your Contributions Absorbed

| Item | Status |
|------|:------:|
| FAO-56 ET₀ concept | ✅ Implemented in `batched_elementwise_f64.wgsl` |
| Water balance concept | ✅ Implemented in `batched_elementwise_f64.wgsl` |
| Spatial interpolation need | ✅ `KrigingF64` with 4 variogram models |
| Validation architecture | ✅ Pattern documented |

### ✅ ToadStool Issues Resolved (Feb 16, 2026)

All four ToadStool issues you identified have been fixed:

| ID | Severity | Issue | Status |
|----|:--------:|-------|:------:|
| TS-001 | Critical | `pow_f64` returns 0.0 for fractional exponents | ✅ **FIXED** |
| TS-002 | Medium | No Rust orchestrator for `batched_elementwise_f64` | ✅ **FIXED** |
| TS-003 | Medium | `acos`/`sin` precision drift | ✅ **FIXED** |
| TS-004 | High | `FusedMapReduceF64` buffer conflict for N≥1024 | ✅ **FIXED** |

See `docs/planning/AIRSPRING_TS_ISSUES_RESOLVED_FEB16_2026.md` for details.

### 🎯 What You Can Work On Now

#### Immediate: Use Kriging for Soil Moisture Mapping

```rust
use barracuda::ops::kriging_f64::{KrigingF64, VariogramModel};

// Your sensor network
let sensors = vec![
    (0.0, 0.0, 0.35),     // (x_meters, y_meters, VWC)
    (100.0, 0.0, 0.28),
    (0.0, 100.0, 0.32),
    (100.0, 100.0, 0.25),
    (50.0, 50.0, 0.30),
];

// Target grid (10x10 = 100 points)
let grid: Vec<(f64, f64)> = (0..10)
    .flat_map(|i| (0..10).map(move |j| (i as f64 * 10.0, j as f64 * 10.0)))
    .collect();

let kriging = KrigingF64::new(device)?;
let model = VariogramModel::Spherical {
    nugget: 0.001,  // Measurement noise
    sill: 0.01,     // Total variance
    range: 75.0,    // Correlation range ~75m
};

let result = kriging.interpolate(&sensors, &grid, model)?;
// result.values[i] = interpolated VWC at grid[i]
// result.variances[i] = uncertainty (useful for adaptive sampling)
```

#### ✅ Batched ET₀ Orchestrator (TS-002 — DONE)

The Rust orchestrator is now available in `barracuda::ops::batched_elementwise_f64`:

```rust
use barracuda::ops::batched_elementwise_f64::{BatchedElementwiseF64, StationDayInput};

let executor = BatchedElementwiseF64::new(device.clone())?;

// FAO-56 Example 18: Uccle, Belgium
let station_days: Vec<StationDayInput> = vec![
    (21.5, 12.3, 84.0, 63.0, 2.78, 22.07, 100.0, 50.8, 187),
    // (tmax, tmin, rh_max, rh_min, wind_2m, rs, elevation, latitude, doy)
];

let et0_values = executor.fao56_et0_batch(&station_days)?;
// et0_values[0] ≈ 3.88 mm/day (validated against FAO-56 Example 18)
```

Also available: `water_balance_batch()` for daily depletion updates.

#### Priority 2: Richards Equation 1D Solver

Unsaturated flow through soil column:
```
∂θ/∂t = ∂/∂z [K(h)(∂h/∂z + 1)] - S(z,t)
```

**Architecture**:
- Finite difference on 1D grid
- Implicit time stepping (Crank-Nicolson)
- Uses existing `CgGpu` for tridiagonal solve
- Similar structure to hotSpring's HFB iteration but simpler (scalar field on 1D grid)

#### Priority 3: Statistical Methods

Port remaining Python methods to Rust:
- `compute_rmse(obs, sim)` — Simple
- `compute_ia(obs, sim)` — Index of Agreement
- `compute_r2(obs, sim)` — Correlation coefficient
- `fit_correction_equations()` — Needs nonlinear solver

---

## Test Suite Available

All teams can run validation tests:

```bash
# All 37 evolution tests
cargo test -p barracuda --test three_springs_evolution_tests

# Specific categories
cargo test -p barracuda --test three_springs_evolution_tests fused_map_reduce
cargo test -p barracuda --test three_springs_evolution_tests kriging
cargo test -p barracuda --test three_springs_evolution_tests e2e
cargo test -p barracuda --test three_springs_evolution_tests chaos
cargo test -p barracuda --test three_springs_evolution_tests fault
cargo test -p barracuda --test three_springs_evolution_tests precision
```

---

## Cross-Spring Collaboration Opportunities

| Pattern | Provider | Consumer | Use Case |
|---------|----------|----------|----------|
| `KrigingF64` | ToadStool | airSpring, wetSpring | Sensor mapping, site interpolation |
| `FusedMapReduceF64` | ToadStool | wetSpring, airSpring | Diversity metrics, batch reductions |
| `BatchedEighGpu` | ToadStool | wetSpring, hotSpring | PCoA, HFB eigensolve |
| Single-dispatch Jacobi | hotSpring | All | Template for iterative GPU kernels |
| Richards solver | airSpring | wetSpring | Soil water modeling |

---

## Git Commands

```bash
# Pull latest
cd /path/to/toadStool
git pull origin master

# Verify
cargo build --release -p barracuda
cargo test -p barracuda --test three_springs_evolution_tests
```

---

## Questions?

File issues at `ecoPrimals/toadStool` or discuss in `wateringHole/`.

---

*February 16, 2026 — Unified math library complete. 313+ validation checks across nuclear physics, life science, and precision agriculture. Single-dispatch eigensolve is the critical path for hotSpring Tier 1. Kriging and fused map-reduce ready for immediate use.*

---

## February 17, 2026 Update — fp32/fp64 Shader Evolution

### New f64 Shaders Added

| Shader | Purpose | Location |
|--------|---------|----------|
| `rk4_f64.wgsl` | 4th-order Runge-Kutta time integration | `ops/md/integrators/` |
| `rk_stage_f64.wgsl` | Dormand-Prince RK45 stages | `shaders/numerical/` |
| `linsolve_f64.wgsl` | Gaussian elimination with pivoting | `shaders/linalg/` |
| `inverse_f64.wgsl` | Gauss-Jordan matrix inversion | `shaders/linalg/` |
| `crank_nicolson_f64.wgsl` | Implicit PDE solver with ADI | `shaders/pde/` |

### Bugs Fixed

| Issue | Root Cause | Fix |
|-------|------------|-----|
| `fd_gradient_f64` pipeline binding errors | Shader used different @group() for different entry points | Changed all 2D/cylindrical operations to @group(0) |
| `qr_gpu` pipeline binding mismatch | Inconsistent storage access modes across entry points | Changed all read bindings to read_write |
| `batched_eigh_gpu` f64 device errors | Tests requested non-f64 device for f64 shaders | Updated 9 tests to use `get_test_device_if_f64_gpu_available()` |

### Test Results

| Category | Passed | Failed | Ignored |
|----------|--------|--------|---------|
| f64 tests | 173 | 0 | 2 |
| linalg tests | 75 | 0 | 0 |
| MD tests | 99 | 0 | 1 |

### Shader Coverage Summary

| Status | Count |
|--------|-------|
| fp32 + fp64 pairs | 38+ |
| fp64 only | 23 |
| Critical gaps filled | 5 (ODE/PDE, linear algebra) |

### Remaining Known Issues

1. **Pre-existing tensor op failures**: Many basic ops (`add`, `mul`, etc.) have test failures unrelated to f64 work
2. **Sparse solver tests**: `cg_gpu` and `bicgstab_gpu` tests fail (pre-existing)
3. **SSF GPU**: `compute_axes` has k-ordering bug (ignored test)

### Commits

- `5c90eb29`: Fix fd_gradient_f64 bind group mismatch (5 tests → pass)
- `55b3d174`: Add critical f64 shader implementations (5 new shaders)
- `728838fa`: Resolve shader/pipeline binding mismatches (QR, batched_eigh)
