# ToadStool + BarraCUDA -- Quick Status

**Date**: February 11, 2026

---

## Quality Gates

```
cargo build --workspace          0 warnings
cargo fmt --all -- --check       CLEAN
cargo clippy --workspace         0 warnings (from 453)
cargo test --workspace           15,490+ passed / 0 failed / 156 ignored
unsafe blocks                    35 blocks, 100% SAFETY documented
middleware tests                 129 passed (linalg, numerical, special, optimize, surrogate, sample)
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

BarraCUDA (Universal Compute Engine)
  414 WGSL shaders, proven cross-vendor
  Bit-identical results: RTX 4070 = RTX 3090 = RX 6950 XT
  39.85 tok/s distributed LLM inference
  CPU backends (LayerNorm, BatchNorm, MatMul, Conv2d, etc.)
  Science: Bessel, spherical harmonics, eigendecomp, linear solve
  PRNG (xoshiro128**), sparse CSR matvec, LOO-CV
  Scientific middleware: 6 modules (linalg, numerical, special, optimize, surrogate, sample)
  129 tests (RBF surrogates, multi-start NM, LHS, gamma, trapz, solve)
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
| WGSL shaders | 414 |
| Server line coverage | ~85% |
| Common line coverage | ~84% |
| Config line coverage | ~85% |
| Unsafe blocks | 35, all SAFETY documented |
| Production `todo!()` | 0 |
| Production mocks | 0 |
| Files over 1000 lines | 0 |

---

## What Works

- 414 WGSL shaders on any GPU (NVIDIA, AMD via Vulkan)
- Distributed LLM inference across machines (LAN TCP, BearDog encrypted)
- Hardware discovery (GPUs, NPUs, CPUs) -- pure Rust, no scripts
- NPU detection via /dev/akida* and IOMMU/VFIO sysfs
- JSON-RPC 2.0 + tarpc IPC over Unix sockets (26 methods)
- GPU job queue with priority and cross-gate routing
- Ollama model management (list, inference, load, unload)
- Cross-gate compute delegation (route by model locality, VRAM, queue depth)
- Science computing (Cholesky, eigh, linsolve, triangular solve, RBF, MD forces)
- Special functions (Bessel J0/J1/I0/K0, spherical harmonics, lgamma, erf)
- PRNG (xoshiro128**), sparse CSR matvec, LOO cross-validation
- FHE acceleration (21.1x speedup on RTX 3090)
- Smart auto-routing with user preference override
- CPU compute backends as universal fallback

## What Needs Evolution

- Test coverage: ~90% combined (3,688 core tests). Target reached.
- GPU dual-precision for RBF surrogate (~14× training speedup)
- VFIO backend for Akida NPU (eliminate C kernel module)
- NPU model pipeline (train/compile/deploy from Rust)
- validator 0.18 migration for API crate

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

## Scientific Middleware

**6 production-grade modules** — same math, any domain (physics, ML, graphics, audio):

```
barracuda::linalg      - Gauss-Jordan solver (f64)
barracuda::numerical   - Gradient, trapezoidal integration
barracuda::special     - Lanczos gamma, factorial
barracuda::optimize    - Nelder-Mead, multi-start global, bisection, eval cache
barracuda::surrogate   - RBF with 6 kernels (TPS, Gaussian, MQ, IMQ, Cubic, Quintic)
barracuda::sample      - Latin Hypercube Sampling, uniform random
```

**Tests**: 129/129 passing
**Quality**: Zero unsafe, clippy clean, pure Rust (no external RNG)
**Docs**: `specs/BARRACUDA_EVOLUTION_ROADMAP.md`, `docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md`

---

**Last Updated**: February 11, 2026
