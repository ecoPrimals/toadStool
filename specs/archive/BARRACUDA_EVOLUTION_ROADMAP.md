# BarraCuda Evolution Roadmap

**Date**: February 10, 2026
**Status**: Active — Phase 2A/2B/2C Complete, Phase 3 Planning

---

## Philosophy

**BarraCuda is the math. ToadStool is how we run the math.**

BarraCuda provides universal mathematical primitives via WGSL shaders and pure Rust middleware. Every operation — whether for physics simulation, ML training, audio processing, ray tracing, or neural signal processing — is the same math expressed once. ToadStool routes that math to the optimal hardware.

hotSpring and future workloads are how we test our limitations and continue to evolve. When hotSpring reveals that a Nelder-Mead optimizer is needed for nuclear physics surrogates, that optimizer belongs in BarraCuda — because the same algorithm serves ML hyperparameter tuning, audio filter design, and rendering parameter optimization.

**Principles**:
- **Pure Rust end to end**. No Python. No FFI. No ports.
- **WGSL shaders are universal compute**. Same shader for physics AND gaming AND audio.
- **Algorithms are cross-domain**. Latin Hypercube Sampling serves surrogate learning AND neural architecture search AND materials science.
- **Workloads inform evolution**. hotSpring tests physics. Future workloads test ray tracing, audio, genomics. Each reveals new shaders and functions to evolve.
- **ToadStool is agnostic**. It routes workloads. It does not contain math.

---

## Architecture

```
Applications & Workloads (hotSpring, NUCLEUS, gaming, audio...)
       |
       | "I need eigendecomposition" / "I need RBF interpolation"
       |
BarraCuda: The Math Layer
  ┌──────────────────────────────────────────────────┐
  │  WGSL Shaders (414+)                             │
  │    GPU-native: matmul, conv, attention, FFT,     │
  │    Bessel, eigh, linsolve, RBF kernels,          │
  │    Nelder-Mead, gradient, integration...          │
  │                                                    │
  │  Rust Middleware (Pure f64 CPU fallback)          │
  │    linalg, numerical, special, optimize,          │
  │    surrogate, sample — same algorithms,           │
  │    CPU path when GPU not needed                   │
  └──────────────────────────────────────────────────┘
       |
ToadStool: The Hardware Router
  ┌──────────────────────────────────────────────────┐
  │  WorkloadHint → Device Selection                  │
  │  17 hints × 4 device types × user override       │
  │  JSON-RPC 2.0 + tarpc IPC                        │
  │  Cross-gate routing across machines               │
  └──────────────────────────────────────────────────┘
       |
  GPU (Vulkan/Metal/DX12)  NPU (Akida)  CPU (Rayon)
```

### Cross-Domain Math Reuse

| BarraCuda Function | Physics Use | ML Use | Graphics Use | Audio Use |
|--------------------|-------------|--------|-------------|-----------|
| `eigh` | HFB eigenvalues | PCA | Normal estimation | Spectral analysis |
| `linsolve` | Force-field fitting | Ridge regression | Mesh smoothing | Filter design |
| `RBF surrogate` | EOS approximation | Bayesian optimization | Terrain interpolation | Transfer function |
| `Nelder-Mead` | Parameter fitting | Hyperparameter tuning | Camera calibration | Codec optimization |
| `Latin Hypercube` | Design of experiments | Architecture search | Material sampling | Impulse response |
| `FFT` | DSF computation | Spectral features | Image filtering | Pitch detection |
| `Bessel J0/J1` | Scattering theory | Kernel functions | Diffraction patterns | Acoustic modes |
| `trapz` | Energy integrals | Loss integration | Area computation | RMS energy |
| `gamma` | Statistical distributions | Regularization | Subsurface scattering | Noise shaping |
| `cdist` | Pair correlations | kNN | Point cloud | Echo detection |

**Key insight**: The same mathematical operation serves different domains. BarraCuda implements it once, correctly, on any hardware. Workloads consume it without caring about the hardware.

---

## Current State (February 11, 2026)

### Completed

| Module | Functions | Tests | Status |
|--------|-----------|-------|--------|
| **414 WGSL Shaders** | Full ML/physics/crypto/audio | 1,127 | ✅ Production |
| **linalg** | `solve_f64` (Gauss-Jordan) | 9 | ✅ Phase 1 |
| **numerical** | `gradient_1d`, `trapz`, `trapz_product` | 15 | ✅ Phase 1 |
| **special** | `gamma`, `factorial`, `laguerre`, `laguerre_all`, `laguerre_simple` | 21 | ✅ Phase 2B |
| **optimize** | `nelder_mead`, `multi_start_nelder_mead`, `bisect`, `ResumableNelderMead` | 37 | ✅ Phase 2A/2B |
| **sample** | `latin_hypercube`, `random_uniform`, `maximin_lhs`, `sparsity_sampler` | 31 | ✅ Phase 2A/2B |
| **surrogate** | `RBFSurrogate`, 6 `RBFKernel` types, `train_adaptive`, `train_adaptive_gpu`, `train_with_validation` | 22 | ✅ Phase 2C |
| **eval_record** | `EvaluationCache`, `EvaluationRecord` | 6 | ✅ Phase 2A |
| **Hardware routing** | 17 WorkloadHints, auto + override | — | ✅ Production |

**Total**: 1,283+ tests (1,242 barracuda lib), 0 unsafe blocks in middleware, 0 production `.unwrap()` on hot paths

---

## Remaining Work

### Phase 2A: Sampling & Global Optimization (COMPLETE)

**Goal**: Enable optimizer-guided space-filling sampling that closes the L2 accuracy gap.

hotSpring L2 revealed that the accuracy gap (χ²=25.43 vs 1.93) is purely algorithmic — Python uses optimizer-directed sampling (SparsitySampler) while our L2 used naive sampling.

| Task | Module | Function | Status |
|------|--------|----------|--------|
| Latin Hypercube Sampling | `barracuda::sample::lhs` | `latin_hypercube(n, bounds, rng)` | ✅ Done (11 tests) |
| Multi-start Nelder-Mead | `barracuda::optimize::multi_start` | `multi_start_nelder_mead(f, bounds, ...)` | ✅ Done (10 tests) |
| Evaluation record system | `barracuda::optimize::eval_record` | `EvaluationRecord`, `EvaluationCache` | ✅ Done (6 tests) |

**Impact**: LHS with multi-start NM provides optimizer-directed space-filling. 74 middleware tests total.

---

### Phase 2B: Full SparsitySampler (COMPLETE)

**Goal**: Full parity with Python L2 (χ²<2)

| Task | Module | Description | Status |
|------|--------|-------------|--------|
| Sparsity-based sampling | `barracuda::sample::sparsity` | Iterative surrogate-directed sampling (Diaw et al. 2024) | ✅ Done (10 tests) |
| Maximin LHS | `barracuda::sample::maximin` | Maximize minimum pairwise distance via CP algorithm | ✅ Done (10 tests) |
| Resumable solver | `barracuda::optimize::solver_state` | Pausable/resumable Nelder-Mead with eval cache | ✅ Done (8 tests) |

**Impact**: Complete SparsitySampler pipeline: maximin LHS → multi-start NM → RBF surrogate → surrogate-guided refinement. 28 new tests.

---

### Phase 2C: GPU-Accelerated Scientific Computing (COMPLETE)

**Goal**: 14× training speedup for large surrogate models

| Task | Module | Description | Status |
|------|--------|-------------|--------|
| Adaptive dispatch | `barracuda::surrogate::adaptive` | Auto f32/f64 based on N, GPU config | ✅ Done (13 tests) |
| 11 shader TODOs evolved | `shaders/` | All TODOs closed: pow, broadcast, cast, determinant, gather/scatter, edge_conv, spectral_norm, index_add, u64_emu Barrett, fhe_key_switch | ✅ Done |
| Wire up `cdist.wgsl` | `barracuda::surrogate` | GPU pairwise distance for RBF | ✅ Done (4 GPU tests) |
| `train_adaptive_gpu()` | `barracuda::surrogate::adaptive` | Full GPU training pipeline with cdist shader | ✅ Done |
| f64 WGSL variants | `shaders/linalg/` | `cdist_f64.wgsl`, `linsolve_f64.wgsl` | 🟡 Deferred (f32+promote sufficient) |

**Impact**: GPU surrogate training implemented via `train_adaptive_gpu()`. Uses `cdist.wgsl` for O(n²·d) distance computation (10-14× faster), promotes to f64 for kernel evaluation and linear solve. Tested on NVIDIA and AMD hardware. 17 total adaptive tests (13 CPU + 4 GPU).

---

### Phase 3: Cross-Domain Shader Evolution

**Goal**: Discover and implement shaders that serve multiple domains

Each new workload (physics, graphics, audio, neural) reveals mathematical operations that may already exist in BarraCuda or need new shaders. The evolution cycle:

```
Workload → Reveal Limitation → Implement Shader → Test → Ship
                ↑                                        |
                └────────────────────────────────────────┘
```

#### Identified Cross-Domain Opportunities

| Shader | Currently Serves | Could Also Serve |
|--------|-----------------|------------------|
| `bessel_j0.wgsl` | Physics (scattering) | Audio (acoustic modes), Graphics (diffraction) |
| `eigh.wgsl` | Physics (HFB) | ML (PCA), Graphics (normal estimation) |
| `spherical_harmonics.wgsl` | Physics (angular momentum) | Graphics (environment maps, SH lighting) |
| `fft_1d.wgsl` | Physics (DSF) | Audio (pitch), Graphics (convolution bloom) |
| `xoshiro128.wgsl` | Physics (Monte Carlo) | ML (dropout), Graphics (noise textures) |

#### Potential Future Shaders

| Shader | Use Case | Cross-Domain Value |
|--------|----------|-------------------|
| `ray_march.wgsl` | Graphics (rendering) | Physics (ray tracing), Audio (room acoustics) |
| `wavelet.wgsl` | Audio (analysis) | Physics (signal), Graphics (texture compression) |
| `poisson_solve.wgsl` | Physics (electrostatics) | Graphics (seamless clone), Audio (room impulse) |
| `sph_kernel.wgsl` | Physics (SPH fluids) | Graphics (fluid rendering), Audio (wave propagation) |
| `voronoi.wgsl` | Materials (grain structure) | Graphics (procedural textures), Audio (spatial) |

---

### Phase 4: Hardware Evolution

**Goal**: Maximize hardware utilization across all device types

| Task | Description | Status |
|------|-------------|--------|
| VFIO NPU driver | Pure Rust Akida driver (eliminate C kernel module) | 🟡 Spec complete |
| NPU model pipeline | Train/compile/deploy from Rust | 🟡 Planned |
| Multi-GPU DevicePool | Use all GPUs on a machine simultaneously | 🟡 Planned |
| INT4/INT8 quantization | Quantized inference shaders | 🟡 Planned |
| Safetensors/GGUF loader | Eliminate PyTorch weight dependency | 🟡 Planned |

---

## Testing Strategy

**hotSpring is the primary test workload** for scientific computing. Future workloads will test other domains.

| Workload | Tests | Domain |
|----------|-------|--------|
| hotSpring Sarkas MD | 60/60 pass | Molecular dynamics |
| hotSpring TTM | 6/6 pass | Thermal transport |
| hotSpring Surrogate Learning | 15/15 pass | Optimization + ML |
| hotSpring Nuclear EOS L1 | χ²=2.27 (beats Python) | Nuclear physics |
| hotSpring Nuclear EOS L2 | χ²=25.43 (needs Phase 2A) | Nuclear physics |
| BarraCuda unit tests | 1,127 pass | All domains |
| ToadStool + infrastructure | 1,204 pass | System integration |

**Future test workloads**:
- Ray tracing benchmark (Sponza, Cornell Box)
- Audio processing benchmark (real-time DSP)
- Genomics pipeline (sequence alignment)
- Neural signal processing (EEG/EMG)

---

## Success Metrics

| Metric | Current | Target | When |
|--------|---------|--------|------|
| L2 χ²/datum | 25.43 | < 2.0 | Phase 3 (larger datasets) |
| WGSL shaders | 414 | 450+ | Phase 3 |
| Middleware functions | **26** | 25+ | ✅ **Reached** |
| Middleware tests | **133** | 100+ | ✅ **Exceeded** |
| GPU surrogate training | **Implemented** | 10-14× speedup | ✅ **Phase 2C** |
| Cross-domain reuse | 3 workloads | 6+ workloads | Phase 3 |

---

## Deep Debt Compliance

All new work follows deep debt principles:

- **Pure Rust**: No Python, no FFI, no external non-Rust dependencies
- **Zero unsafe**: All middleware is 100% safe Rust
- **Idiomatic**: Iterators, closures, Result<T,E>, typed errors, NaN-safe comparisons
- **Zero production panics**: All `.unwrap()` in hot paths evolved to `Result`, `partial_cmp` NaN-safe
- **FFI evolution**: `num_cpus` replaced with `std::thread::available_parallelism()` (pure Rust)
- **Self-knowledge**: Each module knows what it does, not who calls it
- **Capability-based**: Runtime discovery, no hardcoded consumers
- **Tested**: Comprehensive unit tests with edge cases (3,667+ core tests)
- **Documented**: Algorithm references, examples, design rationale

---

**Last Updated**: February 10, 2026 (Phase 2C complete: GPU surrogate training via `train_adaptive_gpu()`, tested on NVIDIA/AMD hardware, 17 adaptive tests)
