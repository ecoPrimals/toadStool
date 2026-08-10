# ToadStool Specifications

## Scope & Aims

**toadStool is the sovereign hardware infrastructure primal.** It discovers,
profiles, and routes to every piece of compute silicon — GPU shader cores,
tensor cores, RT cores, TMUs, ROPs, rasterizers, tessellators, video encoders,
NPUs, CPUs, and future accelerators. No vendor SDKs. No proprietary drivers.
Pure Rust from application to bare metal.

When toadStool split into the compute trio (barraCuda + coralReef + toadStool),
it refocused: **toadStool is WHERE.** It owns hardware discovery, the measured
performance surface, and tolerance-based routing. Every piece of silicon we
unlock is new evolution for the whole trio and new science for the springs.

### The Compute Trio

```
Spring → barraCuda (WHAT)  "Yukawa force, tolerance 1e-14"
              │              Pure math. No hardware knowledge.
              ↓
         toadStool (WHERE)  "RTX 3090: shader DF64 for force eval,
              │              RT cores for neighbor search, ROPs for
              │              accumulation, TMU for potential table"
              ↓              Routes by measured performance surface.
         coralReef (HOW)    Compiles each sub-op to native ISA for
              │              the target unit selected by toadStool.
              ↓
         Hardware            Mixed command stream → all silicon active.
```

- **barraCuda** defines math with tolerance. Hardware-atheistic. Scales fp2→∞.
- **toadStool** discovers units, owns the performance surface, routes to cheapest
  hardware meeting tolerance. Graceful degradation when units unavailable.
- **coralReef** compiles to native ISA per target unit. Learns new targets.

Each primal evolves independently. A new hardware unit requires: toadStool
learns to discover it, coralReef learns to emit its instructions. barraCuda
and all springs are unchanged.

### Core Principles

1. **Every piece of silicon** — a modern GPU die has 8+ distinct hardware
   units, each a special-purpose computer. The rasterizer is a spatial query
   engine. The depth buffer is a min-reducer. The ROPs are scatter-adders.
   toadStool discovers, profiles, and routes to ALL of them.

2. **Tolerance-based routing** — springs specify mathematical tolerance
   (e.g. `1e-14`), not hardware targets. toadStool picks the cheapest
   hardware that meets the tolerance from its measured performance surface.

3. **Hardware atheism** — toadStool does not prefer any vendor or silicon
   type. It discovers what exists and routes work to the best available
   substrate for the requested tolerance.

4. **Self-knowledge only** — toadStool knows about hardware. It discovers
   other primals at runtime via capability-based IPC. No compile-time
   coupling. No hardcoded primal names in dispatch paths.

5. **Sovereign pipeline** — VFIO-based dispatch without Vulkan/CUDA API
   restrictions. This enables mixed command streams (compute + graphics +
   RT in one submission) that no existing framework supports.

6. **ecoBin v3.0** — single binary, pure Rust, cross-platform. Zero C
   application dependencies. Feature-gated optional backends for interop.

### The Hidden Computers on the GPU Die

Every unit was designed for graphics but actually computes a general math
function. Every spring discovery benefits all springs.

| Silicon Unit | What It Actually Computes | Science Use |
|---|---|---|
| **Shader Cores** | FP arithmetic (add, mul, fma) | Compute shaders, DF64 |
| **Tensor Cores** | Matrix multiply-accumulate (4×4→16×16) | CG solver, pairwise distances, convolution |
| **RT Cores** | BVH spatial index query | MD neighbor search, Monte Carlo transport |
| **TMUs** | 2D interpolated lookup (1 cycle) | EOS tables, activation functions, exp/log |
| **ROPs / Blend** | Per-pixel scatter-add / min / max | Histograms, particle deposition, Beer-Lambert |
| **Rasterizer** | Point-in-polygon + barycentric interp | Voxelization, FEM cell assignment, spatial binning |
| **Depth Buffer** | Per-pixel min reduction | Voronoi diagrams, distance fields, nearest-neighbor |
| **Tessellator** | Adaptive mesh subdivision | AMR, FEM mesh refinement |
| **Video Enc/Dec** | Block transform coding + motion estimation | Simulation compression, image registration |

### The Silicon Budget

A single RTX 3090 today delivers 0.33 TFLOPS native fp64. With DF64: 3.24
TFLOPS. With the full hardware budget — all units running in parallel on
different parts of the problem — the projection is **50-100 effective TFLOPS**.
That's a small HPC cluster in a single PCIe slot.

| Tolerance | Sufficient Precision | Cheapest Hardware (RTX 3090) | Throughput |
|---|---|---|---|
| 1e-2 | ~3 digits | FP16 tensor cores | ~142 TFLOPS |
| 1e-4 | ~5 digits | FP16 tensor cores | ~142 TFLOPS |
| 1e-7 | ~7 digits | FP32 shader cores | ~35.6 TFLOPS |
| 1e-10 | ~10 digits | TF32 tensor cores (accumulated) | ~71 TFLOPS |
| 1e-14 | ~14 digits | DF64 on FP32 shader cores | ~3.24-8.9 TFLOPS |
| 1e-16 | ~16 digits | Native FP64 | ~0.33 TFLOPS |

toadStool picks the row. coralReef compiles for it. Springs never see the table.

### Quality Gates

| Gate | Standard | Status |
|------|----------|--------|
| `cargo fmt` | Clean | ✅ |
| `cargo clippy --pedantic --nursery -D warnings` | 0 errors, 0 warnings | ✅ |
| `cargo doc --no-deps` | 0 warnings | ✅ |
| `#![warn(missing_docs)]` | All crates | ✅ |
| License | AGPL-3.0-or-later (SPDX on all files) | ✅ |
| Production `panic!()` / `.unwrap()` | 0 in non-test code | ✅ |
| Unsafe code | All documented with `// SAFETY:` | ✅ |
| Files > 1000 lines | 0 | ✅ |
| Hardcoded IPs/ports/primal names | 0 in production | ✅ |
| Test coverage | Target 90%, current ~85%+ (llvm-cov) | D-COV |
| Mocks in production | 0 (all `#[cfg(test)]` gated) | ✅ |

### Key Numbers (S374)

- **9,008+ lib-only** tests (0 failures), **126 JSON-RPC methods**, **16/16 native cross-arch**, **38/48 WASM crates**
- **3 hardware transports** — Display (DRM), Capture (V4L2), Serial
- **VFIO interface** — BAR0, DMA, power management (nvpmu), sovereign init pipeline
- **NPU dispatch** — Akida AKD1000/1500 (kernel, VFIO, userspace)
- **41 crates** `#![forbid(unsafe_code)]`, **5 crates** with narrow `#[allow(unsafe_code, reason)]` — **138 unsafe blocks** (all hw containment, all SAFETY-documented)
- **ecoBin v3.0 certified** — pure Rust, zero C application deps
- **Rust 2024 edition**, MSRV 1.85
- **47 workspace crates**, **39 external deps**, zero production panics/unwraps, zero dead deps

### Architecture

```
barraCuda (WHAT) → toadStool (WHERE) → coralReef (HOW) → Hardware
                     │                    │
                     │ Performance         │ VFIO transport
                     │ surface routing     │ (coral-driver: channels,
                     │ Per-unit dispatch   │  QMD, pushbuf, DMA)
                     │ Graceful fallback   │
                     │                    │
                     └────────────────────┘
                       VFIO interface
                       (device mgmt, BAR0 init, permissions,
                        pooling, thermal safety, multi-unit routing)
```

**Active evolution:** All-silicon pipeline (Phases A→D), sovereign compute
gap closure, performance surface database. See `ALL_SILICON_PIPELINE.md`.

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
| **[COMPUTE_DISPATCH_ENGINE.md](./COMPUTE_DISPATCH_ENGINE.md)** | Compute dispatch & GPU diesel engine — wgpu path live, VFIO PBDMA runlist gap | **Jun 2** | 🔄 Active |
| **[BARRACUDA_PRIMAL_BUDDING.md](./BARRACUDA_PRIMAL_BUDDING.md)** | barraCuda budding — fully untangled, zero cross-deps | **Mar 3** | ✅ Phase 5 Complete |
| **[ARCHITECTURE_DEMARCATION.md](./ARCHITECTURE_DEMARCATION.md)** | 4-layer chain: barraCuda → coralReef → toadStool + songBird (wire) | **Mar 12** | 🔄 Active |

**Tracker**: Fossilized as `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/TOADSTOOL_SOVEREIGN_COMPUTE_GAPS_S166.md` — remaining gaps tracked in [`../DEBT.md`](../DEBT.md)

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
| **[DISPATCH_WIRE_CONTRACT.md](./DISPATCH_WIRE_CONTRACT.md)** | Wire Standard L3 dispatch contract — compute.dispatch.submit schema | **Apr 12** | ✅ Active |
| **[DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md)** | DRM/input backend — display hardware control | Jan 19 | ✅ Phase 1 Complete |

### Research & Extensions (Future Work)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| ~~RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS~~ | Neuromorphic reservoir computing ops | Jan 29 | Transferred to barraCuda |
| ~~PRIMAL_CAPABILITY_SYSTEM~~ | Capability-based discovery | Nov 2025 | Superseded → `CAPABILITY_BASED_DISCOVERY_STANDARD.md` in wateringHole |

### All-Silicon Pipeline — Phased Evolution

*Every piece of silicon we unlock is new evolution for the whole trio and new
science for the springs. Driven by ludoSpring V24 audit. See `ALL_SILICON_PIPELINE.md`.*

| Phase | Description | Status | What It Unlocks |
|:-----:|-------------|:------:|-----------------|
| **A** | Sovereign compute dispatch (VFIO shader cores) | ✅ COMPLETE | Springs get sovereign shader compute without Vulkan/CUDA |
| **B** | Silicon discovery + performance surface database | ✅ COMPLETE | toadStool knows every unit on the die and measured throughput |
| **C** | Tolerance-based multi-unit routing | ✅ COMPLETE | Single workload splits across shader + tensor + RT + TMU + ROP |
| **D** | Mixed command streams (compute + graphics + RT) | 🔄 ACTIVE — VFIO sovereign path; blocked on PBDMA runlist | 50-100 effective TFLOPS per RTX 3090 — all silicon active |

**Phase B (S159)**: Foundation types landed — `SiliconUnit` enum (9 GPU
functional units), `PerformanceMeasurement` for spring experiment data,
`PerformanceSurfaceEntry` for the routing database, `SiliconCapabilities`
on `GpuAdapterInfo` for per-unit discovery. `compute.performance_surface.*`
JSON-RPC methods specified. Springs can begin reporting experiment data.

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

### Every Piece of Silicon

A GPU is not one computer — it's eight special-purpose computers on one die.
toadStool discovers all of them, measures their throughput for each operation
class, and routes work to the cheapest unit that meets the tolerance.

### Tolerance-Based Routing, Not Hardware Selection

Springs specify mathematical tolerance, not hardware targets. toadStool
selects the cheapest hardware unit that meets the requested precision.

```
tolerance 1e-14 → DF64 on FP32 shader cores (RTX 3090: ~3.24 TFLOPS)
tolerance 1e-7  → FP32 shader cores (RTX 3090: ~35.6 TFLOPS)
tolerance 1e-4  → FP16 tensor cores (RTX 3090: ~142 TFLOPS)
```

Graceful degradation: RT cores unavailable → compute BVH on shader cores.
Tensor cores unavailable → shader cores. The math is the same; the throughput
changes. Springs never see the difference except in speed.

### Runtime Discovery, Not Hardcoding

No vendor strings, no hardcoded ports, no compile-time assumptions about
what silicon is available.

### Self-Knowledge Only

toadStool knows about hardware. It discovers other primals at runtime via
capability-based IPC. No primal names in production dispatch paths.

### Sovereign by Default

The VFIO sovereign pipeline gives bare-metal GPU access without vendor
SDK restrictions. Mixed command streams — compute + graphics + RT in one
submission — that no existing framework supports.

### Deep Debt Resolution

Every workaround has an evolution path. Mocks are test-only. Stubs evolve
to complete implementations. External C dependencies evolve to pure Rust.

---

## Quick Links

- **Immediate work:** `../NEXT_STEPS.md`
- **Debt register:** `../DEBT.md`
- **Sovereign gaps:** Fossilized — `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/TOADSTOOL_SOVEREIGN_COMPUTE_GAPS_S166.md`
- **Architecture docs:** `docs/architecture/`
- **Hardware traits:** `crates/toadstool-core/src/` (NpuDispatch, NpuParameterController)
- **GPU backends:** `crates/runtime/universal/src/backends/`
- **Tests:** `cargo test --workspace`
- **Leverage guide:** Fossilized — `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/TOADSTOOL_LEVERAGE_GUIDE_S166.md`
- **All-silicon plan:** Fossilized — `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING_S166.md`
