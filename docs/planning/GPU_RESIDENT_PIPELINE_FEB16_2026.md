# GPU-Resident Pipeline Evolution — February 16, 2026

**From**: hotSpring Experiment 005 findings  
**Status**: Planning  
**Goal**: Pure GPU faster than CPU for iterative solvers

---

## The Problem

hotSpring's L2 mega-batch experiment revealed the **Amdahl's Law boundary**:

| Metric | Value |
|--------|-------|
| GPU utilization | 95% |
| Dispatch count | 101 (down from 145k) |
| Wall time | 40.9 min (GPU) vs 35s (CPU) |
| **Slowdown** | **70×** |

The eigensolve is only 1% of the SCF iteration. The other 99% (Hamiltonian
construction, BCS pairing, density updates) runs on CPU. Each CPU step
requires GPU→CPU readback and CPU→GPU upload.

**The fix is not better batching. It is moving ALL physics to GPU.**

---

## The Complexity Boundary

For n×n matrix eigensolves:

| n | GPU compute | Dispatch overhead | GPU wins? |
|:-:|:-----------:|:-----------------:|:---------:|
| 12 | ~8 ms | ~50 ms | No (14% compute) |
| 30 | ~125 ms | ~50 ms | Marginal (71%) |
| **50** | **~580 ms** | **~50 ms** | **Yes (92%)** |
| 100+ | >4.6 s | ~50 ms | Dominant |

**Below n≈30**: CPU cache coherence beats GPU parallelism.  
**Above n≈50**: GPU massively parallel Jacobi dominates.  
**For n<30**: GPU wins ONLY with zero CPU↔GPU round-trips during iteration.

---

## Evolution Targets

### 1. Multi-Kernel Pipeline Without CPU Round-Trips

**Problem**: Current pattern requires CPU readback between dependent ops.

**Solution**: Chain dependent GPU operations where output buffer of shader 1
becomes input buffer of shader 2, without CPU involvement.

```
Current:  H-build → [readback] → eigensolve → [readback] → BCS → [readback] → density
Target:   H-build ──────────────> eigensolve ────────────> BCS ────────────> density
                    (GPU buffer)              (GPU buffer)       (GPU buffer)
```

**API Design**:

```rust
// Option A: Pipeline builder with explicit buffer handles
let pipeline = PipelineBuilder::new(&device)
    .add_stage("hamiltonian", hamiltonian_shader, &[input_buf], &[h_buf])
    .add_stage("eigensolve", eigh_shader, &[h_buf], &[eigenvalues_buf, eigenvectors_buf])
    .add_stage("bcs_pairing", bcs_shader, &[eigenvalues_buf], &[occupations_buf])
    .add_stage("density", density_shader, &[eigenvectors_buf, occupations_buf], &[rho_buf])
    .build()?;

// Execute all stages with single submit
pipeline.execute().await?;

// Option B: Extend begin_batch() with dependent ops
let mut batch = device.begin_batch();
let h_buf = batch.op(hamiltonian_op, &[input_buf]);  // Returns buffer handle
let (evals, evecs) = batch.op(eigh_op, &[h_buf]);
let occs = batch.op(bcs_op, &[evals]);
let rho = batch.op(density_op, &[evecs, occs]);
batch.end().await?;  // Single GPU submit
```

**Location**: `crates/barracuda/src/pipeline/` or extend `device/tensor_context.rs`

**Complexity**: Medium-High (requires buffer lifetime tracking across ops)

---

### 2. GPU Hamiltonian Construction Kernel

**Problem**: Hamiltonian assembly runs on CPU.

**Formula**:
```
H[i,j] = T_eff[i,j] + ∫ φ_i(r) · V(ρ,τ,J; params) · φ_j(r) · r²dr
```

This is a weighted inner product over grid points — embarrassingly parallel
per matrix element. ToadStool already has `weighted_dot_f64`. The extension
is a **batched grid-quadrature GEMM**.

**API Design**:

```rust
/// Batched grid quadrature: H[b,i,j] = Σ_k φ[b,i,k] * W[b,k] * φ[b,j,k] * weights[k]
pub struct GridQuadratureGemm {
    device: WgpuDevice,
    batch_size: usize,
    basis_size: usize,  // n
    grid_size: usize,   // number of quadrature points
}

impl GridQuadratureGemm {
    /// phi: [batch, n, grid] - basis functions on grid
    /// w: [batch, grid] - weight function (potential * r²)
    /// quad_weights: [grid] - quadrature weights
    /// output: [batch, n, n] - Hamiltonian matrices
    pub async fn execute(
        &self,
        phi: &Tensor,
        w: &Tensor,
        quad_weights: &Tensor,
    ) -> Result<Tensor, BarracudaError>;
}
```

**WGSL Kernel**: `grid_quadrature_gemm_f64.wgsl`

```wgsl
// Each workgroup computes one H[b,i,j] element
@compute @workgroup_size(256)
fn grid_quadrature(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let batch = gid.z;
    let i = gid.y;
    let j = gid.x;
    
    var sum = f64(0.0);
    for (var k = 0u; k < grid_size; k = k + 1u) {
        let phi_i = phi[batch * n * grid + i * grid + k];
        let phi_j = phi[batch * n * grid + j * grid + k];
        let weight = w[batch * grid + k] * quad_weights[k];
        sum = sum + phi_i * weight * phi_j;
    }
    
    output[batch * n * n + i * n + j] = sum;
}
```

**Location**: `crates/barracuda/src/ops/physics/grid_quadrature_gemm_f64.rs`

**Complexity**: Medium (variation of batched GEMM with weight function)

---

### 3. GPU BCS Pairing Kernel (Batched Bisection)

**Problem**: BCS pairing requires root-finding for chemical potential μ.

**Formula**:
```
Find μ such that: Σ_k v²_k(μ) = N
where v²_k = ½(1 - (ε_k - μ)/√((ε_k - μ)² + Δ²))
```

This is 1,582 independent bisection problems (791 nuclei × 2 isospins).

**API Design**:

```rust
/// Batched 1D bisection root-finding on GPU
pub struct BatchedBisection {
    device: WgpuDevice,
    max_iterations: u32,
    tolerance: f64,
}

impl BatchedBisection {
    /// Find roots of f(x; params) = 0 for each problem in batch
    /// 
    /// lower: [batch] - lower bounds
    /// upper: [batch] - upper bounds
    /// params: [batch, param_count] - function parameters per problem
    /// Returns: [batch] - roots
    pub async fn solve(
        &self,
        lower: &Tensor,
        upper: &Tensor,
        params: &Tensor,
        function_id: BisectionFunction,
    ) -> Result<Tensor, BarracudaError>;
}

pub enum BisectionFunction {
    BcsParticleNumber,  // Built-in: Σ v²_k(μ) - N
    Custom(String),      // User-provided WGSL function
}
```

**WGSL Kernel**: `batched_bisection_f64.wgsl`

```wgsl
// Each thread solves one bisection problem
@compute @workgroup_size(64)
fn batched_bisection(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let problem_idx = gid.x;
    if (problem_idx >= batch_size) { return; }
    
    var lo = lower[problem_idx];
    var hi = upper[problem_idx];
    
    for (var iter = 0u; iter < max_iterations; iter = iter + 1u) {
        let mid = f64(0.5) * (lo + hi);
        let f_mid = evaluate_function(mid, problem_idx);
        
        if (abs_f64(f_mid) < tolerance) {
            roots[problem_idx] = mid;
            return;
        }
        
        let f_lo = evaluate_function(lo, problem_idx);
        if (f_lo * f_mid < f64(0.0)) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    
    roots[problem_idx] = f64(0.5) * (lo + hi);
}

fn evaluate_function(mu: f64, idx: u32) -> f64 {
    // BCS particle number: Σ_k v²_k(μ) - N
    var sum = f64(0.0);
    let n_levels = params_per_problem;
    let base = idx * (n_levels + 2u);  // eigenvalues + delta + N
    let delta = params[base + n_levels];
    let target_n = params[base + n_levels + 1u];
    
    for (var k = 0u; k < n_levels; k = k + 1u) {
        let eps_k = params[base + k];
        let diff = eps_k - mu;
        let e_k = sqrt_f64(diff * diff + delta * delta);
        let v2_k = f64(0.5) * (f64(1.0) - diff / e_k);
        sum = sum + v2_k;
    }
    
    return sum - target_n;
}
```

**Location**: `crates/barracuda/src/ops/optimize/batched_bisection_f64.rs`

**Complexity**: Medium (straightforward parallelization of bisection)

---

### 4. GPU Convergence Reduction

**Problem**: SCF convergence check requires max|E_new - E_old| across all nuclei.

**Solution**: Trivial extension of `SumReduceF64` with `abs(a-b)` and `max`.

**API Design**:

```rust
/// Max absolute difference reduction
pub struct MaxAbsDiffF64 {
    device: WgpuDevice,
}

impl MaxAbsDiffF64 {
    /// Returns max|a[i] - b[i]| across all elements
    pub async fn compute(&self, a: &Tensor, b: &Tensor) -> Result<f64, BarracudaError>;
}
```

**WGSL Kernel**: `max_abs_diff_f64.wgsl`

```wgsl
@compute @workgroup_size(256)
fn max_abs_diff(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    var<workgroup> shared: array<f64, 256>;
    
    let idx = gid.x;
    if (idx < n) {
        shared[lid.x] = abs_f64(a[idx] - b[idx]);
    } else {
        shared[lid.x] = f64(0.0);
    }
    
    workgroupBarrier();
    
    // Tree reduction with max
    for (var s = 128u; s > 0u; s = s >> 1u) {
        if (lid.x < s) {
            shared[lid.x] = max(shared[lid.x], shared[lid.x + s]);
        }
        workgroupBarrier();
    }
    
    if (lid.x == 0u) {
        partial_max[gid.x / 256u] = shared[0];
    }
}
```

**Location**: `crates/barracuda/src/ops/reduce/max_abs_diff_f64.rs`

**Complexity**: Low (simple variation of sum reduction)

---

### 5. Persistent Buffer Management

**Problem**: Current pattern re-creates buffers per dispatch. SCF runs 100-200
iterations — buffer allocation overhead adds up.

**Solution**: "Pin" buffers at solver start, reuse across iterations, release at end.

**API Design**:

```rust
impl BufferPool {
    /// Pin a set of named buffers for the lifetime of a solver
    pub fn pin_solver_buffers(
        &self,
        solver_id: &str,
        buffers: &[(&str, BufferDescriptor)],
    ) -> Result<SolverBufferSet, BarracudaError>;
    
    /// Release all buffers associated with a solver
    pub fn release_solver_buffers(&self, solver_id: &str);
}

pub struct SolverBufferSet {
    solver_id: String,
    buffers: HashMap<String, Arc<wgpu::Buffer>>,
}

impl SolverBufferSet {
    pub fn get(&self, name: &str) -> Option<&wgpu::Buffer>;
}

// Usage:
let buffers = pool.pin_solver_buffers("hfb_scf", &[
    ("hamiltonian", BufferDescriptor::new(batch * n * n * 8)),
    ("eigenvalues", BufferDescriptor::new(batch * n * 8)),
    ("eigenvectors", BufferDescriptor::new(batch * n * n * 8)),
    ("density", BufferDescriptor::new(batch * grid * 8)),
])?;

for iteration in 0..max_iterations {
    // Use buffers.get("hamiltonian") etc — no allocation
}

pool.release_solver_buffers("hfb_scf");
```

**Location**: Extend `crates/barracuda/src/device/tensor_context.rs`

**Complexity**: Low-Medium (lifetime tracking, but straightforward)

---

## Priority Order

| # | Item | Complexity | Impact | Dependencies |
|:-:|------|:----------:|:------:|:------------:|
| 1 | Max Abs Diff Reduction | Low | Medium | None |
| 2 | Persistent Buffer Management | Low-Med | High | None |
| 3 | Batched Bisection | Medium | High | None |
| 4 | Grid Quadrature GEMM | Medium | High | None |
| 5 | Multi-Kernel Pipeline | Med-High | Critical | Items 1-4 |

Items 1-4 can be implemented in parallel. Item 5 ties them together.

---

## Success Criteria

| Metric | Current | Target |
|--------|:-------:|:------:|
| CPU↔GPU round-trips per iteration | ~10 | 1 (convergence check only) |
| Buffer allocations per iteration | ~20 | 0 |
| GPU wall time (791 nuclei, 100 iter) | 40.9 min | ~40s |
| GPU vs CPU speedup | 0.014× (70× slower) | 1.2× faster |

---

## References

- hotSpring Experiment 005 (L2 mega-batch)
- `docs/planning/HOTSPRING_ABSORPTION_FEB15_2026.md` — Previous absorption
- `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md` — MD pipeline
- `specs/BARRACUDA_PARITY_ROADMAP.md` — Performance architecture

---

*Created: February 16, 2026*
