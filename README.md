# ToadStool + BarraCUDA

**Sovereign Distributed Compute** | Pure Rust | ecoBin | February 2026

---

## What Is This?

- **ToadStool** -- Hardware infrastructure primal. Discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. ecoBin compliant: single binary, pure Rust, cross-architecture, cross-platform.
- **BarraCUDA** -- Universal math engine. **Shader-first architecture**: 396 WGSL shaders as the primary math implementation. ToadStool dispatches to GPU or CPU based on hardware. When fp64 GPUs are available, seamless transition. 18 special function shaders (Hermite, Legendre, Laguerre, Bessel, etc.), 3 sampling shaders (Sobol, LHS, random_uniform). **Scientific computing middleware** (linalg, numerical, special, stats, optimize, surrogate, sample, pde) — same math for physics, ML, graphics, and audio. Vendor-agnostic -- same binary, same results on NVIDIA, AMD, Intel.

---

## Quality Gates (February 12, 2026)

| Gate | Status |
|------|--------|
| `cargo build --workspace` | Clean, 0 warnings |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --workspace` | **0 warnings** (down from 453) |
| `cargo test --workspace` | **15,600+ passed, 0 failed** |
| `unsafe` blocks | 100% documented with `// SAFETY:` comments |
| File size | All production files appropriately structured |
| Scientific middleware | 200+ tests, 100% passing, 0 unsafe blocks |

---

## Cross-Vendor Distributed GPU Compute

**Single binary, identical results across vendors and machines:**

| GPU | Vendor | Machine | GFLOPS | Checksum |
|-----|--------|---------|--------|----------|
| RTX 4070 | NVIDIA | Tower | 388.7 | **5.128010** |
| RTX 3090 | NVIDIA | gate2 | 481.0 | **5.128010** |
| RX 6950 XT | AMD | gate2 | 222.7 | **5.128010** |

Zero CUDA. Zero ROCm. Pure Vulkan via WGPU. Bit-identical results.

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
BarraCUDA: 396 WGSL Shaders (SHADER-FIRST)
  ALL math is WGSL primary — ToadStool dispatches to GPU/CPU
  18 special function shaders, 3 sampling shaders
  Middleware: linalg, numerical, special, stats, optimize, surrogate, sample, pde (200+ tests)
  Proven: identical results NVIDIA + AMD
       |
ToadStool: Hardware Discovery + Orchestration + Dispatch
  JSON-RPC 2.0 + tarpc IPC (Unix sockets)
  GPU Job Queue + Cross-Gate Routing
  Ollama Model Lifecycle (list/load/inference/unload)
  Capability-based runtime discovery
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
- **tarpc** for high-performance typed RPC
- **Capability-based discovery** -- primals discover each other at runtime by capability, not name
- **biomeOS socket standard**: `/run/user/$UID/biomeos/{primal}.sock`
- **Multi-family support**: `--family-id` flag for `toadstool-{family_id}.sock`

### JSON-RPC Methods (26 total)

| Domain | Methods |
|--------|---------|
| `toadstool.*` | `health`, `version`, `query_capabilities` |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` |
| `compute.*` | `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` |
| `gpu.*` | `info`, `memory` |
| `ollama.*` | `list_models`, `inference`, `load`, `unload` |
| `gate.*` | `update`, `remove`, `list`, `route` |

---

## Quick Start

```bash
# Build everything
cargo build --release

# Run all quality gates
cargo fmt --all -- --check
cargo clippy --workspace
cargo test --workspace

# Run RBF surrogate demo
cd showcase/rbf-surrogate && ./demo.sh

# Cross-vendor GPU test (runs on any GPU)
cargo test -p barracuda --lib ops::linalg --release
```

---

## Project Structure

```
toadStool/
+-- crates/
|   +-- barracuda/             -- 396 WGSL shaders, tensor ops
|   +-- core/
|   |   +-- common/            -- Shared types, constants, discovery
|   |   +-- config/            -- Centralized configuration (env-aware)
|   |   +-- toadstool/         -- Core runtime, IPC, scheduler
|   +-- server/                -- JSON-RPC server, GPU job queue, Ollama, cross-gate router
|   +-- api/                   -- REST API, middleware
|   +-- cli/                   -- UniBin CLI (single binary)
|   +-- integration/           -- Inter-primal protocols (beardog, nestgate, songbird)
|   +-- distributed/           -- Multi-gate coordination, crypto
|   +-- runtime/
|   |   +-- gpu/               -- WGPU device management
|   |   +-- universal/         -- Universal compute substrate (CPU backends implemented)
|   |   +-- adaptive/          -- Adaptive optimization
|   |   +-- display/           -- DRM/input backend
|   +-- neuromorphic/          -- NPU drivers (Akida)
|   +-- security/              -- Sandbox, policies, monitoring
|   +-- testing/               -- Chaos, fault, property testing
|   +-- management/            -- Analytics, monitoring, resources
+-- showcase/                  -- Demos (RBF, neuromorphic, GPU, FHE)
+-- docs/                      -- Architecture, guides, audits
+-- specs/                     -- Technical specifications
+-- tests/                     -- Workspace-level integration tests
```

---

## Code Quality

### Deep Debt Principles

1. **Modern idiomatic Rust** -- parameter-based APIs, zero global state mutation
2. **Fully concurrent** -- scoped mutex for env tests, event-driven async, no sleep-based sync
3. **Zero-copy hot paths** -- `serde_json::from_slice`, `String::from`, pre-sized buffers
4. **No hardcoding** -- runtime discovery, capability-based, named constants for ports
5. **Mocks isolated to testing** -- production code is complete implementations
6. **Honest documentation** -- no aspirational claims as facts
7. **Vendor-agnostic** -- WGSL over CUDA/ROCm, any GPU works
8. **Sovereign compute** -- no vendor lock-in, pure Rust core (num_cpus FFI eliminated)
9. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments
10. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy warnings | 0 (from 453) |
| Tests passing | 15,490+ (3,688 core) |
| Tests failing | 0 |
| Build warnings | 0 |
| Server line coverage | ~85% |
| Common line coverage | ~84% |
| Config line coverage | ~85% |
| `unsafe` blocks | 35 blocks, 100% documented with `// SAFETY:` |
| File size | All production files under 1000 lines |
| Production `todo!()` | 0 |
| Production mocks | 0 (TestExecutor in test-only code) |
| `#[serial]` in tests | 0 (replaced with scoped Mutex) |
| Sleep-based test sync | 0 in server tests (event-driven) |

---

## What Needs Evolution

- **VFIO NPU backend** -- eliminate C kernel module, pure Rust via `/dev/vfio/*`
- **NPU model pipeline** -- train/compile/deploy from Rust, replace Python cnn2snn
- **Model weight loading** -- need safetensors/GGUF loader (eliminate PyTorch dependency)
- **Multi-GPU orchestration** -- `WgpuDevice::new()` picks one device; need `DevicePool`
- **INT4/INT8 quantization** -- f32 only; need quantized WGSL shaders
- **Cross-gate mesh relay** -- gate.* routing defined, needs Songbird mesh transport

---

## Recent Evolutions (Feb 2026)

- **CPU Tensor Ops** -- tiled matmul, conv2d, pooling with proper cache-blocking
- **CUDA Backend** -- real PTX kernel execution for matmul, reduction ops
- **Unified Memory** -- OpenCL/Vulkan backends now use wgpu fallback (pure Rust)
- **Security Providers** -- Unix socket IPC with JSON-RPC 2.0 for inter-primal security
- **Clippy Clean** -- all barracuda warnings resolved (is_multiple_of, approx_constant)

## Phase 3 Direction (from hotSpring)

BarraCUDA validated against scipy/numpy: **121/121 tests passed** for special functions, linalg, optimizers, and MD forces. Evolution shifts from breadth to depth:

1. **f64 Linalg Bridge** -- expose CPU f64 versions of all GPU ops (eigh, cholesky, LU, QR, SVD)
2. **Auto-Dispatch** -- intelligent CPU/GPU routing based on input size (single-point GPU dispatch causes 90× slowdown)
3. **Cache Persistence** -- save/load EvaluationCache for warm-starting across runs
4. **Heterogeneous Pipelines** -- declarative multi-tier compute (filter cascades saved 91.9% of expensive HFB evaluations)

See `specs/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md` for full roadmap.

---

## Documentation

- **[STATUS.md](STATUS.md)** -- Current honest status
- **[DOCUMENTATION.md](DOCUMENTATION.md)** -- Navigation hub
- **[QUICK_STATUS.md](QUICK_STATUS.md)** -- One-page summary
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** -- Commands and API reference

---

**Last Updated**: February 12, 2026
