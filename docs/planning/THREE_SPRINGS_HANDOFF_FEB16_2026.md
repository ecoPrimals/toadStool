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
| Bray-Curtis concept | ✅ Pattern in `cosine_similarity_f64.wgsl` |

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

#### Priority 1: Wire Batched ET₀ Orchestrator

The shader `batched_elementwise_f64.wgsl` has FAO-56 Penman-Monteith fully implemented. Create Rust orchestrator:

```rust
pub struct BatchedEt0Gpu {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
}

impl BatchedEt0Gpu {
    pub fn compute(&self, inputs: &[Et0Input]) -> Result<Vec<f64>> {
        // Pack inputs: [tmax, tmin, rh_max, rh_min, wind, Rs, lat, elev, doy] per station
        // Dispatch: ceil(n / 64) workgroups
        // Read back: n ET₀ values
    }
}
```

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
