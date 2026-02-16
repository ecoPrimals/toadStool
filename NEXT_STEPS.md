# ToadStool/BarraCUDA — Next Steps

**Updated**: February 16, 2026  
**Focus**: GPU-Resident Physics Pipeline (from hotSpring Exp 005)

---

## The Problem

hotSpring's L2 mega-batch experiment achieved 95% GPU utilization but CPU was **70× faster**:

| Metric | GPU | CPU |
|--------|:---:|:---:|
| Wall time | 40.9 min | 35 sec |
| Slowdown | **70×** | — |

**Root cause**: Amdahl's Law. The eigensolve is only 1% of the SCF iteration.
The other 99% (Hamiltonian, BCS pairing, density updates) runs on CPU with
GPU↔CPU round-trips between every step.

**The fix**: Move ALL physics to GPU. Zero round-trips during iteration.

---

## Implementation Checklist

### Phase 1: Reduction & Buffers (Low Complexity)

- [ ] **Max Abs Diff Reduction** (`max_abs_diff_f64.wgsl`)
  - `max|a[i] - b[i]|` for SCF convergence check
  - Location: `crates/barracuda/src/ops/reduce/`
  - API: `MaxAbsDiffF64::compute(&a, &b) -> f64`
  - ~2 hours

- [ ] **Persistent Buffer Management**
  - Pin buffers at solver start, reuse across iterations
  - Extend: `crates/barracuda/src/device/tensor_context.rs`
  - API: `BufferPool::pin_solver_buffers()`, `release_solver_buffers()`
  - ~3 hours

### Phase 2: Physics Kernels (Medium Complexity)

- [ ] **Batched Bisection** (`batched_bisection_f64.wgsl`)
  - 1D root-finding for BCS chemical potential
  - Location: `crates/barracuda/src/ops/optimize/`
  - API: `BatchedBisection::solve(lower, upper, params, function_id)`
  - Built-in: `BisectionFunction::BcsParticleNumber`
  - ~4 hours

- [ ] **Grid Quadrature GEMM** (`grid_quadrature_gemm_f64.wgsl`)
  - Batched: `H[b,i,j] = Σ_k φ[b,i,k] * W[b,k] * φ[b,j,k] * weights[k]`
  - Location: `crates/barracuda/src/ops/physics/`
  - API: `GridQuadratureGemm::execute(phi, w, quad_weights)`
  - ~4 hours

### Phase 3: Pipeline (Medium-High Complexity)

- [ ] **Multi-Kernel Pipeline**
  - Chain ops: H-build → eigensolve → BCS → density (GPU buffers only)
  - Extend: `crates/barracuda/src/pipeline/` or `device/tensor_context.rs`
  - API: `PipelineBuilder::add_stage().build()` or extend `begin_batch()`
  - Requires: Items from Phase 1 & 2
  - ~6 hours

### Testing

- [ ] Unit tests for each new op
- [ ] E2E test: Full SCF iteration on GPU
- [ ] Benchmark: GPU vs CPU timing comparison

---

## Quick Reference

```rust
// Max Abs Diff (Phase 1)
let max_diff = MaxAbsDiffF64::new(&device)?;
let converged = max_diff.compute(&e_new, &e_old).await? < 1e-10;

// Persistent Buffers (Phase 1)
let buffers = pool.pin_solver_buffers("hfb_scf", &[
    ("hamiltonian", BufferDescriptor::new(batch * n * n * 8)),
    ("eigenvalues", BufferDescriptor::new(batch * n * 8)),
])?;
// ... use buffers across 100+ iterations ...
pool.release_solver_buffers("hfb_scf");

// Batched Bisection (Phase 2)
let bisect = BatchedBisection::new(&device, 64, 1e-12)?;
let mu = bisect.solve(&lower, &upper, &params, BcsParticleNumber).await?;

// Grid Quadrature GEMM (Phase 2)
let h_build = GridQuadratureGemm::new(&device, batch, n, grid_size)?;
let h = h_build.execute(&phi, &potential, &quad_weights).await?;

// Multi-Kernel Pipeline (Phase 3)
let pipeline = PipelineBuilder::new(&device)
    .add_stage("hamiltonian", h_build_shader, &[phi_buf], &[h_buf])
    .add_stage("eigensolve", eigh_shader, &[h_buf], &[evals, evecs])
    .add_stage("bcs", bcs_shader, &[evals], &[occs])
    .add_stage("density", density_shader, &[evecs, occs], &[rho])
    .build()?;
pipeline.execute().await?;  // Single GPU submit, no CPU round-trips
```

---

## Success Criteria

| Metric | Current | Target |
|--------|:-------:|:------:|
| CPU↔GPU round-trips/iteration | ~10 | 1 |
| Buffer allocs/iteration | ~20 | 0 |
| GPU wall time (791 nuclei, 100 iter) | 40.9 min | ~40s |
| GPU vs CPU speedup | 0.014× | 1.2× |

---

## Files Changed

| Phase | File | Action |
|:-----:|------|--------|
| 1 | `crates/barracuda/src/ops/reduce/max_abs_diff_f64.rs` | Create |
| 1 | `crates/barracuda/src/shaders/reduce/max_abs_diff_f64.wgsl` | Create |
| 1 | `crates/barracuda/src/device/tensor_context.rs` | Extend |
| 2 | `crates/barracuda/src/ops/optimize/batched_bisection_f64.rs` | Create |
| 2 | `crates/barracuda/src/shaders/optimize/batched_bisection_f64.wgsl` | Create |
| 2 | `crates/barracuda/src/ops/physics/grid_quadrature_gemm_f64.rs` | Create |
| 2 | `crates/barracuda/src/shaders/physics/grid_quadrature_gemm_f64.wgsl` | Create |
| 3 | `crates/barracuda/src/pipeline/mod.rs` | Create |

---

## Detailed Planning

See `docs/planning/GPU_RESIDENT_PIPELINE_FEB16_2026.md` for:
- Full API designs with WGSL kernel implementations
- Complexity boundary analysis (CPU wins n<30, GPU wins n>50)
- hotSpring experiment findings

---

## Previous Absorption (Complete)

From Feb 14-15 hotSpring handoffs:

- [x] MD pipeline (Yukawa, thermostats, PPPM, observables)
- [x] Math primitives (Hermite, Laguerre, Broyden, FD gradients)
- [x] Science buffer limits (512 MiB storage, 1 GiB total)
- [x] 47 new tests (unit, E2E, chaos, fault)
- [x] All clippy warnings fixed

---

*From the ToadStool evolution desk*
