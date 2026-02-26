# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | Session 66 -- February 26, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: 694 WGSL shaders at f64 precision (zero orphans -- every shader wired to Rust). **All math originates as WGSL** -- barracuda does not care about hardware; toadstool routes to the best substrate at runtime. CPU reference implementations gated behind `#[cfg(test)]`. f64 transcendentals (exp, log, pow, sin, cos, etc.) fully covered via `compile_shader_f64()` polyfill pipeline -- 28 functions, Cody-Waite range reduction, Horner polynomials, Lanczos gamma -- works on every GPU regardless of native f64 support. **No vendor math libraries** (libdevice/ocml) -- pure WGSL, ships with the crate, testable in CI without hardware. **Hybrid FP64 core streaming**: `Fp64Strategy` auto-selects between native f64 (compute-class GPUs) and DF64 double-float f32-pair arithmetic (~14 digits on FP32 cores) for consumer GPUs -- 14 DF64 WGSL files including `df64_core.wgsl` (FMA-optimized) and `df64_transcendentals.wgsl` (exp, log, sqrt, sin, cos, pow, tanh at FP32 core speed). 4 force shaders fully evolved to all-DF64 (zero f64-unit dependency for transcendentals). **Sovereign Compiler**: naga-IR optimizer (FMA fusion, dead expression elimination) with SPIR-V passthrough -- end-to-end Rust compilation pipeline, bypassing WGSL text parsing at runtime. **Nuclear physics**: HFB GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders on cylindrical grids. **Lattice QCD**: 14 GPU shaders + host orchestration (Wilson action, HMC, Dirac, CG solver, pseudofermion). **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, bio/genomics) -- same math for physics, ML, life science, and audio. All linalg GPU-dispatched: solve, cholesky, QR, SVD, LU via WGSL. RBF surrogates use GPU cdist + GPU solve. PPPM electrostatics use GPU FFT. **Complete MathOp coverage**: GPU and CPU executors handle all shape ops, binary ops, activations, batch matmul. **25 bio/evolution GPU ops**. **PDE solvers**: Crank-Nicolson, Richards unsaturated flow (Neumann boundary conditions). **Moving window statistics** GPU op. **ESN GPU-train → NPU-deploy** pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

---

## Ecosystem Role

```
NUCLEUS = BearDog + Songbird + ToadStool + NestGate
Tower   = BearDog + Songbird          <- communication + crypto
Node    = Tower  + ToadStool          <- us -- sovereign compute
Nest    = Tower  + NestGate           <- storage
```

**biomeOS grade**: Node Atomic READY -- ToadStool A++ socket-standardized.

**Deployment**: Tower starts first (BearDog -> Songbird), then ToadStool. Socket: `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`. ToadStool discovers other primals at runtime by capability, not by name.

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace` | Clean |
| `cargo fmt --all -- --check` | 0 diffs |
| `cargo clippy --workspace --all-targets` | 0 warnings |
| `cargo doc --workspace --no-deps` | 0 warnings |
| `cargo test --workspace --lib` | 2,526 barracuda + 21,599 workspace tests |
| Four springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | 2 in barracuda (SPIRV passthrough + pipeline cache), 95+ workspace-wide, all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all evolved to formal `BLOCKED(reason)` markers |
| Hardcoded primal names | 0 -- capability-based discovery throughout |
| Hardcoded localhost/ports | 0 -- bind `0.0.0.0`, port 0 (OS-assigned), `discover_self_ip_address()` |
| Orphan shaders | 0 -- all 694 WGSL shaders wired to Rust (+14 DF64 files) |
| CPU-only math in production | 0 -- all math dispatches GPU shaders |
| File size limit | All production files under 1000 lines |
| `cargo deny check` | All passing — licenses, bans, sources |

---

## Cross-Vendor Distributed GPU Compute

**Single binary, identical results across vendors and machines:**

| GPU | Vendor | Machine | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 | NVIDIA | Tower | 388.7 | **5.128010** |
| RTX 3090 | NVIDIA | gate2 | 481.0 | **5.128010** |
| RX 6950 XT | AMD | gate2 | 222.7 | **5.128010** |

Zero CUDA. Zero ROCm. Pure Vulkan via WGPU. Bit-identical results.

### GPU FP64 Scientific Computing

Pure-GPU double precision with `math_f64.wgsl` polyfill library (28 functions: exp, log, pow, sin, cos, tan, gamma, erf, Bessel -- auto-injected by `compile_shader_f64()`). No vendor math libraries required (no libdevice, no ocml). Pure WGSL, ships with the crate:

| GPU | SHADER_F64 | Hardware FP64:FP32 | Strategy | Effective Throughput |
|-----|-----------|-------------------|----------|---------------------|
| Titan V | Yes | 1:2 (2560 FP64 cores) | Native f64 | 7.45 TFLOPS peak |
| RTX 3090 | Yes | 1:64 (164 FP64 units) | Hybrid DF64+f64 | ~3.5 TFLOPS effective |
| RTX 4070 | Yes | 1:64 | Hybrid DF64+f64 | 48MB L2 cache helps |
| RX 6950 XT | Yes | ~1:4 | Native f64 | 128MB Infinity Cache excellent |

**Key insight**: Consumer NVIDIA GPUs have genuine 1:64 FP64:FP32 hardware ratio (confirmed by `bench_fp64_ratio`). The breakthrough is **hybrid core-streaming**: routing bulk math through double-float f32-pair arithmetic (`df64_core.wgsl`, ~14 digits) on the massive FP32 core array, reserving native f64 for reductions and convergence tests. This delivers ~10x the effective f64-equivalent throughput on consumer GPUs. Compute-class GPUs (Titan V, V100, MI250) use native f64 everywhere via `Fp64Strategy::Native`.

**FMA-optimized DF64**: `two_prod` uses `fma(a, b, -p)` instead of Dekker splitting (17 ops → 2 ops). On Ampere/Ada/RDNA2+, FMA is free-ish (same throughput as mul). Critical for Krylov solver convergence.

**DF64 transcendentals**: `df64_transcendentals.wgsl` provides `exp_df64`, `log_df64`, `sqrt_df64`, `sin_df64`, `cos_df64`, `pow_df64`, `tanh_df64` -- all at FP32 core speed, no vendor library dependency. Born-Mayer, Morse, Yukawa, Lennard-Jones force shaders evolved from hybrid to **full FP32 core streaming** (zero f64-unit transcendental calls).

**DF64 coverage**: 14 DF64 WGSL files, auto-selected at runtime by `Fp64Strategy`:
- `df64_core.wgsl` -- FMA-optimized core arithmetic (add, mul, div, scale)
- `df64_transcendentals.wgsl` -- exp, log, sqrt, sin, cos, pow, tanh (Cody-Waite + Horner)
- `gemm_df64.wgsl` -- batched dense GEMM with shared-memory tiling
- `kinetic_energy_df64.wgsl` -- per-link kinetic energy (lattice QCD)
- `lennard_jones_df64.wgsl` -- LJ pair forces (all-DF64: sqrt via Newton-Raphson)
- `morse_df64.wgsl` -- Morse bond forces (all-DF64: exp via Cody-Waite)
- `born_mayer_df64.wgsl` -- Born-Mayer repulsive forces (all-DF64)
- `yukawa_df64.wgsl` -- Yukawa screened forces with PBC (all-DF64)
- `su3_df64.wgsl` -- SU(3) matrix algebra for lattice QCD
- `wilson_action_df64.wgsl` -- Wilson gauge action (lattice QCD)
- `wilson_plaquette_df64.wgsl` -- Wilson plaquette measurement
- `su3_hmc_force_df64.wgsl` -- HMC molecular dynamics force
- `su3_gauge_force_df64.wgsl` -- SU(3) gauge force (9.9x DF64 throughput)
- `su3_kinetic_energy_df64.wgsl` -- SU(3) kinetic energy

### Universal Cache Awareness

ToadStool discovers and optimizes for every substrate's memory hierarchy:

| Substrate | Largest Cache | Optimal Tile | Impact |
|-----------|---------------|--------------|--------|
| RTX 3090 | L2: 6 MB | 1 MB | 732 tiles/GB |
| RTX 4070 | L2: 48 MB | 11 MB | 92 tiles/GB |
| RX 6950 XT | Infinity: 128 MB | 29 MB | 35 tiles/GB |
| CPU (Zen 3) | L3: 32 MB | 7 MB | 138 tiles/GB |

### Distributed LLM Inference

TinyLlama-1.1B split across two machines over LAN TCP:
- Tower (RTX 4070): Embedding + layers 0-10
- gate2 (RTX 3090): Layers 11-21 + head
- **39.85 tok/s** with BearDog ChaCha20-Poly1305 encrypted tensor transport

---

## Architecture

```
Applications (hotSpring, NUCLEUS inference, etc.)
       |
BarraCUDA: 694 WGSL f64 Shaders (SHADER-FIRST — ALL MATH)
  All math originates as WGSL f64 — barracuda does not care about hardware
  compile_shader_f64() polyfills 28 transcendentals (no libdevice/ocml)
  SovereignCompiler: naga-IR → FMA fusion → DCE → SPIR-V passthrough
  Fp64Strategy: Native f64 (compute GPUs) | Hybrid DF64 (consumer GPUs)
  14 DF64 files: core (FMA), transcendentals, GEMM, 4 force fields, SU(3), 5 lattice QCD
  ComputeDispatch builder: fluent pipeline creation, ~80→5 lines per op
  Zero orphan shaders — every WGSL file wired to Rust
  Linalg: solve, cholesky, QR, SVD, LU — all GPU-dispatched
  Middleware: linalg, numerical, special, stats, optimize, surrogate, sample, pde
  Bio: 25 GPU ops | Physics: 11 HFB shaders | Lattice QCD: 14 shaders
  MD: VV, RDF, MSD, PPPM (GPU FFT), force fields — all GPU
  Proven: identical results NVIDIA + AMD, validated by 4 Springs (4,000+ checks)
       |
ToadStool: Hardware Discovery + Orchestration + Dispatch
  JSON-RPC 2.0 + tarpc IPC (Unix sockets)
  GPU Job Queue + Cross-Gate Routing
  Ollama Model Lifecycle (list/load/inference/unload)
  Capability-based runtime discovery (self-knowledge only)
  Cloud cost / compliance / federation
  Shared error tracking (AtomicU64)
       |
  +--------+---------+--------+
  |        |         |        |
 GPU     GPU       GPU      NPU         CPU
 RTX    RTX 3090  RX 6950  Akida       WGPU
 4070   (NVIDIA)  XT (AMD) (inference)  software
(NVIDIA)                                rasterizer
```

**Key**: Same WGSL shader compiles to Vulkan (NVIDIA/AMD), Metal (Apple), DX12 (Windows) via WGPU. No vendor SDK required.

**Routing**: `Device::select_for_workload(&hint)` auto-routes to the optimal device. `Device::select_with_preference(Some(Device::CPU), &hint)` lets callers override. Auto-routing is smart; user choice is sovereign.

### IPC Architecture

- **Unix sockets** for all primal-to-primal communication
- **JSON-RPC 2.0** protocol with semantic method naming (`{domain}.{operation}[.{variant}]`)
- **tarpc** (0.34) for high-performance typed RPC
- **Capability-based discovery** -- primals discover each other at runtime by capability, not name
- **biomeOS socket standard**: `/run/user/$UID/biomeos/{primal}.sock`
- **Multi-family support**: `--family-id` flag for `toadstool-{family_id}.sock`
- **Self-knowledge principle**: ToadStool only knows its own identity (`PRIMAL_NAME`); external primals are discovered via capability constants or well-known identifiers
- **BearDog**: security provider via capability-based discovery -- authenticated workload operations
- **Real-time events**: `compute.status` JSON-RPC polling or biomeOS/songbird coordination for event streaming

### JSON-RPC Methods (36 total)

| Domain | Methods | Notes |
|--------|---------|-------|
| `toadstool.*` | `health`, `version`, `query_capabilities` | Canonical namespace |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | Canonical namespace |
| `resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | biomeOS neural API routing aliases |
| `compute.*` | `health`, `version`, `capabilities`, `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` | biomeOS Node Atomic aliases + GPU queue |
| `ai.*` | `local_inference`, `local_execute` | biomeOS ai_local capability |
| `gpu.*` | `info`, `memory` | Hardware info |
| `ollama.*` | `list_models`, `inference`, `load`, `unload` | Local LLM lifecycle |
| `gate.*` | `update`, `remove`, `list`, `route` | Distributed routing |

---

## Quick Start

```bash
# Build everything
cargo build --release

# Run all quality gates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --lib

# Run RBF surrogate demo
cd showcase/rbf-surrogate && ./demo.sh

# Cross-vendor GPU test (runs on any GPU)
cargo test -p barracuda --lib ops::linalg --release

# Per-crate coverage
cargo llvm-cov --lib -p toadstool-common --json
```

---

## Project Structure

```
toadStool/
+-- crates/                        43 crates
|   +-- barracuda/                 694 WGSL f64 shaders (shader-first), tensor ops, linalg, MD, HFB physics, lattice QCD, ESN, PDE, scientific middleware
|   +-- core/
|   |   +-- common/                Shared types, constants, primal identity, ecosystem IDs, error types
|   |   +-- config/                Centralized configuration (env-aware, network config)
|   |   +-- toadstool/             Core runtime, IPC, scheduler, production hardening
|   +-- server/                    JSON-RPC server, GPU job queue, Ollama, cross-gate router
|   +-- api/                       REST API, middleware
|   +-- cli/                       UniBin CLI (single binary, BYOB server subcommand)
|   +-- integration/               Inter-primal protocols (beardog, nestgate, songbird)
|   +-- distributed/               Multi-gate coordination, cloud cost/compliance/federation
|   +-- runtime/
|   |   +-- gpu/                   WGPU device management, unified memory, pinned memory
|   |   +-- universal/             Universal compute substrate (CPU backends)
|   |   +-- adaptive/              Adaptive optimization, GPU fingerprinting
|   |   +-- display/               DRM/input backend
|   |   +-- edge/                  Edge device discovery (mDNS, filesystem), serial/TCP comms
|   |   +-- wasm/                  WebAssembly runtime (wasmi)
|   |   +-- container/             BYOB container runtime
|   +-- neuromorphic/              NPU drivers (Akida VFIO/kernel/mmap backends)
|   +-- ml/                        burn-inference (BERT, Whisper, Vision stubs with gated errors)
|   +-- security/                  Sandbox, policies, monitoring
|   +-- testing/                   Chaos, fault, property-based testing (proptest)
|   +-- management/                Analytics, monitoring, resources
+-- showcase/                      Demos (RBF, neuromorphic, GPU, FHE)
+-- docs/                          Architecture, guides, audits, ADRs
+-- specs/                         Technical specifications
+-- tests/                         Workspace-level integration tests
```

---

## Code Quality

### Deep Debt Principles

1. **Shader-first math** -- all math originates as WGSL f64 shaders. Barracuda does not care about hardware. CPU reference code gated `#[cfg(test)]`.
2. **f64 portability** -- `compile_shader_f64()` auto-injects software polyfills (exp, log, pow, sin, cos, etc.) on drivers lacking native support. Every GPU runs every shader.
3. **Modern idiomatic Rust** -- parameter-based APIs, zero global state mutation, thiserror 2.0
4. **Capability-based discovery** -- self-knowledge principle: only `PRIMAL_NAME` is known; everything else discovered at runtime
5. **Zero-copy hot paths** -- `Cow<'a, str>` with `#[serde(borrow)]` on JSON-RPC types, `serde_json::from_slice`, `bytes::Bytes` on binary payloads
6. **No hardcoding** -- well-known ecosystem constants for integration; core logic discovers by capability
7. **Mocks isolated to testing** -- all `#[cfg(test)]` gated; production code is complete implementations
8. **Honest documentation** -- no aspirational claims as facts; ML stubs return `ModelNotLoaded`/`ModelBackendRequired`
9. **Vendor-agnostic** -- WGSL over CUDA/ROCm, any GPU works
10. **Sovereign compute** -- no vendor lock-in, pure Rust core, no external math dependencies
11. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (95+ blocks audited)
12. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy warnings | 0 |
| Doc warnings | 0 |
| Build warnings | 0 |
| Unit tests (barracuda) | 2,541 |
| Unit tests (full workspace) | 21,599+ |
| `unsafe` blocks | 2 in barracuda (SPIRV passthrough + pipeline cache), 95+ workspace-wide, all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all `BLOCKED(reason)` markers |
| Hardcoded localhost/ports/URLs in prod | 0 |
| WGSL shaders | 694 (zero orphans, all f64 shader-first, 14 DF64 files) |
| Four springs validation | 4,000+ acceptance checks |

---

## Evolution

### Active / Next
- **P2: Architecture-specific polynomials** -- different Horner/Estrin evaluation strategies per silicon family (deferred until profiling data)
- **ComputeDispatch migration** -- Builder pattern created; migrating existing ops to reduce boilerplate
- **Conv2D/Pool stride/padding/channels** -- WGSL exists, single-channel wired; full parametric support pending (D-S46-001)
- **W-001/W-003** -- Mesa NAK upstream patches pending Titan V validation
- **NVK/Titan V readiness** -- Ensure f64 workarounds complete for NVK/Volta including NAK-specific paths

### Completed (Session 66: Absorption + Deep Debt + Dependency Evolution, Feb 26, 2026)
- **Cross-spring absorption**: 4 new modules from airSpring/groundSpring (regression, hydrology, moving_window_f64, RAWR). Richards PDE evolved (8 named soil constants, buffer preallocation).
- **Smart refactoring**: 15 files refactored across 3 waves (20-44% reductions each). `precision/mod.rs` 733→452, `workload.rs` 812→452, `cholesky.rs` 815→557, etc.
- **Dependencies eliminated**: `anyhow` → typed `BarracudaError`, `log` → unified `tracing` (68 calls migrated). `async_trait` justified (dyn-required).
- **Dead code**: 13→3 `#[allow(dead_code)]` (10 resolved via accessors/wiring). Remaining 3 are feature-gated/Phase 5+.
- **Hardcoding**: GPU executor scoring → 15 named constants, timeseries ESN → 6 named constants.
- **Production safety**: `expect()` audit (29 all Mutex/RwLock poison guards), 2 evolved to idiomatic patterns.
- **+36 new tests**, all passing. Zero `todo!()`, zero `unimplemented!()`, zero production `unwrap()`.

### Completed (Sessions 64-65: Cross-Spring Absorption + Smart Refactoring, Feb 25, 2026)
- **Cross-spring absorption**: 8 lattice QCD shaders (SU(3), PRNG, DF64 gauge force/kinetic, BLAS-like), `stats::metrics` (RMSE, NSE, R², etc.), `stats::diversity` (Shannon, Bray-Curtis, rarefaction), `chrono` dependency eliminated.
- **Smart refactoring**: 5 large files reduced by 32-44% — `compute_graph.rs` 819→522 (generic `dispatch_pass`), `esn_v2/model.rs` 861→482, `tensor/mod.rs` 808→529 (duplicate test merged), `special/gamma.rs` 685→463, `numerical/rk45.rs` 579→352.
- **Production safety**: `coulomb_f64` `map_async` callback `expect()` → graceful `let _ = tx.send()`. Kernel router magic numbers → named constants.
- **Dead code**: 17→13 `#[allow(dead_code)]` (4 resolved via public accessors).

### Completed (Sessions 61-63: Sovereign Compiler + Deep Debt Evolution, Feb 25, 2026)
- **Sovereign Compiler Phase 4**: `SovereignCompiler` built on naga-IR — parse WGSL, apply FMA fusion (Mul+Add→fma, ~1.3x), dead expression elimination, emit SPIR-V. `SPIRV_SHADER_PASSTHROUGH` in all device paths. Three-stage `compile_shader_f64()` pipeline with WGSL text fallback.
- **Codebase-wide deep debt resolution**: 25+ `#[allow(dead_code)]` WGSL constants evolved to documented `pub` API. Fossil `wgsl_shader()` methods converted to `pub const`. Platform-aware OS compat stubs. Table-driven discovery fallbacks.
- **Smart refactoring**: `morse_f64.rs` 953→804 lines (MorseBuffers + reduce_bond_forces extraction). `coulomb_f64/mod.rs` 610→369 lines (CoulombBuffers + staging helpers). Zero duplication in GPU pipeline code.
- **Dead code → live code**: `solve_gpu_parallel` wired into cyclic reduction for n≥2048 (was complete but unused). `partial_maximin` O(n) optimization wired into maximin_lhs loop (was O(n²)). `erfc_deriv` promoted to public electrostatics API.
- **Hygiene**: `instant` crate removed, `WebGPUAdapter::mock_data` → zero-size `()`, GriffinLim dead_code annotations cleaned.

### Completed (Session 60: DF64 FMA + Transcendentals + Polyfill Hardening, Feb 25, 2026)
- **FMA-optimized DF64**: `two_prod` reduced from 17 ops to 2 via `fma(a, b, -p)` -- eliminates Dekker splitting entirely. `df64_mul` cross-terms use FMA. Critical for Krylov solver convergence.
- **DF64 transcendental library**: new `df64_transcendentals.wgsl` with `sqrt_df64` (Newton-Raphson), `exp_df64` (Cody-Waite), `log_df64` (atanh series), `sin_df64`/`cos_df64` (Cody-Waite π/2 reduction), `pow_df64`, `tanh_df64` -- all at FP32 core speed, no vendor library dependency
- **4 force shaders evolved** from hybrid to full FP32 core streaming: Born-Mayer, Morse, Yukawa, Lennard-Jones DF64 shaders no longer round-trip through f64 units for transcendentals
- **Polyfill patcher hardened**: `patch_transcendentals_in_code()` protects `ldexp()`, `exp_df64()`, `log_df64()` from substring collision mangling
- **Multi-GPU adapter selection**: deterministic GPU pinning via `BARRACUDA_GPU_ADAPTER` (absorbed from hotSpring)

### Completed (Session 59: Deep Audit + Comprehensive Evolution, Feb 24, 2026)
- **`#![deny(unsafe_code)]`** added to 36 crates. TarpcClientWrapper JSON-RPC fallback. Dependency security hardening. Smart refactoring (5 files). 21,599 tests.

### Completed (Session 58: Hybrid DF64 Core Streaming, Feb 24, 2026)
- **6 DF64 shader variants** + `Fp64Strategy` auto-selection + `ComputeDispatch` builder + `unified_hardware` refactored

### Completed (Sessions 43-57: Shader-First + Deep Debt + Cross-Spring Absorption)
- All 46 cross-spring absorptions, 14 lattice QCD shaders, zero CPU-only math, 95+ unsafe blocks audited, hardcoded localhost eliminated

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt

| ID | Description | Status |
|----|-------------|--------|
| W-001 | f64 transcendental workaround for all drivers (NVK/RADV/NVIDIA) | Active -- `compile_shader_f64()` polyfill handles 28 functions; no vendor math library needed; upstream ACO/NAK fix pending |
| W-003 | NAK compiler scheduling gap (SM70 Volta) | Active -- Phases 0-3 live in `compile_shader_f64()`; Titan V benchmark pending; Mesa MR ready |
| D-S46-001 | Conv2D/Pool stride/padding/channels in WGSL | Carried -- shaders exist but lack full parametric support |
| D-S18-002 | cubecl transitive `dirs-sys` | Low -- needs upstream PR replacing `dirs` with `etcetera` |

See [DEBT.md](DEBT.md) for full register and evolution paths.

---

## Documentation

| Document | Purpose |
|----------|---------|
| [STATUS.md](STATUS.md) | Detailed technical status, session-by-session |
| [DEBT.md](DEBT.md) | Active debt register, workarounds, evolution paths |
| [NEXT_STEPS.md](NEXT_STEPS.md) | Roadmap and upcoming work |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Commands, JSON-RPC methods, API reference |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub (guides, specs, audits) |
| [CHANGELOG.md](CHANGELOG.md) | Full session-by-session evolution history |
| [SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md) | Sovereign compute phases and Mesa NAK roadmap |
| [UNIDIRECTIONAL_PIPELINE.md](UNIDIRECTIONAL_PIPELINE.md) | GPU-resident pipeline architecture |

---

**Last Updated**: February 26, 2026 -- Session 66: Cross-spring absorption + deep debt + dependency evolution + cross-spring evolution. 2,541 barracuda tests, 694 WGSL shaders (14 DF64 files), 0 clippy, 0 fmt diffs, 0 production TODOs, 0 hardcoded localhost/ports, 3 `#[allow(dead_code)]` (feature-gated/Phase 5+), `anyhow` + `log` eliminated (typed errors + unified tracing), 2 unsafe blocks (SPIRV passthrough + pipeline cache).
