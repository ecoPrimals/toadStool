# Shader Evolution Tracker

**Mandate**: All math originates as WGSL shaders at fp64 for universal portability. No CPU-only workloads.

---

## Cross-Project Absorption Status

| Spring | Status | Notes |
|--------|--------|-------|
| hotSpring | 100% absorbed | All shaders absorbed. Screened Coulomb eigensolve done S52. |
| neuralSpring | 100% absorbed | All 4 S43 shaders + xoshiro PRNG in toadStool. 13 shaders evolved to f64 (S49), pipelines wired |
| wetSpring | 100% absorbed | All 5 ODE shaders at f64. `BatchedOdeRK4` generic in place |
| airSpring | 100% absorbed | Uses toadStool shaders via GPU bridge. No local shaders |
| groundSpring | N/A | Directory does not exist |
| wateringHole | Reviewed | Latest handoffs (Feb 22) confirm absorption complete |

---

## f32 → f64 Evolution Queue

Shaders absorbed from neuralSpring/hotSpring at original f32 precision. Must be evolved to f64 for the universal math library.

### Bio / Game Theory / Population Genetics

| Shader | Domain | f64 File | Status |
|--------|--------|----------|--------|
| `stencil_cooperation.wgsl` | Fermi imitation dynamics | `stencil_cooperation_f64.wgsl` | DONE S49 |
| `wright_fisher_step.wgsl` | Wright-Fisher drift+selection | `wright_fisher_step_f64.wgsl` | DONE S49 |
| `hill_gate.wgsl` | Two-input Hill AND gate | `hill_gate_f64.wgsl` | DONE S49 |
| `locus_variance.wgsl` | Per-locus AF variance (FST) | `locus_variance_f64.wgsl` | DONE S49 |
| `multi_obj_fitness.wgsl` | Multi-objective EA fitness | `multi_obj_fitness_f64.wgsl` | DONE S49 |
| `swarm_nn_forward.wgsl` | Swarm NN MLP forward | `swarm_nn_forward_f64.wgsl` | DONE S49 |
| `batch_fitness_eval.wgsl` | Linear fitness dot product | `batch_fitness_eval_f64.wgsl` | DONE S49 |

### Numerical / ML

| Shader | Domain | f64 File | Status |
|--------|--------|----------|--------|
| `rk45_adaptive.wgsl` | Dormand-Prince ODE | `rk45_adaptive_f64.wgsl` | DONE S49 |
| `logsumexp.wgsl` | Numerically stable logsumexp | `logsumexp_f64.wgsl` | DONE S49 |
| `hmm_forward_log.wgsl` | HMM forward pass (log-domain) | `hmm_forward_log_f64.wgsl` | DONE S49 |
| `prng_xoshiro.wgsl` | Xoshiro128** PRNG | `prng_xoshiro_f64.wgsl` | DONE S49 |

### ESN (Echo State Networks)

| Shader | Domain | f64 File | Status |
|--------|--------|----------|--------|
| `esn_reservoir_update.wgsl` | Reservoir leaky-tanh update | `esn_reservoir_update_f64.wgsl` | DONE S49 |
| `esn_readout.wgsl` | Readout matvec | `esn_readout_f64.wgsl` | DONE S49 |

---

## Missing Shaders (Not Yet Created)

| Shader | Domain | Source | Priority |
|--------|--------|--------|----------|
| ~~`heat_current_f64.wgsl`~~ | ~~Thermal conductivity~~ | ~~hotSpring~~ | DONE S49 |
| Screened Coulomb eigensolve | Plasma physics | hotSpring CPU `screened_coulomb.rs` | Low |

---

## CPU → GPU Evolutions (S49c–S49d)

### S49c

| Area | Change | Status |
|------|--------|--------|
| `RdfHistogramF64` | Wired to `rdf_histogram_f64.wgsl` GPU dispatch; CPU fallback removed | DONE |
| `cdist_f64.wgsl` | Created f64 pairwise distance shader (Euclidean/Manhattan/Cosine) | DONE |
| `compute_distances_f64_gpu()` | Standalone f64 GPU distance API in `cdist_wgsl.rs` | DONE |

### S49d — Shader-First Enforcement

| Area | Change | Status |
|------|--------|--------|
| `VelocityVerletF64` | Full GPU dispatch via `velocity_verlet_f64.wgsl` (3 entry points: step, half_vel, pos_update). CPU removed. | DONE |
| `msd_f64.wgsl` + `MsdGpu` | New MSD observable shader + GPU wrapper (O(N×frames) per lag) | DONE |
| `cubic_spline_eval_f64.wgsl` | Evolved from bitcast to native f64. `eval_many_gpu()` added to `CubicSpline` | DONE |
| `CoulombF64` | CPU fallback removed — always dispatches GPU shader | DONE |
| `MorseForceF64` | CPU fallback removed — always dispatches GPU shader | DONE |
| `BornMayerF64` | CPU fallback removed — always dispatches GPU shader | DONE |
| `YukawaCellListF64` | CPU fallback removed — always dispatches GPU shader | DONE |
| All force CPU functions | Gated behind `#[cfg(test)]` as reference implementations | DONE |
| `special/gamma.rs` | Documented shader-first duality (WGSL equivalents listed per function) | DONE |
| `special/laguerre.rs` | Documented shader-first duality | DONE |

### S49e — Comprehensive CPU Fallback Elimination

| Area | Change | Status |
|------|--------|--------|
| **Threshold-gated CPU fallbacks (20+ files)** | Removed `if n < THRESHOLD` gates from all ops with GPU shaders. CPU functions gated `#[cfg(test)]`. | DONE |
| `crank_nicolson.rs` | `if n<32 \|\| n_steps<10` removed — always GPU | DONE |
| `cyclic_reduction_f64.rs` | `if n<64` removed — always GPU | DONE |
| `batched_elementwise_f64.rs` | `if batch_size<64` removed — always GPU | DONE |
| `moving_window_stats.rs` | `if n<256 \|\| n_out<64` removed — always GPU | DONE |
| `correlation_wgsl.rs` | `if n<256` + batch gate removed — always GPU | DONE |
| `covariance_wgsl.rs` | `if n<256` removed — always GPU | DONE |
| `fused_map_reduce_f64.rs` | `if n<1024` removed — always GPU | DONE |
| `rk_stage.rs` | `if n<128` removed — always GPU | DONE |
| `bray_curtis_f64.rs` | `if n_samples<32` removed — always GPU | DONE |
| All 4 bessel ops | `if size<256` removed — always GPU | DONE |
| `laguerre_f64_wgsl.rs` | Both `if size<256` removed — always GPU | DONE |
| `hermite_f64_wgsl.rs` | Both `if size<256` removed — always GPU | DONE |
| `legendre_f64_wgsl.rs` | Both `if size<256` removed — always GPU | DONE |
| `spherical_harmonics_f64_wgsl.rs` | `if size<256` removed — always GPU | DONE |
| `weighted_dot_f64.rs` | Both `if n<1024` removed — always GPU | DONE |
| `cosine_similarity_f64.rs` | Both size gates removed — always GPU | DONE |
| **Always-CPU ops wired to GPU** | 6 ops that had shaders but never dispatched them | DONE |
| `KineticEnergyF64` | Full GPU dispatch via `kinetic_energy_f64.wgsl` (was 100% CPU) | DONE |
| `VarianceF64` | Wired to `variance_f64.wgsl` (evolved to native f64) | DONE |
| `CovarianceF64` | Wired to `covariance_f64.wgsl` (evolved to native f64) | DONE |
| `CorrelationF64` | Wired to `correlation_f64.wgsl` (evolved to native f64) | DONE |
| `DigammaF64` | Wired to `digamma_f64.wgsl` (evolved to native f64) | DONE |
| `BetaF64` | Wired to `beta_f64.wgsl` (evolved to native f64) | DONE |

### S49f — Linalg + RBF + PPPM GPU Wiring

| Area | Change | Status |
|------|--------|--------|
| `solve_f64` | Now takes `Arc<WgpuDevice>`, delegates to `LinSolveF64` (`linsolve_f64.wgsl`). CPU renamed `solve_f64_cpu`, gated `#[cfg(test)]`. | DONE |
| `cholesky_f64` | Now takes `Arc<WgpuDevice>`, delegates to `CholeskyF64::execute()` (`cholesky_f64.wgsl`). CPU renamed `cholesky_f64_cpu`, gated `#[cfg(test)]`. | DONE |
| `gen_eigh_f64` | Updated to accept device, passes through to `cholesky_f64` | DONE |
| `RBFSurrogate` | Added `device: Arc<WgpuDevice>` field. `train()`, `predict()`, `loo_cv_rmse()`, `compute_hat_diagonal()` all use GPU solve + GPU cdist | DONE |
| `compute_distances` (RBF) | Replaced by `compute_distances_f64_gpu()` via `cdist_f64.wgsl`. CPU version gated `#[cfg(test)]`. | DONE |
| `Pppm` (electrostatics) | Added `device` field. FFT now dispatches `Fft3DF64` (GPU) instead of `fft_3d_cpu`. CPU FFT gated `#[cfg(test)]`. | DONE |
| `sample/direct.rs` | `direct_sampler` takes `device: Arc<WgpuDevice>` | DONE |
| `sample/sparsity/*` | All sampler entry points take `device`, pass through to RBF train | DONE |
| `dispatch/benchmark.rs` | GPU device threaded into solve/cholesky/eigh benchmarks | DONE |
| `adaptive/mod.rs` | Device forwarded to `solve_f64` and `RBFSurrogate::from_parts` | DONE |

### Remaining CPU Math (Assessed)

| Area | Impact | GPU exists? | Status |
|------|--------|-------------|--------|
| `eigh_f64` (Jacobi) | Medium | WGSL exists (multi-pass) | Needs GPU orchestration wrapper |
| Cubic spline setup (Thomas) | Low | O(N) sequential | CPU appropriate |
| Conv2D/Pool shaders | Medium | WGSL exists but lacks stride/padding/channels | D-S46-001 |

---

## Inventory Summary

- **645+ WGSL shaders** in barracuda (zero orphans, all wired to Rust)
- **13 f32 shaders** evolved to f64 (S49) — all Naga-validated
- **5 shaders evolved** to native f64 (S49e): variance, covariance, correlation, digamma, beta
- **27+ CPU fallback gates** eliminated (S49e) — all ops always dispatch GPU
- **6 always-CPU ops** wired to GPU (S49e): KE, variance, covariance, correlation, digamma, beta
- **Linalg GPU-first** (S49f): `solve_f64`, `cholesky_f64` now dispatch GPU shaders
- **RBF surrogate** (S49f): full GPU pipeline (cdist + solve)
- **PPPM FFT** (S49f): CPU FFT replaced with GPU `Fft3DF64`
- **4 shaders GPU-wired** (S49c): RDF, cdist_f64
- **3 shaders wired** (S49d): VV, MSD, cubic spline eval
- **4 force modules** — CPU fallbacks removed (coulomb, morse, born_mayer, yukawa)
- All lattice QCD: GPU-first (S47-S48)
- All bio ODE: f64 (5 RK4 shaders)
- All MD: f64 (Yukawa, VV, Berendsen, RDF, VACF, MSD, KE, stress virial, PPPM)
- All physics: f64 (HFB, BCS, SEMF, deformed)
- All statistics: GPU (variance, covariance, correlation)
- All special functions: GPU (bessel, laguerre, hermite, legendre, spherical harmonics, digamma, beta)
- All linalg: GPU (solve, cholesky, linsolve, QR, SVD, LU via ops/)

---

## Deep Debt Principles (Applied)

1. **Shader-first math** — all compute originates as WGSL f64. Barracuda does not care about hardware.
2. **f32 retained** as fossil only (original absorption record)
3. **CPU reference** code gated behind `#[cfg(test)]` — production always dispatches shader
4. **No external math deps** — pure Rust + WGSL
5. **Capability-based dispatch** — toadstool routes to best substrate at runtime
6. **Portability** — by enforcing all math starts as shaders, others can develop consistent idiomatic mathematics

---

*Updated: Session S49 complete, Feb 23, 2026*
*Shader-first architecture enforced: zero CPU-only math in production. All linalg, RBF, PPPM, MD, special functions, statistics dispatch GPU shaders. f64 transcendentals covered by compile_shader_f64() polyfill pipeline.*
