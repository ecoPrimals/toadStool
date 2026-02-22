# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | Session 41 -- February 22, 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: 600+ WGSL shaders (zero orphans -- every shader wired to Rust) as the primary math implementation. ToadStool dispatches to GPU or CPU based on hardware. Dedicated Conv2D, MaxPool2D, AvgPool2D compute shaders for neural network ops. **Nuclear physics**: HFB (Hartree-Fock-Bogoliubov) GPU-resident SCF suite -- 5 spherical + 6 axially-deformed shaders on cylindrical (ρ,z) grids; potentials, Hamiltonian, density, energy, BCS bisection, wavefunctions. **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde, lattice QCD, bio/genomics) -- same math for physics, ML, life science, and audio. **Complete MathOp coverage**: GPU and CPU executors handle all shape ops, binary ops, activations, and batch matmul. **TensorSession**: batched operation recording with single-submit execution (add, mul, fma, scale, matmul, relu, gelu, softmax, layer_norm, attention). **25 bio/evolution GPU ops**: ANI, dN/dS, HMM, DADA2, SNP, pangenome, quality filter, RF inference, ODE sweep, locus variance, pairwise Hamming/Jaccard/L2, spatial PD payoff, batch fitness, Hill gate, multi-objective fitness, swarm NN, k-mer histogram, taxonomy FC, UniFrac. **PDE solvers**: Crank-Nicolson (heat, Schrodinger), Richards unsaturated flow (van Genuchten-Mualem). **Moving window statistics** GPU op for IoT sensor streams. **ESN GPU-train → NPU-deploy**: export/import weights pipeline. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

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
| `cargo test --workspace --lib` | 5,965+ unit tests passing |
| Four springs validation | 4,000+ acceptance checks |
| `unsafe` blocks | FFI only (VFIO, DRM, alloc) -- all SAFETY documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Hardcoded primal names | 0 -- capability-based discovery throughout |
| Orphan shaders | 0 -- all 600+ WGSL shaders wired to Rust |
| TODOs/FIXMEs in production | 0 |
| File size limit | All files under 1000 lines |
| Line coverage (core crates) | common 87%, config 89%, core 79%, server 77% |

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

Pure-GPU double precision with `math_f64.wgsl` library:

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
BarraCUDA: 600+ WGSL Shaders (SHADER-FIRST)
  ALL math is WGSL primary -- ToadStool dispatches to GPU/CPU
  Zero orphan shaders -- every WGSL file wired to Rust
  NN ops: Conv2D, MaxPool2D, AvgPool2D (dedicated WGSL compute shaders)
  Middleware: linalg, numerical, special, stats, optimize, surrogate, sample, pde
  Bio: 25 GPU ops | Physics: 11 HFB shaders | Lattice QCD: 5 shaders
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
|   +-- barracuda/                 600+ WGSL shaders, tensor ops, NN ops, HFB nuclear physics, ESN, PDE solvers, scientific middleware
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

1. **Modern idiomatic Rust** -- parameter-based APIs, zero global state mutation, thiserror 2.0
2. **Capability-based discovery** -- self-knowledge principle: only `PRIMAL_NAME` is known; everything else discovered at runtime
3. **Zero-copy hot paths** -- `Cow<'a, str>` with `#[serde(borrow)]` on JSON-RPC types, `serde_json::from_slice`, `bytes::Bytes` on binary payloads
4. **No hardcoding** -- well-known ecosystem constants for integration; core logic discovers by capability
5. **Mocks isolated to testing** -- all `#[cfg(test)]` gated; production code is complete implementations
6. **Honest documentation** -- no aspirational claims as facts; ML stubs return `ModelNotLoaded`/`ModelBackendRequired`
7. **Vendor-agnostic** -- WGSL over CUDA/ROCm, any GPU works
8. **Sovereign compute** -- no vendor lock-in, pure Rust core
9. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (55 blocks audited)
10. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy warnings | 0 |
| Doc warnings | 0 |
| Unit tests passing | 5,965+ |
| Build warnings | 0 |
| Line coverage (common) | 87% |
| Line coverage (config) | 89% |
| Line coverage (core) | 79% |
| Line coverage (server) | 77% |
| Line coverage (distributed) | 55% |
| `unsafe` blocks | 55 -- all FFI, all SAFETY documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Hardcoded primal names in prod | 0 |
| WGSL shaders | 600+ (zero orphans) |
| Dead code annotations | Audited -- 38 remaining all verified legitimate |
| Four springs validation | 4,000+ acceptance checks |

---

## What Needs Evolution

### Next
- **Test coverage -> 90%** -- server and distributed crates need integration-style tests with mocks for I/O paths
- **NPU model pipeline** -- train/compile/deploy from Rust
- **burn-inference models** -- BERT/Whisper/YOLO (currently return `ModelNotLoaded`/`ModelBackendRequired`)
- **W-001/W-003** -- Mesa NAK upstream patches pending Titan V validation

### Completed (Sessions 39-41: Spring Absorption, PDE, f64 Bug Fix)
- **S41**: Fixed 6 f64 shaders using `compile_shader()` instead of `compile_shader_f64()` (wetSpring critical blocker -- ODE, GEMM, Hill, pair-reduce, tolerance-search, KMD)
- **S41**: `cpu_conv_pool` functions promoted to `pub` (unblocks neuralSpring LeNet-5); all 25 bio ops re-exported at crate root
- **S40**: 1D Richards equation solver (van Genuchten-Mualem soil hydraulics, Picard/Crank-Nicolson) for airSpring precision agriculture
- **S40**: Moving window statistics GPU op (mean/var/min/max over sliding windows for IoT sensor streams)
- **S40**: Dependency audit -- workspace already pure Rust; libc confined to akida VFIO ioctls
- **S39**: Absorbed 7 neuralSpring + 3 wetSpring + 11 hotSpring HFB physics shaders (600+ WGSL total)
- **S39**: S-14 Naive matmul tier removed (barrier-safe Tiled16 minimum); S-15 matmul hang fix; S-16 transpose dispatch fix
- **S39**: `GemmCachedF64::execute_to_buffer()`, `barracuda::math` module, `FlatTree` CSR, `sparse_eigh`, `quantize_affine_i8`

### Completed (Sessions 32-38: Deep Debt, Precision, Coverage)
- Zero clippy warnings workspace-wide; zero blind `unwrap()` in production
- TS-001/TS-003/TS-004 precision fixes; S-13 PooledBuffer race fix
- HFB nuclear physics (spherical + deformed) + lattice QCD absorption
- Capability-based discovery; thiserror 2.0; zero-copy JSON-RPC
- Conv2D/MaxPool2D/AvgPool2D WGSL; FHE fault injection; edge runtime
- 3,847+ workspace tests; coverage: common 87%, config 89%, core 79%, server 77%

### Completed (Sessions 31-31h)
- 55 orphan shaders wired, clippy clean sweep, dead code audit
- GpuExecutor 31 MathOps wired, CpuExecutor full dispatch, LU/QR/SVD refactoring
- Batched eigendecomposition, generalized eigensolver, TensorSession ML ops
- safetensors + GGUF weight loading, INT4/INT8 quantized WGSL shaders
- Four Springs absorption (hotSpring + wetSpring + neuralSpring + airSpring)

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt

| ID | Description | Status |
|----|-------------|--------|
| W-001 | f64 `exp`/`log` workaround for NVK/RADV open-source drivers | Active -- software fallback ~2x penalty; upstream ACO/NAK fix pending Titan V validation |
| W-003 | NAK compiler scheduling gap (SM70 Volta) | Active -- Phases 0-3 live in `compile_shader_f64()`; Titan V benchmark pending; Mesa MR ready |
| D-S18-002 | cubecl transitive `dirs-sys` | Low -- needs upstream PR replacing `dirs` with `etcetera` |
| D-S20-003 | neuralSpring `evolved/` (~2075 lines) | Carried -- barracuda APIs ready; neuralSpring team migration pending |

See [DEBT.md](DEBT.md) for full register and evolution paths.

---

## Documentation

- **[STATUS.md](STATUS.md)** -- Current honest status
- **[DOCUMENTATION.md](DOCUMENTATION.md)** -- Navigation hub
- **[QUICK_STATUS.md](QUICK_STATUS.md)** -- One-page summary
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** -- Commands and API reference

---

**Last Updated**: February 22, 2026 -- Session 41: 6 f64 shader compile bug fixes, Richards PDE solver, moving window GPU stats, 600+ WGSL shaders, all bio ops re-exported, four Springs validated.
