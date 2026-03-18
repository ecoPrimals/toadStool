# ToadStool Specifications

## Scope & Aims

**toadStool is the sovereign hardware infrastructure primal.** It discovers,
manages, and orchestrates every piece of compute silicon available — GPU
shader cores, tensor cores, RT cores, TMUs, ROPs, rasterizers, NPUs, CPUs,
and future accelerators. No vendor SDKs. No proprietary drivers. Pure Rust
from application to bare metal.

### Core Principles

1. **Hardware atheism** — toadStool does not prefer any vendor or silicon
   type. It discovers what exists and routes work to the best available
   substrate for the requested tolerance.

2. **Self-knowledge only** — toadStool knows about hardware. It discovers
   other primals (barraCuda, coralReef, songBird, bearDog, nestGate) at
   runtime via capability-based IPC. No compile-time coupling. No hardcoded
   primal names in dispatch paths.

3. **Every piece of silicon** — driven by the ludoSpring V24 audit
   (`wateringHole/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md`): a modern
   GPU die has 8+ distinct hardware units, each a special-purpose computer.
   toadStool aims to discover, profile, and route to ALL of them.

4. **Tolerance-based routing** — springs specify mathematical tolerance
   (e.g. `1e-14`), not hardware targets. toadStool picks the cheapest
   hardware that meets the tolerance from its measured performance surface.

5. **Sovereign pipeline** — VFIO-based dispatch without Vulkan/CUDA API
   restrictions. This enables mixed command streams (compute + graphics +
   RT in one submission) that no existing framework supports.

6. **ecoBin v3.0** — single binary, pure Rust, cross-platform. Zero C
   application dependencies. Feature-gated optional backends (CUDA, OpenCL,
   Vulkan) for interop where needed.

### Quality Gates

| Gate | Standard | Status |
|------|----------|--------|
| `cargo fmt` | Clean | ✅ |
| `cargo clippy --pedantic --nursery` | 0 errors, 0 warnings | ✅ S158 |
| `cargo doc --no-deps` | 0 warnings | ✅ |
| `#![warn(missing_docs)]` | All 38 library crates | ✅ S158 |
| License | AGPL-3.0-or-later (SPDX on all files) | ✅ S158 |
| Production `panic!()` | 0 in non-test code | ✅ |
| Production `.unwrap()` | 0 in non-test code | ✅ |
| Unsafe code | All documented with `// SAFETY:` | ✅ |
| Files > 1000 lines | 0 | ✅ |
| Hardcoded IPs/ports | Centralized to `constants::network` | ✅ S158 |
| Test coverage | Target 90%, current ~83% | 🔄 D-COV |
| Mocks in production | 0 (all `#[cfg(test)]` gated) | ✅ |

### Key Numbers (S158)

- **56 workspace crates**, 1,896 `.rs` files, 565,323 lines
- **21,156+ tests** (0 failures)
- **96+ JSON-RPC methods** (`domain.operation` semantic naming)
- **3 hardware transports** — Display (DRM), Capture (V4L2), Serial
- **VFIO interface** — BAR0, DMA, power management (nvpmu)
- **NPU dispatch** — Akida AKD1000/1500 (kernel, VFIO, userspace)
- **29 crates** `#![forbid(unsafe_code)]`, remainder `#![deny(unsafe_code)]`
- **ecoBin v3.0 certified** — sysinfo replaced by `toadstool-sysmon`
- **Rust 2024 edition**, MSRV 1.85

### Architecture

```
barraCuda (WHAT) → coralReef (COMPILE+DISPATCH) → toadStool (WHERE+ORCHESTRATE)
                     │ VFIO transport              │ VFIO interface
                     │ (coral-driver: channels,    │ (device mgmt, BAR0 init,
                     │  QMD, pushbuf, DMA)         │  permissions, pooling,
                     │                             │  thermal safety, routing)
```

**Active evolution:** Sovereign compute gap closure, VFIO validation,
performance surface database, multi-unit routing. See `../SOVEREIGN_COMPUTE_GAPS.md`.

---

## Active Specifications

### Universal Precision (barraCuda-owned since S93)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[UNIVERSAL_PRECISION_ARCHITECTURE.md](./UNIVERSAL_PRECISION_ARCHITECTURE.md)** | Math is universal, precision is silicon — compilation pipeline design | **Feb 24** | ✅ barraCuda team |

**Note**: Precision strategy (DF64, f64/f32 validation) transferred to barraCuda team (S93). toadStool serves hardware capabilities.

### Sovereign Compute

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[SOVEREIGN_COMPUTE_EVOLUTION.md](./SOVEREIGN_COMPUTE_EVOLUTION.md)** | WGSL optimizer, VFIO interface/transport, mycelial ToadStool — master roadmap | **Mar 12** | 🔄 Active |
| **[BARRACUDA_PRIMAL_BUDDING.md](./BARRACUDA_PRIMAL_BUDDING.md)** | barraCuda budding — fully untangled, zero cross-deps | **Mar 3** | ✅ Phase 5 Complete |
| **[ARCHITECTURE_DEMARCATION.md](./ARCHITECTURE_DEMARCATION.md)** | 4-layer chain: barraCuda → coralReef → toadStool + songBird (wire) | **Mar 12** | 🔄 Active |

**Tracker**: [`../SOVEREIGN_COMPUTE_GAPS.md`](../SOVEREIGN_COMPUTE_GAPS.md) — remaining work before proceeding

### Performance & Evolution

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[HYBRID_FP64_CORE_STREAMING.md](./HYBRID_FP64_CORE_STREAMING.md)** | DF64 core streaming — hybrid FP32/FP64 | **Feb 23** | ✅ barraCuda team |
| ~~BARRACUDA_PARITY_ROADMAP~~ | Performance evolution, benchmarks | Feb 16 | Transferred to barraCuda |
| **[FP64_GPU_EVOLUTION.md](./FP64_GPU_EVOLUTION.md)** | Pure-GPU f64 math, fossil functions | **Feb 23** | ✅ barraCuda team |
| **[CROSS_PLATFORM_WORKLOADS.md](./CROSS_PLATFORM_WORKLOADS.md)** | Cross-vendor workload strategy (GPU + NPU) | Feb 13 | ✅ Current |
| ~~CROSS_VENDOR_BENCHMARK_SPEC~~ | Benchmark methodology and validation | Feb 13 | Transferred to barraCuda |

### NPU & Multi-Tenant

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[NPU_DRIVER_ARCHITECTURE.md](./NPU_DRIVER_ARCHITECTURE.md)** | Dual VFIO backend: NPU + GPU | **Mar 12** | ✅ Current |
| **[NPU_MULTI_TENANT_ARCHITECTURE.md](./NPU_MULTI_TENANT_ARCHITECTURE.md)** | Multi-tenant NPU resource partitioning | Feb 8 | ✅ Current |
| **[MULTITENANT_COMPUTE_ARCHITECTURE.md](./MULTITENANT_COMPUTE_ARCHITECTURE.md)** | Compute multi-tenancy across GPU/NPU/CPU | Feb 8 | ✅ Current |
| ~~BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2~~ | Universal tensor ops (CPU, GPU, NPU) | Feb 2 | Transferred to barraCuda |

### Hardware Transport & Dual-Fabric (S94b)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[DUAL_FABRIC_ARCHITECTURE.md](./DUAL_FABRIC_ARCHITECTURE.md)** | Multi-machine dual-fabric: hardware backbone (toadStool) + network plane (Songbird) | **Mar 3** | ✅ Specified |
| **[HARDWARE_TRANSPORT_SPEC.md](./HARDWARE_TRANSPORT_SPEC.md)** | `HardwareTransport` trait + VFIO clarification — data-plane vs control-plane | **Mar 12** | ✅ Implemented |
| **[DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md)** | DRM/input backend — display hardware control | Jan 19 | ✅ Phase 1 Complete |

### Research & Extensions (Future Work)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| ~~RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS~~ | Neuromorphic reservoir computing ops | Jan 29 | Transferred to barraCuda |
| **[PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md)** | Capability-based discovery | Nov 2025 | ✅ Implemented |

### Every Piece of Silicon — Future Evolution (S158)

*Driven by ludoSpring V24 `GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md`.*

| Phase | Description | Status | Dependency |
|:-----:|-------------|:------:|------------|
| **A** | Sovereign compute dispatch (VFIO shader cores) | 🔄 WIP | coralReef USERD_TARGET |
| **B** | Performance surface database (per-unit profiling) | 📋 Planned | Phase A, spring experiments |
| **C** | Multi-unit routing (shader + tensor + RT + TMU + ROP) | 📋 Planned | Phase B |
| **D** | Mixed command streams (compute + graphics + RT) | 📋 Planned | Phase C, coralReef MMA/draw emission |

**Target silicon**: Shader cores, tensor cores, RT cores, TMUs, ROPs, rasterizer, tessellator, video enc/dec — every functional unit on the GPU die. See `wateringHole/TOADSTOOL_LEVERAGE_GUIDE.md` Section 11 and `wateringHole/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md`.

---

## GPU-Resident Pipeline (Feb 16) ✅ COMPLETE

| # | Item | Status | Impact |
|:-:|------|:------:|--------|
| 1 | Max Abs Diff Reduction | ✅ DONE | Convergence check |
| 2 | Persistent Buffer Management | ✅ DONE | Pin for solver lifetime |
| 3 | Batched Bisection | ✅ DONE | GPU BCS pairing |
| 4 | Grid Quadrature GEMM | ✅ DONE | GPU Hamiltonian |
| 5 | Multi-Kernel Pipeline | ✅ DONE | Buffer chaining |

**Result**: GPU-resident SCF with zero CPU↔GPU round-trips during iteration.

## Future Work

### Unidirectional Pipeline Architecture (Feb 17 Exploration) 🆕

| Phase | Item | Status | Description |
|:-----:|------|:------:|-------------|
| 0 | **Design docs** | ✅ Done | 4 planning docs: GPU-Direct, Hardware Routing, Pipeline, Simulation |
| 1 | **GpuRingBuffer** | 📋 Planned | Ring buffer staging for input/output |
| 2 | **UnidirectionalPipeline** | 📋 Planned | Fire-and-forget input, batched async output |
| 3 | **Bandwidth throttling** | 📋 Planned | Simulate 90/10 split |
| 4 | **Benchmark vs traditional** | 📋 Planned | Measure speedup from eliminating round-trips |
| 5 | **HDMI + capture (hardware)** | ✅ Implemented | `DisplayTransport` (DRM Tx) + `CaptureTransport` (V4L2 Rx) + `TransportRouter` |

**Key insight**: 10 GB/s of completed results (not raw data) = 12.5M eigensolves/sec.

### Existing Future Work

| Category | Items | Status | Requesting Spring |
|----------|-------|--------|-------------------|
| **Fused Map-Reduce** | Single-dispatch map + sum | ✅ Complete | wetSpring |
| **cosine_similarity_f64** | MS2 spectral matching | ✅ Complete | wetSpring |
| **Batched ET₀ GPU** | N station-days in one dispatch | ✅ Template | airSpring |
| **Spatial Interpolation** | Kriging for sensor grids | ✅ Complete | airSpring, wetSpring |
| **1D Richards Solver** | Unsaturated flow PDE | 📋 Planned | airSpring |
| **Single-Dispatch Eigensolve** | Jacobi loop in shader | 📋 Planned | hotSpring Tier 1.1 |
| **NPU Pipeline** | Train/compile/deploy from Rust | 📋 Planned | ToadStool core |
| **Display Backend** | DRM/KMS + modesetting + page flip | ✅ Implemented | ToadStool display |
| **Hardware Transport** | `HardwareTransport` trait, frame protocol, router | ✅ Implemented | ToadStool core |
| **Dual-Fabric Architecture** | Hardware backbone + network plane multi-machine | ✅ Specified | ToadStool core |
| **Multi-Gate Routing** | Cross-machine GPU job routing | 📋 Planned | ToadStool core |
| **Multi-Link Striping** | Aggregate bandwidth across multiple HDMI outputs | 📋 Planned | ToadStool core |

---

## Completed Evolution (Reference)

### log_f64 Bug Fix (Feb 16) ✅

| Item | Status | Impact |
|------|:------:|--------|
| Coefficients halved | ✅ DONE | ~1e-3 → ~1e-15 precision |
| `zero + literal` pattern | ✅ DONE | Full f64 constant precision |
| Native builtins documented | ✅ DONE | log/exp REJECTED by NVVM |

### GPU-Resident Pipeline (Feb 16) ✅

| Target | Status |
|--------|:------:|
| Max Abs Diff Reduction | ✅ DONE |
| Persistent Buffer Management | ✅ DONE |
| Batched Bisection | ✅ DONE |
| Grid Quadrature GEMM | ✅ DONE |
| Multi-Kernel Pipeline | ✅ DONE |

### ecoBin Compliance (Feb 16) ✅

| Item | Status |
|------|:------:|
| Platform paths (XDG) | ✅ DONE |
| TOML config preferred | ✅ DONE |
| Semantic method naming | ✅ DONE |
| NPU Executor | ✅ DONE |
| Device Registry | ✅ DONE |

### MD Pipeline (Feb 14) ✅

| Target | Status |
|--------|:------:|
| f64 Yukawa force with PBC + PE | ✅ DONE |
| Cell-list neighbor search | ✅ DONE |
| Split Velocity-Verlet | ✅ DONE |
| Berendsen/Nosé-Hoover/Langevin thermostats | ✅ DONE |
| GPU observables (KE, RDF) | ✅ DONE |
| CPU observables (VACF, SSF, MSD) | ✅ DONE |
| PPPM/Ewald (full solver) | ✅ DONE |

### Math Primitives (Feb 15) ✅

| Primitive | Status |
|-----------|:------:|
| Hermite/Laguerre f64 | ✅ DONE |
| Broyden mixing | ✅ DONE |
| FD gradients | ✅ DONE |
| Weighted inner product | ✅ DONE |
| Science buffer limits | ✅ DONE |

---

## Archive

Historical documents have been moved to the `ecoPrimals/` fossil record. Current state is in active specs above.

---

## Design Principles

### Runtime Discovery, Not Hardcoding

toadStool discovers hardware at runtime and routes work based on measured
capabilities. No vendor strings, no hardcoded ports, no compile-time
assumptions about what silicon is available.

```rust
// WRONG — hardcoded vendor assumption
let cache_size = if vendor == "AMD" { 128_MB } else { 6_MB };

// RIGHT — runtime capability probe
let hierarchy = SubstrateMemoryHierarchy::probe(&device).await;
```

### Self-Knowledge Only

toadStool knows about hardware. It discovers other primals at runtime via
capability-based IPC (`get_socket_path_for_capability`). No primal names
in production dispatch paths. No compile-time cross-primal dependencies.

```rust
// WRONG — hardcoded primal name
let socket = get_socket_path_for_service("beardog");

// RIGHT — capability-based discovery
let socket = get_socket_path_for_capability(capabilities::CRYPTO);
```

### Tolerance-Based Routing

Springs specify mathematical tolerance, not hardware targets. toadStool
selects the cheapest hardware unit that meets the requested precision
from its measured performance surface.

```
tolerance 1e-14 → DF64 on FP32 shader cores (RTX 3090: ~3.24 TFLOPS)
tolerance 1e-7  → FP32 shader cores (RTX 3090: ~35.6 TFLOPS)
tolerance 1e-4  → FP16 tensor cores (RTX 3090: ~142 TFLOPS)
```

### Sovereign by Default

The VFIO sovereign pipeline gives bare-metal GPU access without vendor
SDK restrictions. This enables mixed command streams — compute + graphics
+ RT in one submission — that no existing framework supports.

### ecoBin v3.0 — Pure Rust

Zero C application dependencies. Feature-gated optional backends
(CUDA, OpenCL, Vulkan) for interop. Remaining libc is ecosystem-
transitive only (mio, tokio, wgpu).

### Deep Debt Resolution

Every workaround has an evolution path. Mocks are test-only. Stubs evolve
to complete implementations. External C dependencies evolve to pure Rust.
Large files are smart-refactored into coherent domain modules. Unsafe code
is evolved to safe Rust where possible, hardware-justified and documented
where not.

---

## Quick Links

- **Immediate work:** `../NEXT_STEPS.md`
- **Debt register:** `../DEBT.md`
- **Sovereign gaps:** `../SOVEREIGN_COMPUTE_GAPS.md`
- **Architecture docs:** `docs/architecture/`
- **Hardware traits:** `crates/toadstool-core/src/` (NpuDispatch, NpuParameterController)
- **GPU backends:** `crates/runtime/universal/src/backends/`
- **Tests:** `cargo test --workspace`
- **Leverage guide:** `wateringHole/TOADSTOOL_LEVERAGE_GUIDE.md`
- **All-silicon plan:** `wateringHole/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md`
