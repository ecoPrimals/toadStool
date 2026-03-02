# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | March 2, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. Capability-based discovery -- primals discover each other at runtime by capability, not name. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: **844 WGSL shaders** (zero orphans -- every shader wired to Rust, **zero f32-only** -- all f64 canonical with LazyLock downcast). **All math originates as WGSL** -- barracuda does not care about hardware; toadstool routes to the best substrate at runtime. CPU reference implementations gated behind `#[cfg(test)]`. f64 transcendentals (exp, log, pow, sin, cos, etc.) fully covered via `compile_shader_f64()` and `compile_shader_df64()` polyfill pipelines -- 28 functions, Cody-Waite range reduction, Horner polynomials, Lanczos gamma -- works on every GPU regardless of native f64 support. **No vendor math libraries** (libdevice/ocml) -- pure WGSL, ships with the crate, testable in CI without hardware. **Hybrid FP64 core streaming**: `Fp64Strategy` auto-selects between native f64 (compute-class GPUs) and DF64 double-float f32-pair arithmetic (~14 digits on FP32 cores) for consumer GPUs -- probe-informed via `fp64_strategy_probed()` with runtime f64 compile test. **Runtime f64 probe**: `basic_f64` compile probe catches NAK/NVVM that advertise `SHADER_F64` but cannot compile f64 WGSL -- forces DF64 fallback automatically. **AlphaFold2 primitives**: 17 Evoformer shaders (triangle updates, MSA attention, IPA, structure, FAPE loss, confidence). **HMM**: Forward + backward + Viterbi + log-domain f32/f64 dispatch. **Anderson coupling**: GPU-accelerated disorder Hamiltonian construction. **Grid search ops**: 2D surface fit, 3D brute-force minimum, band-edge extraction -- all wired via `ComputeDispatch` builder. **ESN multi-head**: 11-head constants, weight migration, f32 buffer system. **ODE universal precision**: `BatchedOdeRK4` template supports `Scalar`/`op_*` for f32/f64/df64/f16 compilation. **Sovereign Compiler**: naga-IR optimizer (FMA fusion, dead expression elimination) with SPIR-V passthrough -- end-to-end Rust compilation pipeline. **metalForge streaming**: `PipelineBuilder` → `StreamingPipeline` -- chained GPU dispatches without CPU readback. **ComputeDispatch builder**: fluent pipeline creation, 144 ops migrated (~14,000+ lines boilerplate removed). **NAK workgroup tuning**: architecture-aware workgroup sizes (Volta 64, Ada 256, RDNA 64). **Nuclear physics**: HFB GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders. **Lattice QCD**: 14 GPU shaders + host orchestration. **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, bio/genomics). All linalg GPU-dispatched: solve, cholesky, QR, SVD, LU. **25+ bio/evolution GPU ops**. **PDE solvers**: Crank-Nicolson, Richards unsaturated flow. **ESN GPU-train → NPU-deploy** pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

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
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo doc --workspace --no-deps` | 0 warnings |
| `cargo test --workspace` | 2,866 barracuda + 5,500+ workspace lib + integration tests |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| Five springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | 45 workspace-wide (2 barracuda, rest FFI/hardware/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production stubs | 0 -- all stubs evolved to real implementations or proper errors |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all evolved to formal `BLOCKED(reason)` markers |
| Dead code | ~400 lines removed; ~35 justified `#[allow(dead_code)]` (feature-gated, GPU fallbacks) |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) -- pure std::time, tokio-native, serde_yaml_ng |
| Hardcoded primal names | 0 inline strings -- all use `primals::*` constants or capability-based discovery |
| `async-trait` migration | 5 crates migrated to native AFIT (Rust 1.80+); remaining uses justified by `dyn Trait` dispatch |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- `DEFAULT_HOSTNAME` / `LOCALHOST_IPV4` constants |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| Orphan shaders | 0 -- all 844 WGSL shaders wired to Rust (37 DF64, 15 folding) |
| File size limit | All production files under 1000 lines (28+ god files smart-refactored into domain modules) |
| Test concurrency | All tests concurrent (`--test-threads=8`), zero `#[serial]`, zero fixed sleeps in non-chaos tests |
| Environment safety | All env-var tests use `temp_env` (thread-safe), zero `std::env::set_var` in tests |

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

**DF64 coverage**: 25 DF64 WGSL files, auto-selected at runtime by `Fp64Strategy`:
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
- `gelu_df64.wgsl` -- GELU activation (DF64 ML primitive, S70+)
- `sigmoid_df64.wgsl` -- Sigmoid activation (DF64 ML primitive, S70+)
- `softmax_df64.wgsl` -- Softmax (DF64 single-workgroup, S70+)
- `layer_norm_df64.wgsl` -- Layer normalization (DF64, S70+)
- `sdpa_df64.wgsl` -- Scaled dot-product attention (DF64, S70+)

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
BarraCUDA: 844 WGSL Shaders (MATH IS UNIVERSAL — PRECISION IS SILICON)
  All math originates as WGSL — barracuda does not care about hardware
  Dual-layer universal precision:
    Layer 1 (source): op_preamble — op_add/op_mul/Scalar alias → all precisions
    Layer 2 (compiler): naga-guided df64_rewrite — infix operators → bridge functions
  compile_shader_universal(): one shader → f16/f32/f64/df64 via pipeline
  compile_op_shader(): abstract ops work at ALL precisions without transformation
  compile_shader_f64() / compile_shader_df64() polyfill 28 transcendentals (no libdevice/ocml)
  downcast_f64_to_f32/f16/df64(): text-transform with sentinel protection
  SovereignCompiler: naga-IR → FMA fusion → DCE → df64 infix rewrite → SPIR-V passthrough
  Fp64Strategy: Native f64 (compute GPUs) | Hybrid DF64 (consumer GPUs) | Concurrent (dual validation)
  25 DF64 files: core (FMA), transcendentals (15 functions complete), GEMM, 4 force fields, SU(3), 5 lattice QCD, 5 ML (GELU/sigmoid/softmax/layernorm/SDPA)
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

### JSON-RPC Methods (44 total)

| Domain | Methods | Notes |
|--------|---------|-------|
| `toadstool.*` | `health`, `version`, `query_capabilities` | Canonical namespace |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | Canonical namespace |
| `resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | biomeOS neural API routing aliases |
| `compute.*` | `health`, `version`, `capabilities`, `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` | biomeOS Node Atomic aliases + GPU queue |
| `ai.*` | `local_inference`, `local_execute` | biomeOS ai_local capability |
| `ai.nautilus.*` | `status`, `observe`, `train`, `predict`, `screen`, `edges`, `shell.export`, `shell.import` | Evolutionary reservoir computing (feature-gated `nautilus`) |
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
|   +-- barracuda/                 844 WGSL shaders (shader-first, dual-layer universal precision), tensor ops, linalg, MD, HFB physics, lattice QCD, ESN, PDE, scientific middleware
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
| Clippy warnings (`-D warnings`) | 0 |
| Doc warnings | 0 |
| Build warnings | 0 |
| Unit tests (barracuda) | 2,866 |
| Shader-specific tests | 155 (unit + e2e + chaos + fault + naga validation) |
| WGSL shaders (barracuda) | 844 (zero orphans, shader-first, 37 DF64 + 15 folding + 200+ f64 — zero f32-only, all f64 canonical) |
| Lib tests (server) | 576 |
| Lib tests (core toadstool) | 1,340 |
| Lib tests (distributed) | 1,057 |
| Lib tests (common) | 923 |
| Lib tests (config) | 368 |
| Lib tests (CLI) | 209 |
| Lib tests (testing) | 104 |
| Lib tests (API) | 58 |
| Full workspace test time | ~6m30s (8 threads, GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | 45 workspace-wide (2 barracuda, rest FFI/hardware/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs / mocks | 0 -- all evolved to real implementations or proper errors |
| Production `todo!()`/`dbg!()` | 0 |
| Dead code removed | ~400 lines; ~35 justified `#[allow(dead_code)]` remain |
| Hardcoded localhost/ports/URLs in prod | 0 -- `DEFAULT_HOSTNAME` / `LOCALHOST_IPV4` constants |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core) |
| Five springs validation | 4,000+ acceptance checks |
| Default test timeout | 5s (unit: 2s, integration: 30s, chaos: 20s) |

---

## Evolution

**We are still evolving.** barracuda owns the math at all precisions. Springs migrate from local math to universal dispatch. All 5 spring handoffs absorbed. Remaining work is ComputeDispatch migration, DF64 architecture, and coverage.

### Active / Next
- **ComputeDispatch migration** -- 144/280+ ops migrated; ~139 legacy ops use manual BGL/BG boilerplate (incremental)
- **DF64 as default path** -- df64_rewrite as default precision, not fallback (groundSpring V35)
- **NpuDispatch trait** -- generic NPU interface (airSpring/wetSpring/groundSpring converge)
- **Test coverage** -- pushing toward 90% target; major coverage gains in CLI, server, API, monitoring, distributed
- **DF64 transcendental coverage** -- COMPLETE: 15 functions (exp, log, sin, cos, tan, sqrt, pow, asin, acos, atan, atan2, sinh, cosh, gamma, erf)
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining, architecture-specific peepholes

### Recently Completed
- **Sessions 84–86: ComputeDispatch Batches 5–7 + Deep Debt** -- 33 more ops → ComputeDispatch (144 total). hydrology.rs god-file refactored. experimental.rs stub → real probes. wgpu_backend.rs magic numbers → device limits. Full ops audit (corrected ~139 remaining).
- **Session 80: bingoCube Nautilus absorption + BatchedEncoder + Nelder-Mead GPU + fused_mlp** -- `barracuda::nautilus` module (7 files, 22 tests) — standalone evolutionary reservoir computing absorbed from bingoCube. `ai.nautilus.*` 8 JSON-RPC methods wired into daemon (feature-gated). `BatchedEncoder` for fused multi-op GPU pipelines (single `queue.submit()`). `fused_mlp` via BatchedEncoder. Batch Nelder-Mead GPU (N parallel optimizations). `StatefulPipeline<S>` for day-over-day state. `GpuDriverProfile` sin/cos F64 workarounds (Taylor preamble for NVK). `NeighborMode::PrecomputedBuffer` (2D/3D/4D lattice). `BatchedMultinomialGpu` alignment (cumulative_probs + seed). ComputeDispatch 76→95 ops (4 migration batches). Socket resolution consolidated.
- **Session 79: ESN MultiHeadEsn + ExportedWeights + SpectralAnalysis** -- 36-head `MultiHeadEsn` with `HeadGroup` variants, `head_disagreement()` uncertainty, `SpectralAnalysis` extensions (`spectral_bandwidth`, `classify_spectral_phase`), `ExportedWeights` aligned with hotSpring.
- **Session 75: Continued deep debt — module architecture + build streamlining** -- 6 god files smart-refactored: `primal_integration.rs` (1,163L→5 modules), `capability_provider.rs` (746L→5 modules), `primals/lib.rs` (580L→7 modules), `opencl_impl.rs` (831L→6 modules), `env_overrides.rs` (726L→9 modules), `os_layer/compat.rs` (766L→7 modules). Wildcard `pub use *` narrowed to explicit re-exports in 6 high-traffic crates (toadstool, distributed, server, gpu, universal, orchestration). `pollster` removed from toadstool + universal Cargo.toml. 3 evolved backends gated behind `#[cfg(test)]`. `TYPES_REFERENCE.md` updated with Module Structure Reference.
- **Session 74: Deep debt evolution — dependency + capability + resilience** -- `serde_yaml` → `serde_yaml_ng`. `async-trait` → native AFIT in 4 crates (performance, analytics, wasm, gpu). `pollster` → `tokio_block_on` in barracuda (removed dependency). Hardcoded primal names → capability-based language in CLI templates, JSON-RPC, error messages. `AuthResponse::standalone()`. Type aliases for capability-based naming (`OrchestrationConfigurator`, `PkiSecurityConfig`). Edge platform stubs → genuine hardware probing. Discovery stubs → real mDNS/k8s/docker/registry probing. God files: `workload.rs` (829L→2 modules), `unified.rs` (613L→3 modules), `precision/mod.rs` (816L→3 modules). GPU test resilience: 11 barracuda + 29 ml-inference + homomorphic tests wrapped with `catch_unwind` for NVK driver panics. `WgpuDevice::poll_safe()` for device-lost resilience. Doctest fixes across barracuda and showcase. Net -3,828 lines.
- **Session 71: Deep debt + GPU dispatch + ComputeDispatch acceleration** -- 6 GPU dispatch structs wired (HMM log, Bootstrap, Histogram, Kimura, Jackknife, Hargreaves). DF64 transcendental suite completed (15 functions). 32 ops migrated to ComputeDispatch (66 total). 6 large files smart-refactored. Hardcoded primal names → constants. 4 unsafe items reduced. External deps audited. Net -9,192 lines.
- **Session 70+++: Builder refactor + dead code removal + monitoring evolution** -- `builder.rs` (975 lines) smart-refactored into `builder/` module (mod.rs + profiler.rs + substrate.rs). Deleted deprecated `EcosystemCaller` (dead code, zero references). Monitoring collectors evolved from hardcoded stubs to real `sysinfo` (health thresholds, real CPU/memory/storage/network, session-aware performance metrics). NestGate `connect()` evolved from placeholder to real socket path resolution.
- **Session 70+/++: Cross-spring absorption + sovereignty + architecture** -- 7 new WGSL shaders (gelu/sigmoid/softmax/layernorm DF64, sdpa_df64, brent_f64 root-finding, seasonal_pipeline). 6 new GPU ops. 3 new stats modules. SimpleMLP with JSON weight serde. `Fp64Strategy::Concurrent`. Sovereignty: port 8084→`daemon_port()`, songbird→mdns discovery, capability-based adapter. +37 new tests.
- **Session 70: Deep debt + test concurrency evolution** -- 15 production stubs evolved to real implementations. All `std::env::set_var` → `temp_env`. All non-chaos sleeps removed. Full workspace: 6m30s, 0 failures.
- **Session 69++: Architecture evolution** -- metalForge streaming pipeline. manual_jsonrpc → pure_jsonrpc. 34 ops → ComputeDispatch. NAK workgroup tuning. 16 large files smart-refactored. rust-version 1.75→1.80.
- **Session 69/69+: Cross-spring absorption + deep debt** -- 5 spring handoffs absorbed (196 handoff files). 30+ new WGSL shaders. anyhow fully eliminated.
- **Session 68+++: Deep debt sweep** -- chrono eliminated (28 crates). Unsafe 47→45. ~400 lines dead code removed.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt

| ID | Description | Status |
|----|-------------|--------|
| D-CD | ComputeDispatch migration (~139 legacy ops) | Active -- incremental, 144 done |
| D-DF64 | DF64 as default path (not fallback) | Active -- architectural |
| D-NPU | NpuDispatch trait (generic NPU interface) | Active -- design phase |
| D-COV | Test coverage → 90% | Active -- major gains in server, CLI, API, monitoring |
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

**Last Updated**: March 2, 2026 -- Session 86. 844 WGSL shaders (37 DF64, 15 folding, 15-function DF64 transcendental suite). 2,866 barracuda tests. 5,500+ workspace lib tests (8,300+ total). 144 ops migrated to ComputeDispatch (~139 remaining). 44 JSON-RPC methods (8 `ai.nautilus.*`). `barracuda::nautilus` (evolutionary reservoir computing, 22 tests). `BatchedEncoder` + `fused_mlp`. Batch Nelder-Mead GPU. `StatefulPipeline`. `GpuDriverProfile` sin/cos workarounds. All quality gates green. Fully concurrent test suite. Rust 1.80+. Zero anyhow. Zero chrono. Zero pollster. Zero serde_yaml. Zero libc (akida-driver). Zero production stubs. 45 justified unsafe. 35+ god files smart-refactored. Capability-based discovery.
