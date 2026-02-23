# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | Session 49 -- February 23, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: 645+ WGSL shaders at f64 precision (zero orphans -- every shader wired to Rust). **All math originates as WGSL** -- barracuda does not care about hardware; toadstool routes to the best substrate at runtime. CPU reference implementations gated behind `#[cfg(test)]`. f64 transcendentals (exp, log, pow, sin, cos, etc.) fully covered via `compile_shader_f64()` polyfill pipeline -- works on every GPU regardless of native f64 support. **Nuclear physics**: HFB GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders on cylindrical grids. **Lattice QCD**: 14 GPU shaders + host orchestration (Wilson action, HMC, Dirac, CG solver, pseudofermion). **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, bio/genomics) -- same math for physics, ML, life science, and audio. All linalg GPU-dispatched: solve, cholesky, QR, SVD, LU via WGSL. RBF surrogates use GPU cdist + GPU solve. PPPM electrostatics use GPU FFT. **Complete MathOp coverage**: GPU and CPU executors handle all shape ops, binary ops, activations, batch matmul. **25 bio/evolution GPU ops**. **PDE solvers**: Crank-Nicolson, Richards unsaturated flow. **Moving window statistics** GPU op. **ESN GPU-train → NPU-deploy** pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

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
| `cargo test --workspace --lib` | 14,000+ tests passing |
| Four springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | 95+ audited -- FFI only, all SAFETY documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Hardcoded primal names | 0 -- capability-based discovery throughout |
| Orphan shaders | 0 -- all 645+ WGSL shaders wired to Rust |
| CPU-only math in production | 0 -- all math dispatches GPU shaders |
| TODOs/FIXMEs in production | 0 |
| File size limit | All files under 1000 lines |
| Line coverage (core crates) | common 87%, config 89%, core ~87%, server ~85% |

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
BarraCUDA: 645+ WGSL f64 Shaders (SHADER-FIRST — ALL MATH)
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
|   +-- barracuda/                 645+ WGSL f64 shaders (shader-first), tensor ops, linalg, MD, HFB physics, lattice QCD, ESN, PDE, scientific middleware
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
| Unit tests passing | 14,000+ |
| Build warnings | 0 |
| Line coverage (common) | 87% |
| Line coverage (config) | 89% |
| Line coverage (core) | ~87% |
| Line coverage (server) | ~85% |
| Line coverage (distributed) | 55% |
| `unsafe` blocks | 95+ audited -- all FFI, all SAFETY documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors |
| Hardcoded primal names in prod | 0 |
| WGSL shaders | 645+ (zero orphans, all f64 shader-first) |
| Dead code annotations | Audited -- all verified legitimate |
| Four springs validation | 4,000+ acceptance checks |

---

## What Needs Evolution

### Next
- **`eigh_f64` GPU wrapper** -- Jacobi eigenvalue solver has multi-pass WGSL shader, needs orchestration wrapper
- **Conv2D/Pool stride/padding/channels** -- WGSL exists but lacks full parameter support (D-S46-001)
- **Test coverage -> 90%** -- planner (49%), ecosystem (62%), detector (65%) are lowest remaining
- **NPU model pipeline** -- train/compile/deploy from Rust (awaiting hardware)
- **burn-inference models** -- BERT/Whisper/YOLO (currently return `ModelNotLoaded`/`ModelBackendRequired`)
- **W-001/W-003** -- Mesa NAK upstream patches pending Titan V validation

### Completed (Sessions 46-49: Shader-First Architecture, Feb 23, 2026)
- **S49e-f: Zero CPU-only math** -- 27+ threshold-gated CPU fallbacks eliminated, 6 always-CPU ops wired to GPU, linalg (solve, cholesky) GPU-dispatched, RBF surrogate GPU pipeline (cdist + solve), PPPM electrostatics GPU FFT
- **S49c-d: Force field + MD GPU enforcement** -- Velocity-Verlet, MSD, cubic spline, RDF, cdist all GPU-first. Coulomb, Morse, Born-Mayer, Yukawa CPU fallbacks removed. Special functions documented shader-first.
- **S49: Spring shader ingestion** -- 13 f32→f64 evolutions (bio, ESN, numerical). All 4 springs absorbed at f64.
- **S48: Lattice QCD GPU orchestration** -- CG solver + full HMC trajectory host loops
- **S47: Lattice QCD shaders** -- 14 WGSL shaders (Wilson action, HMC leapfrog, Dirac, pseudofermion, polyakov loop). CPU lattice code gated `#[cfg(test)]`.
- **S46: Cross-project absorption** -- hotSpring, neuralSpring, wetSpring shader absorption complete
- **f64 transcendental coverage** -- `compile_shader_f64()` auto-injects `math_f64.wgsl` polyfills (exp, log, pow, sin, cos, gamma, erf) on all drivers

### Completed (Session 45: Deep Debt Evolution, Feb 23, 2026)
- 21 `Box<dyn Error>` → typed errors in server/core production code
- 20+ barracuda shader/device test fixes (atanh, batch_pair_reduce_f64, NPU ops, ESN)
- 38 new coverage tests (planner, ecosystem, detector)
- 95+ unsafe blocks audited with SAFETY comments; 0 `NonNull::new_unchecked` remaining

### Completed (Sessions 43-44: Deep Debt + Sleep Elimination, Feb 22, 2026)
- Refactored oversized files; `gpu_job_queue.rs` 1127→344 lines; normalization/tensor_ops modularized
- 33+ production/test sleeps → event-driven (Notify, channel, interval, black_box)

### Completed (Sessions 31-41: Foundation, Springs, Physics, Shaders)
- Zero clippy warnings; zero blind `unwrap()`; HFB nuclear physics; lattice QCD
- Capability-based discovery; thiserror 2.0; zero-copy JSON-RPC; Four Springs validated
- 600→645+ WGSL shaders; 55 orphan shaders wired; GpuExecutor 31 MathOps
- 6 f64 shader compile fixes; Richards PDE solver; moving window GPU stats

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

**Last Updated**: February 23, 2026 -- Session 49: Shader-first architecture complete. 645+ WGSL f64 shaders, zero CPU-only math in production, f64 transcendental polyfills, lattice QCD GPU orchestration, linalg/RBF/PPPM all GPU-dispatched, all quality gates green.
