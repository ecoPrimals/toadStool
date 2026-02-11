# hotSpring Scientific Computing Extraction Plan

**Date**: February 11, 2026  
**Status**: Investigation Complete — Ready for Implementation  
**Priority**: High (required for L2 accuracy parity)

---

## Executive Summary

hotSpring Python code has been analyzed. Key scientific computing functions identified for extraction into `barracuda` middleware. Primary goal: **achieve L2 accuracy parity** (currently χ²/datum=25.43, target χ²/datum=1.93).

**Root Cause of L2 Gap**: Python uses `mystic.SparsitySampler` for space-filling sampling. BarraCUDA L2 currently uses naive sampling, leading to 13× accuracy gap despite 1.7× throughput advantage.

---

## Key Findings from hotSpring

### 1. SparsitySampler (🔴 HIGHEST PRIORITY)

**Location**: `mystic.samplers.SparsitySampler` (external dependency)  
**Usage**: `control/surrogate/scripts/full_iterative_workflow.py:216`

**What it does**:
```python
sampler = SparsitySampler(bounds, objective, npts=16, solver=NelderMeadSimplexSolver)
sampler.sample_until(iters=1000)  # Runs npts solvers in parallel
```

- Launches `npts` parallel Nelder-Mead simplex solvers
- Each solver explores different regions of parameter space
- `sample_until(N)` runs until N total function evaluations accumulated
- Captures **ALL** evaluations, not just best solutions
- Results are space-filling AND guided by gradient descent

**Impact**: L2 paper used SparsitySampler to achieve χ²=1.93. Our naive sampling gave χ²=25.43.

**Implementation Strategy**:
- **Option A**: Port `mystic.SparsitySampler` algorithm to Rust
- **Option B**: Implement multi-start Nelder-Mead with maximin distance sampling
- **Option C**: Hybrid: Latin Hypercube + multi-start Nelder-Mead

Recommend **Option B** initially (simpler, proven), then evolve to full SparsitySampler port if needed.

---

### 2. Latin Hypercube Sampling (🟡 HIGH PRIORITY)

**Location**: `control/surrogate/scripts/run_benchmark_functions.py:108-120`

**Implementation** (Python):
```python
def latin_hypercube_sampling(func, bounds, n_samples):
    ndim = len(bounds)
    samples = np.zeros((n_samples, ndim))
    for d in range(ndim):
        lo, hi = bounds[d]
        perm = np.random.permutation(n_samples)
        intervals = np.linspace(lo, hi, n_samples + 1)
        for i in range(n_samples):
            samples[perm[i], d] = np.random.uniform(intervals[i], intervals[i+1])
    return samples
```

**Algorithm**:
1. Divide each dimension into `n_samples` equal intervals
2. Randomly permute the intervals for each dimension
3. Sample one point per interval (uniformly within interval)
4. Result: space-filling design with one sample per "row" and "column"

**Rust Implementation**:
```rust
pub fn latin_hypercube(
    n_samples: usize,
    bounds: &[(f64, f64)],
    rng: &mut impl Rng,
) -> Result<Vec<Vec<f64>>>
```

**Status**: Already in handoff (item 8), now we have reference implementation.

---

### 3. GPU-Accelerated RBF (🟢 MEDIUM PRIORITY)

**Location**: `control/surrogate/nuclear-eos/wrapper/gpu_accel.py:497-551`

**Implementation** (Python/PyTorch):
```python
class GPURBFInterpolator:
    def __init__(self, X, y, kernel='thin_plate_spline'):
        # X: (N, D) training points
        # y: (N,) function values
        K = self._kernel_matrix(X, X)  # (N, N) - on GPU
        P = [ones, X]  # polynomial terms
        A = [K | P; P^T | 0]
        coeffs = torch.linalg.solve(A, [y; 0])  # GPU linear solve
    
    def _kernel_matrix(self, X1, X2):
        r = cdist(X1, X2)  # pairwise distances
        return r**2 * torch.log(torch.clamp(r, min=1e-20))
```

**Key Operations**:
1. Pairwise distance matrix: `cdist(X1, X2)` — already have `cdist.wgsl`
2. Kernel evaluation: `r² log(r)` — elementwise on GPU
3. Linear solve: `torch.linalg.solve` — already have `linsolve.wgsl` (but f32, need f64)

**Speedup**: hotSpring saw ~14× training speedup for N=3000+ points.

**Implementation Strategy**:
- Wire up existing `cdist.wgsl` shader (already in barracuda)
- Implement f64 variant: `cdist_f64.wgsl`
- Create `RBFSurrogate::train_gpu()` method using WGSL shaders
- Fallback to CPU for small N (<1000)

**Status**: Already in handoff (item 12, medium priority). Now we have reference.

---

### 4. Multi-start Nelder-Mead (🟡 HIGH PRIORITY)

**Location**: Implicit in SparsitySampler (uses `npts` parallel solvers)

**Algorithm**:
```rust
pub fn multi_start_nelder_mead<F>(
    f: F,
    bounds: &[(f64, f64)],
    n_starts: usize,
    max_iter: usize,
    tol: f64,
    initial_sampler: InitialSampler,  // LHS or Random
) -> Result<(Vec<f64>, f64, Vec<EvaluationRecord>)>
where F: Fn(&[f64]) -> f64
{
    // 1. Generate n_starts initial guesses (LHS or random)
    let starts = initial_sampler.sample(n_starts, bounds);
    
    // 2. Run Nelder-Mead from each start
    let mut all_evals = Vec::new();
    let mut best = (Vec::new(), f64::INFINITY);
    
    for x0 in starts {
        let (x_opt, f_opt, n_evals) = nelder_mead(f, &x0, bounds, max_iter, tol)?;
        all_evals.push(EvaluationRecord { x: x_opt.clone(), f: f_opt });
        if f_opt < best.1 {
            best = (x_opt, f_opt);
        }
    }
    
    // 3. Return best solution + ALL evaluations (for RBF training)
    Ok((best.0, best.1, all_evals))
}
```

**Key Insight**: SparsitySampler returns ALL evaluations, not just best. This is critical for RBF surrogate training.

**Status**: Already in handoff (item 14, high priority). Now we have usage pattern.

---

## Implementation Roadmap

### Phase 2A: Core Sampling (Highest Priority)

**Goal**: Close the L2 accuracy gap (χ²=25.43 → χ²<5)

1. **Latin Hypercube Sampling** (2 hours)
   - New module: `barracuda::sample::lhs`
   - Function: `latin_hypercube(n_samples, bounds, rng) -> Vec<Vec<f64>>`
   - Tests: 2D space-filling property, maximin distance vs random

2. **Multi-start Nelder-Mead** (3 hours)
   - New module: `barracuda::optimize::multi_start`
   - Function: `multi_start_nelder_mead(f, bounds, n_starts, ...)`
   - Returns: `(best_x, best_f, all_evaluations)`
   - Tests: Rosenbrock, Rastrigin with multiple local minima

3. **Evaluation Record System** (1 hour)
   - Struct: `EvaluationRecord { x: Vec<f64>, f: f64 }`
   - Accumulator for ALL evaluations during optimization
   - Required for RBF surrogate training

**Deliverable**: `barracuda::optimize::multi_start_nelder_mead()` that replicates SparsitySampler's space-filling + gradient-guided exploration.

**Expected Impact**: L2 χ²/datum reduction from 25.43 to ~5-10 (2-5× improvement).

---

### Phase 2B: SparsitySampler Port (High Priority, if Phase 2A insufficient)

**Goal**: Full parity with Python L2 (χ²<2)

1. **SparsitySampler Algorithm** (8 hours)
   - New module: `barracuda::sample::sparsity`
   - Struct: `SparsitySampler { solvers: Vec<NelderMead>, cache: Vec<Eval> }`
   - Method: `sample_until(&mut self, n_evals: usize) -> Vec<Eval>`
   - Parallel execution using `rayon`

2. **Maximin Distance Sampling** (2 hours)
   - Select initial points to maximize minimum pairwise distance
   - Algorithm: greedy sequential maximin or Latin Hypercube

3. **Solver State Management** (2 hours)
   - Each solver maintains independent state
   - Resume/pause capability for `sample_until()`
   - Cache ALL evaluations across solvers

**Deliverable**: Full `SparsitySampler` port matching mystic behavior.

**Expected Impact**: L2 χ²/datum parity with Python (χ²≈1.93).

---

### Phase 2C: GPU Acceleration (Medium Priority)

**Goal**: 14× RBF training speedup for large N

1. **GPU Pairwise Distance** (2 hours)
   - Wire up existing `cdist.wgsl` to `RBFSurrogate`
   - Add f64 variant: `cdist_f64.wgsl`

2. **GPU RBF Training** (3 hours)
   - `RBFSurrogate::train_gpu()` method
   - Kernel matrix on GPU
   - Linear solve using `linsolve_f64.wgsl`
   - Transfer only final coefficients back to CPU

3. **Adaptive Dispatch** (1 hour)
   - Auto-select GPU for N > 1000
   - CPU for N < 1000 (kernel matrix overhead dominates)

**Deliverable**: `RBFSurrogate::train()` with automatic GPU dispatch.

**Expected Impact**: 14× speedup for L3+ surrogates (N=3000+).

---

## Testing Strategy

### Unit Tests

**Latin Hypercube**:
- 2D space-filling: check one sample per row/column
- Bounds respected: all samples within specified bounds
- Maximin distance: LHS > random sampling

**Multi-start Nelder-Mead**:
- Rosenbrock 2D: find global minimum from 10 starts
- Rastrigin 2D: handle multiple local minima
- Evaluation accumulation: verify ALL evals captured

**SparsitySampler** (Phase 2B):
- sample_until(1000): verify 1000 evaluations accumulated
- Parallel solvers: check state independence
- Space-filling: maximin distance > multi-start NM

### Integration Tests

**L2 Nuclear EOS Reproduction**:
```rust
// Reproduce hotSpring L2 result
let objective = NuclearEOSObjective::new();
let bounds = load_skyrme_bounds();

// Phase 2A: Multi-start NM
let (x_best, chi2, evals) = multi_start_nelder_mead(
    |x| objective.chi2(x),
    &bounds,
    100,  // n_starts
    1000, // max_iter per start
    1e-8,
);

// Train RBF on ALL evaluations
let surrogate = RBFSurrogate::train_from_evals(&evals)?;

// Expected: chi2 < 5 (Phase 2A), chi2 < 2 (Phase 2B)
assert!(chi2 < 5.0);
```

---

## File Structure

```
crates/barracuda/src/
├── sample/
│   ├── mod.rs             -- Sample module exports
│   ├── lhs.rs             -- Latin Hypercube Sampling
│   └── sparsity.rs        -- SparsitySampler (Phase 2B)
├── optimize/
│   ├── mod.rs             -- Update exports
│   ├── multi_start.rs     -- Multi-start Nelder-Mead
│   └── eval_record.rs     -- EvaluationRecord system
├── surrogate/
│   ├── mod.rs             -- Update exports
│   └── rbf_gpu.rs         -- GPU-accelerated RBF (Phase 2C)
└── lib.rs                 -- Add `pub mod sample;`
```

---

## Dependencies

**New** (all pure Rust):
- None! All algorithms are self-contained.

**Existing**:
- `rayon` — parallel solver execution (SparsitySampler)
- `nalgebra` — linear algebra (already in use)
- `rand` — RNG for sampling

---

## Timeline Estimate

| Phase | Tasks | Estimated Time | Priority |
|-------|-------|---------------|----------|
| **2A** | LHS + Multi-start NM + Eval Records | **6 hours** | 🔴 Highest |
| **2B** | Full SparsitySampler port | **12 hours** | 🟡 High (if 2A insufficient) |
| **2C** | GPU RBF acceleration | **6 hours** | 🟢 Medium |
| **Total** | | **24 hours** (all phases) | |

**Recommendation**: Implement Phase 2A first (6 hours), test against L2 objective. If χ²/datum < 5, ship it. If not, proceed to Phase 2B.

---

## Success Metrics

| Metric | Baseline | Phase 2A Target | Phase 2B Target | Phase 2C Target |
|--------|----------|-----------------|-----------------|-----------------|
| L2 χ²/datum | 25.43 | < 5.0 | < 2.0 | < 2.0 |
| L2 throughput | 0.48 eval/s | 0.48 eval/s | 0.40 eval/s | 1.0 eval/s |
| Accuracy vs Python | 13× worse | 2-3× worse | Parity | Parity |
| Training time (N=3000) | 4.2s | 4.2s | 4.2s | 0.3s (14×) |

---

## References

1. **hotSpring L2 Results**: `hotSpring/README.md` lines 38-49
   - Python L2: χ²=1.93, 3008 evals, 3.2h
   - BarraCUDA L2: χ²=25.43, 1009 evals, 35min (1.7× faster, 13× less accurate)

2. **SparsitySampler Usage**: `hotSpring/control/surrogate/scripts/full_iterative_workflow.py`
   - Line 216: `sampler = SparsitySampler(bounds, cached, npts=16, solver=solver)`
   - Line 230: `sampler.sample_until(iters=target_evals)`

3. **LHS Implementation**: `hotSpring/control/surrogate/scripts/run_benchmark_functions.py:108-120`

4. **GPU RBF**: `hotSpring/control/surrogate/nuclear-eos/wrapper/gpu_accel.py:497-551`

5. **Paper**: Diaw et al. (2024) "Efficient learning of accurate surrogates for simulations of complex systems"  
   *Nature Machine Intelligence*, doi:10.1038/s42256-024-00839-1

---

**Next Steps**: Proceed with Phase 2A implementation (Latin Hypercube + Multi-start Nelder-Mead).
