# ToadStool + BarraCUDA -- Quick Status

**Date**: February 14, 2026 (L2 Evolution — hotSpring MD Complete)

---

## Quality Gates

```
cargo build --workspace          CLEAN
cargo fmt --all -- --check       CLEAN
cargo clippy --workspace         CLEAN (was 166 warnings)
cargo test --workspace           15,700+ passed / 0 failed
unsafe blocks                    FFI only (VFIO, DRM) - SAFETY documented
middleware tests                 330+ passed (linalg, sparse, numerical, special, stats, optimize, surrogate, sample, pde, pipeline)
dependency evolution             once_cell, lazy_static → std::sync::LazyLock
```

*All clippy warnings resolved. Workspace fully clean.*

---

## At a Glance

```
ToadStool (Hardware Infrastructure Primal)
  Pure Rust | ecoBin | UniBin | JSON-RPC 2.0 + tarpc
  26 JSON-RPC methods (compute, gpu, ollama, gate)
  GPU Job Queue with Cross-Gate Routing
  Ollama model lifecycle management
  3 GPUs across 2 machines, 2 vendors (NVIDIA + AMD)
  52 GB combined VRAM, 88 CPU threads
  Capability-based runtime discovery (zero hardcoding)
  Shared error tracking across all transports
  Hardware-agnostic workload routing (GPU / NPU / CPU)

BarraCUDA (Universal Compute Engine — SHADER-FIRST)
  396 WGSL shaders, proven cross-vendor
  **Shader-first architecture**: ALL math is WGSL primary
  ToadStool dispatches to GPU/CPU based on hardware
  When fp64 GPUs available, seamless transition
  Bit-identical results: RTX 4070 = RTX 3090 = RX 6950 XT
  39.85 tok/s distributed LLM inference
  18 special function shaders (Hermite, Legendre, Laguerre, Bessel, etc.)
  3 sampling shaders (Sobol, LHS, random_uniform)
  Scientific middleware: 8 modules (linalg, numerical, special, stats, optimize, surrogate, sample, pde)
  200+ tests (RBF, LU/QR/SVD, RK45, Crank-Nicolson, BFGS, Sobol, correlation)
  Smart auto-routing with user device preference override
```

---

## Cross-Vendor Validation

| GPU | Vendor | GFLOPS | Checksum |
|-----|--------|--------|----------|
| RTX 4070 | NVIDIA | 388.7 | 5.128010 |
| RTX 3090 | NVIDIA | 481.0 | 5.128010 |
| RX 6950 XT | AMD | 222.7 | 5.128010 |

Same binary. Same shader. Same results. Zero vendor SDK.

---

## Hardware Routing

```
WorkloadHint         Auto-Route   Override?
─────────────────    ──────────   ─────────
PhysicsForce         GPU          yes (CPU fallback)
FFT                  GPU          yes
EigenDecomp          GPU          yes
LinearSolve          GPU          yes
Training             GPU          yes
MonteCarlo           GPU          yes
SparseMath           GPU          yes
SurrogateEval        GPU          yes
LargeMatrices        GPU          yes
SparseEvents         NPU          yes (CPU/GPU fallback)
Inference            NPU          yes
PreScreen            NPU          yes
Reservoir            NPU          yes
EventProcessing      NPU          yes (CPU fallback)
SmallWorkload        CPU          yes (can force GPU)
StringOps            CPU          yes
General              GPU→CPU      yes
```

`Device::select_with_preference(Some(Device::CPU), &hint)` honours the
user's choice. Auto only kicks in when preference is `None` or `Auto`.

---

## Code Quality

| Metric | Value |
|--------|-------|
| Clippy warnings | 0 (was 166) |
| Build warnings | 0 |
| Tests passing | 15,700+ |
| Tests failing | 0 |
| WGSL shaders | 396+ (including MD f64) |
| Server line coverage | ~85% |
| Common line coverage | ~84% |
| Config line coverage | ~85% |
| Unsafe blocks | FFI only (VFIO, DRM) |
| Production `todo!()` | 0 |
| Production mocks | 0 |
| External lazy deps | 0 (migrated to std) |

---

## What Works

- 396 WGSL shaders on any GPU (NVIDIA, AMD via Vulkan)
- **Shader-first architecture**: ALL math is WGSL, ToadStool dispatches
- Distributed LLM inference across machines (LAN TCP, BearDog encrypted)
- Hardware discovery (GPUs, NPUs, CPUs) -- pure Rust, no scripts
- NPU detection via /dev/akida* and IOMMU/VFIO sysfs
- JSON-RPC 2.0 + tarpc IPC over Unix sockets (26 methods)
- GPU job queue with priority and cross-gate routing
- Ollama model management (list, inference, load, unload)
- Cross-gate compute delegation (route by model locality, VRAM, queue depth)
- Matrix decompositions (LU, QR, SVD, Cholesky, eigh, tridiagonal)
- ODE/PDE solvers (RK45 adaptive, Crank-Nicolson heat equation)
- Special functions (Bessel, Hermite, Legendre, erf, gamma, digamma, beta)
- Statistics (Normal distribution, correlation, covariance matrices)
- Optimization (Nelder-Mead, BFGS, bisection)
- Sampling (LHS, Sobol quasi-random, maximin)
- RBF surrogates with GPU-accelerated training
- FHE acceleration (21.1x speedup on RTX 3090)
- Smart auto-routing with user preference override
- CPU compute backends (matmul, conv2d, pooling) as universal fallback
- CUDA PTX kernel execution via cudarc
- Unified memory with wgpu fallback (OpenCL/Vulkan)
- Unix socket security providers (JSON-RPC 2.0)

## L2 Evolution Status (hotSpring Validation Response)

### ✅ Completed (Feb 13, 2026) — L2 Evolution Tier 1 Critical Fixes

- **gradient_1d 2nd-order boundaries** -- Matches numpy.gradient; exact for polynomials ≤ degree 2
  - Expected impact: ~40 MeV reduction in HFB SCF offset
- **auto_smoothing = true default** -- Prevents SparsitySampler overfitting (was false)
- **smoothing = 1e-3 default** -- Reasonable fallback (was 1e-12)

### ✅ Completed (Feb 13, 2026) — L2 Evolution Tier 2 Algorithm Improvements

- **Hybrid evaluation mode** -- `n_direct_solvers` field in SparsitySamplerConfig
  - Direct solvers run NM on TRUE objective (exploration)
  - Surrogate solvers run NM on surrogate (exploitation)
  - Closes evaluation density gap vs Python's mystic
- **Warm-start cascade** -- `top_k_seeds()` on SparsitySamplerResult and DirectSamplerResult
  - Enables SparsitySampler → DirectSampler cascade optimization
  - L1 seeds flow to L2 optimization

### ✅ Completed (Feb 13, 2026) — Phase 5 Tier 3 Architecture

- **Dispatch Benchmark Suite** -- `BenchmarkSuite` for empirical CPU/GPU threshold determination
- **Pipeline Orchestration** -- `Cascade` API for hotSpring-validated multi-stage filtering
- **Sparse Linear Algebra** -- `CsrMatrix`, `cg_solve`, `bicgstab_solve` for large HFB basis sets

### ✅ Completed (Feb 13, 2026) — Phase 5 Tier 2 New Algorithms

- **Direct Sampler** -- `direct_sampler()` round-based NM on true objective (achieved χ²/datum = 1.19)
- **Chi² Decomposition** -- `chi2_decomposed()` with per-datum residuals, pulls, worst-N
- **Bootstrap CI** -- `bootstrap_ci()` non-parametric confidence intervals for any statistic
- **Convergence Diagnostics** -- `convergence_diagnostics()` detecting stagnation/oscillation/divergence
- **Adaptive Penalty** -- `adaptive_penalty()` data-driven penalty from feasible values

### ✅ Completed (Feb 13, 2026) — Phase 5 Tier 1 Critical Fixes

- **LOO-CV hat matrix bug fixed** -- K_raw for RHS, K_smooth for system (was H_ii = 1.0 always)
- **Auto-smoothing** -- `SparsitySamplerConfig::auto_smoothing`, `loo_cv_optimal_smoothing()`
- **Penalty filtering** -- `PenaltyFilter` enum (Threshold, Quantile, AdaptiveMAD)
- **Warm-start seeds** -- `SparsitySamplerConfig::with_warm_start()` for L1→L2 seeding
- **digamma(x)** -- ψ(x) = Γ'(x)/Γ(x) with 1e-9 precision
- **beta(a,b), ln_beta(a,b)** -- B(a,b) = Γ(a)Γ(b)/Γ(a+b)

### hotSpring Validation Results

```
L1 (SEMF): χ²/datum = 1.19 (BarraCUDA) vs 6.62 (scipy) → 82% BETTER
Validation Suite: 129/129 tests PASS
```

## Phase 3 Status (Complete)

### ✅ Completed (Feb 12, 2026)

- **f64 linalg bridges** -- `cholesky_f64`, `eigh_f64`, `gen_eigh_f64`, LU/QR/SVD/tridiagonal
- **Auto-dispatch system** -- `dispatch` module with per-operation CPU/GPU thresholds
- **EvaluationCache persistence** -- `save/load/load_or_new` via serde_json
- **LOO-CV wiring** -- `loo_cv_rmse()`, `loo_cv_errors()` on RBFSurrogate
- **Root-finding** -- Newton-Raphson, Secant, Brent methods
- **Chi-squared distribution** -- CDF, PDF, quantile, goodness-of-fit test
- **Cubic spline** -- Natural/clamped/not-a-knot boundaries
- **Generalized eigenvalue** -- `gen_eigh_f64` via Cholesky reduction
- **Deep debt** -- Mock isolation (feature-gated), hardcoded path removal

### Awaiting Hardware (Phase C)

- Multi-GPU DevicePool (awaiting Titan V)
- f64 WGSL shaders (when WebGPU adds f64 extensions)
- f64 Tensor type

### ✅ Cross-Platform Systems (Feb 13, 2026 — VERIFIED)

- **VFIO Backend** -- Pure Rust VFIO driver for 2x Akida AKD1000 (DMA working, 80 NPUs each)
- **Multi-GPU Pool** -- 3 GPUs: 2x RTX 3090 + 1x RX 6950 XT (~2100 GFLOPS total)
- **Cross-Vendor Parity** -- NVIDIA = AMD, <1e-5 max difference (same WGSL shaders)
- **MMIO Infrastructure** -- Register map for AKD1000 (BAR0: control, BAR1: model, BAR2: data)
- **Burn ML Framework** -- wgpu-based inference (`burn-inference` crate with safetensors loader)
- **NeuroBench Harness** -- Pure Rust benchmark harness for neuromorphic evaluation
- **Cross-Platform Showcase** -- `multi_gpu_bench`, `npu_test`, `gpu_parity`, `cascade_demo`
- **hotSpring Bridge** -- Integration with MD validation suite

### Infrastructure (Ongoing)

- NPU model pipeline (train/compile/deploy from Rust)
- Safetensors/GGUF weight loader
- mDNS/K8s discovery (env vars work, others pending)

---

## Quick Commands

```bash
cargo build --release
cargo fmt --all -- --check
cargo clippy --workspace
cargo test --workspace
cargo test -p barracuda --lib
cargo llvm-cov -p toadstool-server --lib
```

---

## Documentation

- [README.md](README.md) -- Full overview
- [STATUS.md](STATUS.md) -- Detailed status
- [DOCUMENTATION.md](DOCUMENTATION.md) -- Navigation hub
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) -- Commands and API reference

---

## Scientific Middleware (Shader-First)

**12 production-grade modules** — WGSL shaders primary, ToadStool dispatches:

```
barracuda::linalg         - solve, cholesky, eigh, gen_eigh, LU, QR, SVD, tridiagonal
barracuda::linalg::sparse - CsrMatrix, CooMatrix, CG, BiCGSTAB, Jacobi solvers
barracuda::numerical      - Gradient, trapz, RK45 adaptive ODE solver
barracuda::special        - gamma, chi_squared, Hermite, Legendre, Laguerre, digamma, beta, erf, Bessel
barracuda::stats          - norm_cdf, norm_ppf, correlation, covariance, variance, bootstrap, chi2
barracuda::optimize       - Nelder-Mead, BFGS, bisection, Newton, Brent, diagnostics, penalty
barracuda::surrogate      - RBF with 6 kernels, GPU-accelerated training, LOO-CV
barracuda::sample         - Sobol, LHS, random_uniform, direct_sampler
barracuda::pde            - Crank-Nicolson heat equation solver
barracuda::interpolate    - Cubic spline (natural/clamped/not-a-knot) with derivatives
barracuda::dispatch       - Auto CPU/GPU routing with benchmark suite
barracuda::pipeline       - Cascade multi-stage filtering, Stage with Target devices
burn-inference            - HuggingFace models via Burn (wgpu backend)
neurobench-runner         - Pure Rust NeuroBench harness for NPU benchmarking
```

**Tests**: 350+ passing (156 Phase 3 + 62 Phase 5 + 25 cross-platform + MD)
**Quality**: Zero unsafe in compute ops, clippy clean, pure Rust
**Architecture**: Shader-first — 464 WGSL shaders, universal hardware
**Audit**: All hotSpring Tiers 1-3 complete (Feb 13), MD pipeline complete (Feb 14)
**Evolution**: once_cell/lazy_static → std::sync::LazyLock (pure std)
**MD Pipeline**: Full thermostat suite + MSD + Cell-list + **PPPM universal** (CPU + GPU w/kspace) — 38 tests
**Bug Fix (Feb 15)**: Cell-list i32 % wrapping bug fixed (hotSpring ALERT)
**Docs**: `specs/BARRACUDA_PHASE5_EVOLUTION_HOTSPRING.md`, `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md`

**Remaining Evolution Work**:
- Wire LU/QR/SVD WGSL shaders to public API
- Sparse solvers: need WGSL implementation
- Replace math_f64 software with native builtins in MD kernels

---

**Last Updated**: February 15, 2026
