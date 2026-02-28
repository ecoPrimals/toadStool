# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | February 28, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: **661 WGSL shaders** (zero orphans -- every shader wired to Rust, **zero f32-only** -- all f64 canonical with LazyLock downcast). **All math originates as WGSL** -- barracuda does not care about hardware; toadstool routes to the best substrate at runtime. CPU reference implementations gated behind `#[cfg(test)]`. f64 transcendentals (exp, log, pow, sin, cos, etc.) fully covered via `compile_shader_f64()` and `compile_shader_df64()` polyfill pipelines -- 28 functions, Cody-Waite range reduction, Horner polynomials, Lanczos gamma -- works on every GPU regardless of native f64 support. **No vendor math libraries** (libdevice/ocml) -- pure WGSL, ships with the crate, testable in CI without hardware. **Hybrid FP64 core streaming**: `Fp64Strategy` auto-selects between native f64 (compute-class GPUs) and DF64 double-float f32-pair arithmetic (~14 digits on FP32 cores) for consumer GPUs -- probe-informed via `fp64_strategy_probed()` with runtime f64 compile test. **Runtime f64 probe**: `basic_f64` compile probe catches NAK/NVVM that advertise `SHADER_F64` but cannot compile f64 WGSL -- forces DF64 fallback automatically. **AlphaFold2 primitives**: 17 Evoformer shaders (triangle updates, MSA attention, IPA, structure, FAPE loss, confidence). **HMM**: Forward + backward + Viterbi. **Anderson coupling**: GPU-accelerated disorder Hamiltonian construction. **Grid search ops**: 2D surface fit, 3D brute-force minimum, band-edge extraction -- all wired via `ComputeDispatch` builder. **ESN multi-head**: 11-head constants, weight migration, f32 buffer system. **ODE universal precision**: `BatchedOdeRK4` template supports `Scalar`/`op_*` for f32/f64/df64/f16 compilation. **Sovereign Compiler**: naga-IR optimizer (FMA fusion, dead expression elimination) with SPIR-V passthrough -- end-to-end Rust compilation pipeline. **metalForge streaming**: `PipelineBuilder` → `StreamingPipeline` -- chained GPU dispatches without CPU readback. **ComputeDispatch builder**: fluent pipeline creation, 34 ops migrated (~3,739 lines boilerplate removed). **NAK workgroup tuning**: architecture-aware workgroup sizes (Volta 64, Ada 256, RDNA 64). **Nuclear physics**: HFB GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders. **Lattice QCD**: 14 GPU shaders + host orchestration. **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, bio/genomics). All linalg GPU-dispatched: solve, cholesky, QR, SVD, LU. **25+ bio/evolution GPU ops**. **PDE solvers**: Crank-Nicolson, Richards unsaturated flow. **ESN GPU-train → NPU-deploy** pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

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
| `cargo clippy --workspace --all-targets` | 1 warning (deprecated grpc fallback -- intentional) |
| `cargo doc --workspace --no-deps` | 0 warnings |
| `cargo test --workspace --lib` | 2,726+ barracuda + 21,599+ workspace tests |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| Five springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | 45 workspace-wide (2 barracuda, rest FFI/hardware/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all evolved to formal `BLOCKED(reason)` markers |
| Dead code | ~400 lines removed; ~35 justified `#[allow(dead_code)]` (feature-gated, GPU fallbacks) |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) -- pure std::time |
| Hardcoded primal names | 0 -- capability-based, `get_primal_default_port()` pattern |
| Hardcoded ports/localhost | 0 inline literals -- `DEFAULT_HOSTNAME` / `LOCALHOST_IPV4` constants |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| Orphan shaders | 0 -- all 661 WGSL shaders wired to Rust (21 DF64 files) |
| File size limit | All production files under 1000 lines |

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

**DF64 coverage**: 21 DF64 WGSL files, auto-selected at runtime by `Fp64Strategy`:
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
BarraCUDA: 661 WGSL Shaders (MATH IS UNIVERSAL — PRECISION IS SILICON)
  All math originates as WGSL — barracuda does not care about hardware
  Dual-layer universal precision:
    Layer 1 (source): op_preamble — op_add/op_mul/Scalar alias → all precisions
    Layer 2 (compiler): naga-guided df64_rewrite — infix operators → bridge functions
  compile_shader_universal(): one shader → f16/f32/f64/df64 via pipeline
  compile_op_shader(): abstract ops work at ALL precisions without transformation
  compile_shader_f64() / compile_shader_df64() polyfill 28 transcendentals (no libdevice/ocml)
  downcast_f64_to_f32/f16/df64(): text-transform with sentinel protection
  SovereignCompiler: naga-IR → FMA fusion → DCE → df64 infix rewrite → SPIR-V passthrough
  Fp64Strategy: Native f64 (compute GPUs) | Hybrid DF64 (consumer GPUs)
  21 DF64 files: core (FMA), transcendentals, GEMM, 4 force fields, SU(3), 5 lattice QCD
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
|   +-- barracuda/                 661 WGSL shaders (shader-first, dual-layer universal precision), tensor ops, linalg, MD, HFB physics, lattice QCD, ESN, PDE, scientific middleware
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

1. **Math is universal, precision is silicon** -- dual-layer universal precision. Layer 1 (source): `op_preamble` — abstract operations (`op_add`, `op_mul`, `Scalar` alias) that compile to all precisions. Layer 2 (compiler): naga-guided `df64_rewrite` — parses f64 WGSL, identifies infix operators by type, replaces with bridge functions routing computation through DF64. `compile_shader_universal()` and `compile_op_shader()` compile one source to f16/f32/f64/df64. `downcast_f64_to_f32/f16/df64()` with sentinel protection. 122 shader tests (unit + e2e + chaos + fault).
2. **f64 portability** -- `compile_shader_f64()` auto-injects software polyfills (exp, log, pow, sin, cos, etc.) on drivers lacking native support. `compile_shader_df64()` auto-injects DF64 core + transcendentals for f64-class precision on FP32 cores. Every GPU runs every shader.
3. **Modern idiomatic Rust** -- parameter-based APIs, zero global state mutation, thiserror 2.0
4. **Capability-based discovery** -- self-knowledge principle: only `PRIMAL_NAME` is known; everything else discovered at runtime
5. **Zero-copy hot paths** -- `Cow<'a, str>` with `#[serde(borrow)]` on JSON-RPC types, `serde_json::from_slice`, `bytes::Bytes` on binary payloads
6. **No hardcoding** -- well-known ecosystem constants for integration; core logic discovers by capability
7. **Mocks isolated to testing** -- all `#[cfg(test)]` gated; production code is complete implementations
8. **Honest documentation** -- no aspirational claims as facts; ML stubs return `ModelNotLoaded`/`ModelBackendRequired`
9. **Vendor-agnostic** -- WGSL over CUDA/ROCm, any GPU works
10. **Sovereign compute** -- no vendor lock-in, pure Rust core, no external math dependencies
11. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (45 blocks, all justified)
12. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy warnings | 0 |
| Doc warnings | 0 |
| Build warnings | 0 |
| Unit tests (barracuda) | 2,726+ |
| Shader-specific tests | 155 (unit + e2e + chaos + fault + naga validation) |
| WGSL shaders (barracuda) | 661 (zero orphans, shader-first, 21 DF64 + 200+ f64 — zero f32-only, all f64 canonical) |
| Unit tests (full workspace) | 21,599+ |
| `unsafe` blocks | 45 workspace-wide (2 barracuda, rest FFI/hardware/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production `todo!()`/`dbg!()` | 0 |
| Production TODOs | 0 -- all `BLOCKED(reason)` markers |
| Dead code removed | ~400 lines; ~35 justified `#[allow(dead_code)]` remain |
| Hardcoded localhost/ports/URLs in prod | 0 -- `DEFAULT_HOSTNAME` / `LOCALHOST_IPV4` constants |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core) |
| Five springs validation | 4,000+ acceptance checks |

---

## Evolution

**We are still evolving.** barracuda owns the math at all precisions. Springs migrate from local math to universal dispatch. All 5 spring handoffs absorbed. Remaining work is ComputeDispatch migration, DF64 architecture, and coverage.

### Active / Next
- **ComputeDispatch migration** -- 34/250 ops migrated; ~216 legacy ops use manual BGL/BG boilerplate (incremental)
- **DF64 as default path** -- df64_rewrite as default precision, not fallback (groundSpring V35)
- **NpuDispatch trait** -- generic NPU interface (airSpring/wetSpring/groundSpring converge)
- **Test coverage** -- barracuda at ~82% → 90% target
- **DF64 transcendental coverage** -- extend `asin_df64`, `acos_df64`, `atan_df64`, `sinh_df64`, `cosh_df64`, `gamma_df64`, `erf_df64`
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining, architecture-specific peepholes

### Recently Completed
- **Session 69++: Architecture evolution** -- metalForge streaming pipeline. manual_jsonrpc → pure_jsonrpc (full migration). 34 ops → ComputeDispatch (~3,739 lines removed). NAK workgroup tuning. 4 production stubs → real implementations. 16 large files smart-refactored. +100 new tests. rust-version 1.75→1.80.
- **Session 69/69+: Cross-spring absorption + deep debt** -- 5 spring handoffs absorbed (196 handoff files). 30+ new WGSL shaders (17 AlphaFold2, GPU Lanczos, airSpring batch ops, MD observables). anyhow fully eliminated from all ~30 crates. 13 S69 shaders → Rust dispatch via ComputeDispatch.
- **Session 68+++: Deep debt sweep** -- chrono eliminated (28 crates, 200+ files → `std::time`). Unsafe 47→45. ~400 lines dead code removed.
- **Session 68: Dual-layer universal precision** -- `op_preamble` + naga-guided `df64_rewrite`. 122 shader tests.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt

| ID | Description | Status |
|----|-------------|--------|
| D-CD | ComputeDispatch migration (~216 legacy ops) | Active -- incremental, 34 done |
| D-DF64 | DF64 as default path (not fallback) | Active -- architectural |
| D-NPU | NpuDispatch trait (generic NPU interface) | Active -- design phase |
| D-COV | Test coverage 82% → 90% | Active |
| W-001 | f64 transcendental polyfill for all drivers | Sovereign -- `compile_shader_f64()` handles 28 functions |
| W-003 | NAK compiler scheduling gap (SM70 Volta) | Active -- Phases 1+4 done; Titan V hw validation pending |
| D-S20-003 | neuralSpring `evolved/` migration (~2075 lines) | Blocked -- awaiting neuralSpring team |
| D-S18-002 | cubecl transitive `dirs-sys` | Blocked -- needs upstream PR |

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

**Last Updated**: February 28, 2026 -- 661 WGSL shaders. 2,726+ barracuda tests. 34 ops migrated to ComputeDispatch. All quality gates green. Rust 1.80+. Zero anyhow. Zero chrono. 45 justified unsafe. All files < 1000 lines.
