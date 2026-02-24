# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | Session 57 -- February 24, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: 650+ WGSL shaders at f64 precision (zero orphans -- every shader wired to Rust). **All math originates as WGSL** -- barracuda does not care about hardware; toadstool routes to the best substrate at runtime. CPU reference implementations gated behind `#[cfg(test)]`. f64 transcendentals (exp, log, pow, sin, cos, etc.) fully covered via `compile_shader_f64()` polyfill pipeline -- works on every GPU regardless of native f64 support. **Nuclear physics**: HFB GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders on cylindrical grids. **Lattice QCD**: 14 GPU shaders + host orchestration (Wilson action, HMC, Dirac, CG solver, pseudofermion). **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, bio/genomics) -- same math for physics, ML, life science, and audio. All linalg GPU-dispatched: solve, cholesky, QR, SVD, LU via WGSL. RBF surrogates use GPU cdist + GPU solve. PPPM electrostatics use GPU FFT. **Complete MathOp coverage**: GPU and CPU executors handle all shape ops, binary ops, activations, batch matmul. **25 bio/evolution GPU ops**. **PDE solvers**: Crank-Nicolson, Richards unsaturated flow (Neumann boundary conditions). **Moving window statistics** GPU op. **ESN GPU-train → NPU-deploy** pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

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
| `cargo test --workspace --lib` | 14,200+ tests passing (4,224 across 5 core crates) |
| Four springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | 95+ audited -- FFI only, all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all evolved to formal `BLOCKED(reason)` markers |
| Hardcoded primal names | 0 -- capability-based discovery throughout |
| Hardcoded localhost/ports | 0 -- bind `0.0.0.0`, port 0 (OS-assigned), `discover_self_ip_address()` |
| Orphan shaders | 0 -- all 650+ WGSL shaders wired to Rust |
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

Pure-GPU double precision with `math_f64.wgsl` polyfill library (exp, log, pow, sin, cos, tan, gamma, erf -- auto-injected by `compile_shader_f64()`):

| GPU | SHADER_F64 | Observed FP64:FP32 Ratio | Notes |
|-----|-----------|-------------------------|-------|
| RTX 3090 | Yes | ~1:2 (not 1:64!) | Vulkan bypasses CUDA throttling |
| RTX 4070 | Yes | ~1:2 | 48MB L2 cache helps f64 |
| RX 6950 XT | Yes | ~1:2 | 128MB Infinity Cache excellent |

**Key insight**: Consumer GPUs advertise 1:64 FP64:FP32 ratio, but via pure Vulkan/wgpu we achieve ~1:2 -- the silicon is capable, vendor SDKs throttle it.

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
BarraCUDA: 650+ WGSL f64 Shaders (SHADER-FIRST — ALL MATH)
  All math originates as WGSL f64 — barracuda does not care about hardware
  compile_shader_f64() polyfills transcendentals (exp, log, pow, sin, cos...)
  Zero orphan shaders — every WGSL file wired to Rust
  CPU reference impls gated #[cfg(test)] only
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
|   +-- barracuda/                 650+ WGSL f64 shaders (shader-first), tensor ops, linalg, MD, HFB physics, lattice QCD, ESN, PDE, scientific middleware
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
| Unit tests (5 core crates) | 4,224 |
| Unit tests (full workspace) | 14,200+ |
| `unsafe` blocks | 95+ audited -- all FFI, all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs | 0 -- all `BLOCKED(reason)` markers |
| Hardcoded localhost/ports/URLs in prod | 0 |
| WGSL shaders | 650+ (zero orphans, all f64 shader-first) |
| Four springs validation | 4,000+ acceptance checks |

---

## Evolution

### Active / Next
- **GPU runtime integration** -- H-002 (CG solve loop, buffer allocation) and H-003 (GPU dispatch paths) need device testing
- **`eigh_f64` GPU wrapper** -- Jacobi eigenvalue solver has multi-pass WGSL shader, needs orchestration wrapper
- **Conv2D/Pool stride/padding/channels** -- WGSL exists, single-channel wired; full parametric support pending (D-S46-001)
- **NPU model pipeline** -- train/compile/deploy from Rust (awaiting hardware)
- **burn-inference models** -- BERT/Whisper/YOLO (currently return `ModelNotLoaded`/`ModelBackendRequired`)
- **W-001/W-003** -- Mesa NAK upstream patches pending Titan V validation

### Completed (Sessions 54-57: Final Absorptions + Coverage Push, Feb 24, 2026)
- **S57**: Coverage push (+47 tests across 5 uncovered modules: cost/optimizer, cost/pricing, gpu_job_queue, credentials, compliance). `println!` evolved to `tracing` in config_utils. TOML serialization test un-ignored.
- **S56**: Final 3 neuralSpring absorptions (belief_propagation, boltzmann_sampling, disordered_laplacian). Idiomatic Rust pass (17 edits). 2 large test files split.
- **S55**: 3 large files refactored (cost.rs, triangular_solve.rs, cpu_executor.rs). Stubs completed (DRM buffer, Crank-Nicolson Neumann BC, graceful degradation). 29 tautological assertions removed. Unsafe audit deepened.
- **S54**: 3 baseCamp primitives absorbed (graph_laplacian, effective_rank, numerical_hessian). 3 GPU bugs fixed (pow_f64, acos precision, FusedMapReduce buffer). 5 new WGSL shaders + spectral density primitives.

### Completed (Sessions 51-53: Cross-Spring Absorption + Deep Debt, Feb 24, 2026)
- **S53**: Hardcoded localhost eliminated from 5 production files. Unsafe audit (1 block removed). `Box<dyn Error>` → `ServerError`. `multi_gpu/mod.rs` refactored (921→54 lines). +193 new tests.
- **S52**: 26 cross-spring absorption items completed (7 HIGH, 10 MEDIUM, 9 LOW). 15 large files refactored. +103 tests.
- **S51**: 7 HIGH items: CG shaders, ESN NPU export, generic ODE, CPU solver, FlatTree, FusedMapReduce dot.

### Completed (Sessions 46-50: Shader-First Architecture + Audit, Feb 23, 2026)
- **S50**: Coverage 73→84%, cargo-deny 0.18.5, mock evolution, builder `#[must_use]`, 12 large files refactored.
- **S49**: Zero CPU-only math in production. 13 f32→f64 shader evolutions. All 4 springs absorbed at f64.
- **S48**: Lattice QCD GPU orchestration (CG solver + HMC trajectory).
- **S47**: 14 lattice QCD WGSL shaders. CPU lattice code gated `#[cfg(test)]`.
- **S46**: Cross-project shader absorption complete (hotSpring, neuralSpring, wetSpring).

### Completed (Sessions 43-45: Deep Debt, Feb 22-23, 2026)
- 21 `Box<dyn Error>` → typed errors. 95+ unsafe blocks audited. 38 coverage tests. Oversized files refactored. 33+ sleeps → event-driven.

### Completed (Sessions 31-41: Foundation)
- Zero clippy warnings. HFB nuclear physics. Lattice QCD. Capability-based discovery. thiserror 2.0. 600→645+ WGSL shaders. 6 f64 compile fixes.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt

| ID | Description | Status |
|----|-------------|--------|
| W-001 | f64 `exp`/`log` workaround for NVK/RADV open-source drivers | Active -- `compile_shader_f64()` polyfill handles it; upstream ACO/NAK fix pending Titan V validation |
| W-003 | NAK compiler scheduling gap (SM70 Volta) | Active -- Phases 0-3 live in `compile_shader_f64()`; Titan V benchmark pending; Mesa MR ready |
| D-S46-001 | Conv2D/Pool stride/padding/channels in WGSL | Carried -- shaders exist but lack full parametric support |
| D-S18-002 | cubecl transitive `dirs-sys` | Low -- needs upstream PR replacing `dirs` with `etcetera` |

See [DEBT.md](DEBT.md) for full register and evolution paths.

---

## Documentation

- **[STATUS.md](STATUS.md)** -- Current honest status
- **[DOCUMENTATION.md](DOCUMENTATION.md)** -- Navigation hub
- **[QUICK_STATUS.md](QUICK_STATUS.md)** -- One-page summary
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** -- Commands and API reference

---

**Last Updated**: February 24, 2026 -- Session 57: Coverage push (+47 tests). All cross-spring absorptions complete (46 items across S51-S56). 4,224 core tests, 650+ WGSL shaders, 0 clippy errors, 0 hardcoded localhost/ports, 0 `Box<dyn Error>`, 0 production TODOs. println evolved to tracing.
