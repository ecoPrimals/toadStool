# BarraCuda Evolution Guide — Lessons from hotSpring, GPU-Ready Roadmap

**Date**: February 13, 2026
**From**: ecoPrimals Control Team (Eastgate) — hotSpring validation
**To**: ToadStool / BarraCuda Team
**Context**: Titan V incoming (~1 week), 87%/74% theoretical max on AMD/NVIDIA via pure wgpu
**Purpose**: Map everything we learned from science workloads to the next evolution of BarraCuda

---

## 1. What We Learned: The Complete Picture

### 1.1 The Validation Journey

We used nuclear physics (Skyrme EDF parameter optimization) as a demanding, real-world scientific workload to validate BarraCuda against Python/SciPy. This is not a toy benchmark — it's the same math that nuclear physicists run daily.

```
                        BarraCuda Performance vs Python/SciPy
   ┌──────────────────────────────────────────────────────────────┐
   │ Level 1 (SEMF):   0.80 chi²  vs  6.62  → 8.3× BETTER     │
   │ Level 2 (HFB):   16.11 chi²  vs 61.87  → 3.8× BETTER     │
   │ Throughput (L1):  0.44s/64ev  vs 180s/1008ev → 400× FASTER │
   │ Dependencies:     0 external  vs scipy+numpy+mystic         │
   └──────────────────────────────────────────────────────────────┘
```

### 1.2 What Worked (validated functions)

These BarraCuda functions have been battle-tested against SciPy on real science:

| Function | What It Does | Science Validation | Ready for GPU? |
|----------|-------------|-------------------|----------------|
| `eigh_f64` | Symmetric eigendecomposition (Jacobi) | HFB Hamiltonian diag, 20-50 dim | **HIGH PRIORITY** — WGSL `eigh.wgsl` exists |
| `brent` | Root-finding to machine precision | BCS chemical potential (1e-10 tol) | Low — scalar, CPU-bound |
| `gradient_1d` | 2nd-order finite differences | HFB wavefunction derivatives | **YES** — embarrassingly parallel |
| `trapz` | Trapezoidal integration | All radial integrals, density normalization | **YES** — reduction pattern |
| `gamma` | Gamma function | HO wavefunction normalization | YES — element-wise |
| `laguerre` | Generalized Laguerre polynomials | Radial wavefunctions | YES — element-wise |
| `latin_hypercube` | Space-filling sampling | Initial parameter space coverage | Low — one-shot |
| `direct_sampler` | Round-based Nelder-Mead | Core optimizer (L1 and L2) | MEDIUM — parallel starts |
| `chi2_decomposed_weighted` | Per-datum chi² analysis | Goodness-of-fit testing | Low — post-processing |
| `bootstrap_ci` | Bootstrap confidence intervals | Uncertainty quantification | MEDIUM — many resamples |

### 1.3 What Broke and Why

| Issue | Root Cause | Impact | Lesson |
|-------|-----------|--------|--------|
| `gradient_1d` boundary stencil | 1st-order instead of 2nd-order | ~65 MeV systematic offset | **Numerical precision at boundaries matters for SCF** |
| SparsitySampler overfitting | `smoothing: 1e-12` default | 40.83 vs 0.80 chi² (51× worse than DirectSampler) | **Default hyperparameters must be safe for rugged landscapes** |
| nalgebra eigensolver mismatch | Different eigenvalue ordering | Small but systematic energy shift | **Eigensolver output conventions must be documented** |
| BCS bisection precision | 1e-6 vs Brent's 1e-10 | Occupation number errors accumulate in SCF | **Root-finding precision propagates through iterative loops** |

### 1.4 Performance Bottleneck Analysis

Where the CPU time goes in the science workload:

```
L2 HFB Evaluation (~55 seconds per parameter set, 52 nuclei):
├── HFB SCF iterations (per nucleus): ~85% of time
│   ├── Build Hamiltonian H              5%  ← matrix element integrals
│   ├── Diagonalize H (eigh_f64)        25%  ← JACOBI ITERATIONS
│   ├── BCS occupations (brent)          5%  ← scalar root-finding
│   ├── Compute densities               20%  ← wavefunction summation
│   ├── Coulomb (Poisson + Slater)      15%  ← cumulative sums
│   └── Skyrme potential                15%  ← density-dependent terms
├── Parallelism overhead (rayon):        5%
└── Data movement:                      10%
```

**GPU acceleration targets** (ordered by impact × feasibility):

1. **Density computation** (20%): Sum `|psi_i(r)|² * occupation_i` over all states → WGSL parallel reduction
2. **Eigendecomposition** (25%): Jacobi iterations → WGSL `eigh.wgsl` already exists, needs batched mode
3. **Potential construction** (30%): Element-wise on radial grid → trivially parallel
4. **Matrix element integrals** (5%): `<i|V|j>` → `trapz` of products → parallel reduction

---

## 2. GPU Strategy for Titan V

### 2.1 Hardware Profile

```
NVIDIA Titan V (GV100):
├── Compute: 5120 CUDA cores, 640 Tensor cores
├── Memory: 12 GB HBM2, 652 GB/s bandwidth
├── FP64: 7.4 TFLOPS (science needs double precision!)
├── FP32: 14.9 TFLOPS
├── Architecture: Volta (sm_70)
└── wgpu backend: Vulkan
```

Key advantage: **FP64 is 1/2 FP32** on Volta (best ratio in consumer GPUs). Most consumer GPUs (Turing, Ampere, Ada) have FP64 = 1/32 or 1/64 of FP32. For scientific computing at double precision, the Titan V is uniquely powerful.

**UPDATE (Feb 13, 2026)**: Our validation shows consumer GPUs are MUCH better than spec:
- RTX 3090: fp64/fp32 = 2.16x (not 32x!)
- RX 6950 XT: fp64/fp32 = 1.33x (not 16x!)

The 1:32 ratio is CUDA/driver throttling that wgpu/Vulkan bypasses. Titan V will be even faster.

### 2.2 Immediate GPU Wins (Week 1 with Titan V)

#### Win 1: Batched HFB Evaluation
Currently: 52 nuclei evaluated via rayon (CPU). Each nucleus is independent.
GPU path: Dispatch all 52 nucleus HFB evaluations as GPU workgroups.

```
Current (CPU, rayon):     52 nuclei × 0.7s/nucleus ÷ 24 threads = ~1.5s
Titan V (batched GPU):    52 nuclei × all grid ops on GPU        = ~0.1s (est.)
Expected speedup: ~15×
```

#### Win 2: Batched Eigendecomposition
Currently: `eigh_f64` runs Jacobi on a single matrix sequentially.
GPU path: `eigh.wgsl` already exists. Need batched mode: solve 52 independent ~30×30 systems simultaneously.

```
Current: 52 × eigh(30×30) sequential = ~0.5s
Titan V: 52 × eigh(30×30) batched    = ~0.01s (est., memory-bound)
```

#### Win 3: Density Grid Operations
All the element-wise operations on the radial grid (120-200 points):
- Coulomb integration (cumulative sum)
- Skyrme potential (element-wise arithmetic)
- Wavefunction products
- Density summation

These are classic GPU workloads. The WGSL shaders for these patterns already exist (`reduce/`, `math/`, `tensor/`).

### 2.3 Medium-Term GPU Wins (Post-Titan V)

#### Win 4: L3 Deformed HFB on GPU
The L3 solver operates on a 2D cylindrical grid (~60×80 = 4800 points) with 220 basis states.
- **Density computation**: Sum `|psi(rho,z)|² * occ` for 220 states on 4800 grid → **4800 × 220 = 1M operations → perfect for GPU**
- **Potential construction**: 4800 element-wise evaluations → trivially parallel
- **Wavefunction evaluation**: 220 × 4800 = 1M Hermite/Laguerre evaluations → WGSL `special/hermite.wgsl` and `special/laguerre.wgsl` already exist!

```
Current L3 (CPU): 1948s for 52 nuclei (2811× L2)
Titan V L3 (est): ~50-100s for 52 nuclei (50-100× L2)
Expected speedup: 20-40×
```

#### Win 5: Optimization on GPU
- Multiple NM starts running in parallel on GPU
- SparsitySampler RBF evaluation → matmul pattern
- Bootstrap resampling → embarrassingly parallel

### 2.4 Dispatch Thresholds for Science

Current dispatch thresholds in `barracuda::dispatch::config`:

| Operation | Current Threshold | Science Workload Size | Recommendation |
|-----------|------------------|----------------------|----------------|
| eigh | (not set) | 30×30 to 220×220 | GPU for N>16 (batched) |
| matmul | 64 | 30×30 | GPU when batched (>8 matrices) |
| gradient_1d | (not set) | 120-200 points | CPU (too small alone, GPU when batched) |
| trapz | (not set) | 120-200 points | CPU (too small alone) |
| reduce_sum | 4096 | 120-4800 per grid | GPU for L3 (4800) |
| element-wise | 512 | 120-4800 per grid | GPU for L3 |

**Key insight**: Individual operations on the radial grid (120 points) are too small for GPU. But when **batched** across 52 nuclei × 30 states × 200 grid points = 312,000 elements, GPU wins decisively. The dispatch system needs a **batched mode**.

---

## 3. Specific Evolution Targets

### 3.1 MUST DO (before Titan V arrives)

| # | Target | File | Impact | Effort |
|---|--------|------|--------|--------|
| 1 | **Commit `gradient_1d` 2nd-order fix** | `numerical/gradient.rs` | Already applied, needs git commit | 5 min |
| 2 | **Change SparsitySampler defaults** | `sample/sparsity.rs:166` | `auto_smoothing: true` default | 10 min |
| 3 | **Add `n_explore_solvers` config** | `sample/sparsity.rs` | Configurable hybrid evaluation | 1 hr |
| 4 | **Wire `parallel` feature in Cargo.toml** | `Cargo.toml` | Cascade pipeline compiles | 5 min |
| 5 | **Fill benchmark stubs** | `dispatch/benchmark.rs` | Real perf data for dispatch decisions | 1 day |

### 3.2 GPU Evolution (with Titan V)

| # | Target | Description | Priority |
|---|--------|-------------|----------|
| 6 | **Batched `eigh_f64`** | Solve N independent eigenvalue problems simultaneously on GPU | **CRITICAL** |
| 7 | **Batched `gradient_1d`** | N arrays of M points, all at once | HIGH |
| 8 | **Batched `trapz`** | N integrals simultaneously | HIGH |
| 9 | **GPU density accumulator** | Sum `occ_i * |psi_i(r)|²` for all states at all grid points | HIGH |
| 10 | **2D grid operations** | `trapz_2d`, `gradient_2d` for L3 deformed | MEDIUM |
| 11 | **Batched `laguerre` + `gamma`** | Evaluate special functions on entire grid at once | MEDIUM |
| 12 | **GPU-accelerated NM starts** | Run multiple Nelder-Mead solvers on GPU simultaneously | MEDIUM |

### 3.3 Science Module Evolution (for team)

| # | Target | Description | hotSpring provides |
|---|--------|-------------|-------------------|
| 13 | `trapz_2d(f, dx, dy)` | 2D trapezoidal integration | Reference impl in `hfb_deformed.rs` |
| 14 | `gradient_2d(f, dx, dy)` | 2D finite differences, 2nd-order at boundaries | Pattern from `gradient_1d` fix |
| 15 | `poisson_2d_cylindrical` | Coulomb solver in (rho, z) | Physics reference in handoff |
| 16 | `broyden_mixer` | Good Broyden mixing for SCF convergence | Needed for L3 deformed |
| 17 | `block_eigh` | Block-diagonal eigensolver | L3 Omega-block structure |
| 18 | `augmented_lagrangian` | Constrained optimization | L3 deformation constraint |

---

## 4. The Benchmarking Story

### 4.1 What 87% AMD / 74% NVIDIA Means

BarraCuda achieves 87% and 74% of theoretical max throughput on AMD and NVIDIA via pure wgpu. This means the shader dispatch and memory management are excellent. But the science workload reveals a different bottleneck:

**The science bottleneck is not compute, it's latency.**

```
Science workload pattern:
  for each SCF iteration:          ← SERIAL (cannot parallelize)
    for each nucleus:              ← PARALLEL ✓
      build H matrix               ← small matrix, memory-bound
      diag H (eigh)               ← compute-bound but small
      compute occupations          ← scalar (brent)
      compute densities            ← PARALLEL ✓ (best GPU target)
      compute potentials           ← PARALLEL ✓ (best GPU target)
    convergence check              ← SERIAL
```

The serial SCF loop means we can't just throw everything at the GPU. We need **latency-optimized dispatch** — keep data on GPU across SCF iterations, minimize host-device transfers.

### 4.2 GPU Memory Layout for Science

```
GPU Memory (12 GB HBM2 on Titan V):
├── Static (loaded once):
│   ├── Wavefunctions: 52 nuclei × 30 states × 200 grid = 312K f64 = 2.4 MB
│   ├── Grid coordinates: 52 × 200 × 2 = 0.2 MB
│   └── Parameters: 10 f64 = 80 bytes
├── Dynamic (per SCF iteration):
│   ├── Densities: 52 × 200 × 2 (p/n) = 0.2 MB
│   ├── Potentials: 52 × 200 × 2 = 0.2 MB
│   └── Hamiltonians: 52 × 30 × 30 = 0.4 MB
└── Total: ~4 MB (0.03% of 12 GB!)
```

The Titan V has **3,000× more memory** than we need for L2. For L3 with 2D grids: 52 × 4800 × 220 = 55M floats = 420 MB — still only 3.5% of GPU memory. We could fit the entire optimization loop (thousands of parameter evaluations) on-GPU.

### 4.3 Recommended Benchmarks for Science Workloads

The `dispatch/benchmark.rs` stubs should be filled with these specific benchmarks:

```rust
// 1. Batched eigendecomposition
benchmark_batched_eigh(n_matrices: 52, dim: 30);  // L2 spherical
benchmark_batched_eigh(n_matrices: 52, dim: 220); // L3 deformed

// 2. Batched density accumulation (sum |psi|² * occ)
benchmark_density_accumulation(n_nuclei: 52, n_states: 30, n_grid: 200);

// 3. Element-wise potential evaluation
benchmark_potential_evaluation(n_nuclei: 52, n_grid: 200);

// 4. SCF iteration (full cycle, including data transfer)
benchmark_scf_iteration_gpu_vs_cpu(n_nuclei: 52, n_states: 30, n_grid: 200);

// 5. Optimization objective evaluation
benchmark_objective_evaluation(n_nuclei: 52);
```

---

## 5. The Parity Ladder

### 5.1 Where We Are

```
                    Paper Target
                         ↑
                    L4: ~10⁻⁶ relative (keV RMS)     ← beyond-mean-field
                         ↑
                    L3: ~10⁻⁵ relative (0.1 MeV)     ← deformation [architecture built]
                         ↑
              ┌─── L2 floor: ~10⁻⁴ (0.5 MeV RMS)   ← achievable with GPU budget
              │          ↑
    ★ HERE ★  │    L2 current: ~10⁻² (30 MeV RMS)   ← optimizer-limited
              │          ↑
              └─── Python/SciPy: 61.87 chi²           ← ALREADY BEATEN
                         ↑
                    Initial: 28,450 chi²              ← starting point
```

### 5.2 What GPU Budget Buys Us

| Scenario | L2 Evals | Expected chi² | Time (CPU 24-core) | Time (Titan V est.) |
|----------|----------|--------------|-------------------|---------------------|
| Current | 60 | 16.11 | 3208s | ~200s |
| 10× budget | 600 | ~5-8 | ~32,000s (9hr) | ~2,000s (33min) |
| 100× budget | 6000 | ~1-3 | infeasible | ~20,000s (5.5hr) |
| Paper-level | ~50,000 | ~0.5 (L2 floor) | infeasible | ~2 days |

The Titan V makes the 10× budget run feasible in under an hour. This alone could push us from chi²=16 to chi²<5, which is **12× better than Python**.

### 5.3 Multi-Workload Validation Plan

Once the Titan V is available, validate BarraCuda on additional science workloads:

| Workload | Math Profile | GPU Opportunity | hotSpring Module |
|----------|-------------|-----------------|-----------------|
| Nuclear EOS (current) | eigh, brent, trapz, gradient | Batched SCF | `physics/hfb.rs` |
| Molecular dynamics | Force computation, integration | Particle parallelism | future |
| PDE solvers | Crank-Nicolson, finite elements | Grid parallelism | `pde/` shaders exist |
| Surrogate modeling | RBF, kernel evaluation | Matrix operations | `sample/sparsity.rs` |
| Bayesian optimization | GP prediction, acquisition | Cholesky, matmul | `linalg/` shaders exist |

---

## 6. What hotSpring Provides as Reference

### 6.1 Reference Implementations (Rust, CPU)

These are complete, validated implementations that the BarraCuda team can use as the "ground truth" for GPU versions:

| Module | What It Solves | Lines | Validated Against |
|--------|---------------|-------|-------------------|
| `physics/hfb.rs` | Spherical HFB (L2) | 745 | Python `skyrme_hfb.py` |
| `physics/hfb_deformed.rs` | Deformed HFB (L3) | 520 | Architecture only |
| `physics/semf.rs` | SEMF binding energy (L1) | ~100 | Python `objective.py` |
| `physics/nuclear_matter.rs` | NMP computation | ~150 | Python `nuclear_matter.py` |
| `bin/nuclear_eos_l2_ref.rs` | Full L2 pipeline | 500 | Multi-seed validation |

### 6.2 Validated Test Cases

For every GPU-ported function, test against these reference outputs:

```
# L1 (must give chi2/datum < 1.0)
cargo run --release --bin nuclear_eos_l1_ref

# L2 seed=42 (must give chi2_BE = 16.11 exactly — reproducible)
cargo run --release --bin nuclear_eos_l2_ref -- --seed=42 --lambda=0.1 --rounds=5

# L2 seed=123 (must give chi2_BE = 19.29, all NMP within 2σ)
cargo run --release --bin nuclear_eos_l2_ref -- --seed=123 --lambda=1.0 --rounds=5
```

### 6.3 SLy4 Reference Point

For validating individual functions, use SLy4 parameters on O-16:
```
params = [-2488.91, 486.82, -546.39, 13777.0, 0.834, -0.344, -1.0, 1.354, 0.1667, 123.0]
nucleus = (Z=8, N=8)
B_exp = 127.62 MeV
```

---

## 7. Immediate Action Items

### For BarraCuda Team (before Titan V)

- [ ] Commit `gradient_1d` 2nd-order boundary fix (already in tree, tests pass)
- [ ] Set `auto_smoothing: true` as default in `SparsitySamplerConfig::new()`
- [ ] Add `n_explore_solvers: usize` to `SparsitySamplerConfig`
- [ ] Add `parallel` feature to Cargo.toml (for cascade pipeline)
- [ ] Fill `dispatch/benchmark.rs` stubs with real benchmarks
- [ ] Design batched `eigh_f64` API: `batched_eigh_f64(matrices: &[f64], n: usize, batch: usize)`
- [ ] Design batched `gradient_1d` API: `batched_gradient_1d(arrays: &[&[f64]], dx: f64)`

### For hotSpring Team (with Titan V)

- [ ] Wire GPU dispatch for L2 HFB evaluation
- [ ] Benchmark CPU vs Titan V on L2 (52 nuclei, SLy4)
- [ ] Run L2 with 10× budget on Titan V (target chi²<5)
- [ ] Debug L3 deformed energy functional (normalization, double-counting)
- [ ] Run L3 on Titan V (should be feasible in minutes, not hours)

---

## 8. Summary: The Evolution Equation

```
BarraCuda Evolution = (validated math) + (GPU acceleration) + (science feedback)

    ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
    │   hotSpring   │────→│  BarraCuda   │────→│   Science    │
    │  Validation   │     │  Evolution   │     │  Workloads   │
    │              │←────│              │←────│              │
    └──────────────┘     └──────────────┘     └──────────────┘
    
    Cycle 1: 28,450 → 92    (physics fixes)
    Cycle 2: 92 → 16.11     (numerical precision: gradient, brent, eigh)
    Cycle 3: 16.11 → ???    (GPU budget: Titan V)
    Cycle 4: ??? → paper    (deformation: L3 on GPU)
```

Each cycle, hotSpring validates, identifies the bottleneck, and the BarraCuda team evolves. The Titan V is the accelerant for Cycle 3.

---

**The math is right. The physics is right. Now we need the compute.**

**Last Updated**: February 13, 2026 (evening)
