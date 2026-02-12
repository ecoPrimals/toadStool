# ToadStool + BarraCUDA -- Quick Status

**Date**: February 12, 2026

---

## Quality Gates

```
cargo build --workspace          0 warnings
cargo fmt --all -- --check       CLEAN
cargo clippy --workspace         0 warnings (from 453)
cargo test --workspace           15,600+ passed / 0 failed
unsafe blocks                    35 blocks, 100% SAFETY documented
middleware tests                 200+ passed (linalg, numerical, special, stats, optimize, surrogate, sample, pde)
```

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
| Clippy warnings | 0 |
| Build warnings | 0 |
| Tests passing | 15,490+ (3,688 core) |
| Tests failing | 0 |
| WGSL shaders | 396 |
| Server line coverage | ~85% |
| Common line coverage | ~84% |
| Config line coverage | ~85% |
| Unsafe blocks | 35, all SAFETY documented |
| Production `todo!()` | 0 |
| Production mocks | 0 |
| Files over 1000 lines | 0 |

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

## Phase 3 Status

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

### Infrastructure (Ongoing)

- VFIO backend for Akida NPU (eliminate C kernel module)
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

**10 production-grade modules** — WGSL shaders primary, ToadStool dispatches:

```
barracuda::linalg      - solve, cholesky, eigh, gen_eigh, LU, QR, SVD, tridiagonal
barracuda::numerical   - Gradient, trapz, RK45 adaptive ODE solver
barracuda::special     - gamma, chi_squared, Hermite, Legendre, Laguerre, digamma, beta, erf, Bessel
barracuda::stats       - norm_cdf, norm_ppf, correlation, covariance, variance
barracuda::optimize    - Nelder-Mead, BFGS, bisection, Newton, Brent + EvaluationCache persistence
barracuda::surrogate   - RBF with 6 kernels, GPU-accelerated training, LOO-CV
barracuda::sample      - Sobol, LHS, random_uniform
barracuda::pde         - Crank-Nicolson heat equation solver
barracuda::interpolate - Cubic spline (natural/clamped/not-a-knot) with derivatives
barracuda::dispatch    - Auto CPU/GPU routing with per-operation thresholds
```

**Tests**: 300+ passing (156 new in Phase 3)
**Quality**: Zero unsafe, clippy clean, pure Rust
**Architecture**: Shader-first — ALL math runs on GPU when fp64 available
**Audit**: All hotSpring HIGH/MEDIUM gaps resolved (Feb 12)
**Docs**: `specs/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md`, `docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md`

---

**Last Updated**: February 12, 2026
