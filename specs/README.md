# ToadStool Specifications

## Current Status (March 5, 2026 — Deep Debt Execution)

**Quick Start:**
- **`../README.md`** — Project overview, architecture, key achievements
- **`../STATUS.md`** — Detailed status with quality gates
- **`UNIVERSAL_PRECISION_ARCHITECTURE.md`** — Math is universal, precision is silicon

**Key Numbers:**
- **19,109 workspace tests** (0 failures, all concurrent)
- **61+ JSON-RPC methods** (dynamically built from semantic registry)
- **3 hardware transports** — DisplayTransport (DRM), CaptureTransport (V4L2), SerialTransport
- **Capability-based discovery** — sovereignty: all production callers migrated
- **ecoBin pure-rust verified** — zero C FFI deps
- **Rust 1.82+** — `is_some_and`, `div_ceil`, modern idiomatic patterns

**Latest (Mar 5 — Deep Debt Execution):**

| Update | Impact |
|--------|--------|
| **Hardware Transport wired** | `transport.discover/list/route` JSON-RPC + `toadstool transport discover/list/status` CLI |
| **18,028 tests** | Up from 5,369 — expanded coverage across detection, monitoring, transport |
| **Detection stubs evolved** | 11 functions → real /proc/cpuinfo, meminfo, os-release, nvidia-smi parsing |
| **Smart refactoring** | `security.rs` (771→5 modules), `config_utils/mod.rs` (777→5 modules) |
| **Hardcoding eliminated** | 35+ primal names → `well_known::*` constants; framework placeholders → explicit Unavailable |
| **Dual-Fabric Architecture** | Spec for hardware backbone (HDMI/serial) + network plane (Songbird) multi-machine deployments |
| **Airgapped data diode** | Hardware-enforced unidirectional data flow via HDMI — commodity GPU + capture card |
| **Pixel format + buffer bugs fixed** | CaptureTransport AR24 alignment; DisplayTransport double-buffer alternation |

**Remaining toadStool debt:** D-COV (90% target)

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
| **[SOVEREIGN_COMPUTE_EVOLUTION.md](./SOVEREIGN_COMPUTE_EVOLUTION.md)** | WGSL optimizer, LatencyModel, mycelial ToadStool — master roadmap | **Feb 18** | 🔄 Active |
| **[BARRACUDA_PRIMAL_BUDDING.md](./BARRACUDA_PRIMAL_BUDDING.md)** | barraCuda budding — fully untangled, zero cross-deps | **Mar 3** | ✅ Phase 5 Complete |
| **[ARCHITECTURE_DEMARCATION.md](./ARCHITECTURE_DEMARCATION.md)** | 3-layer ownership: barraCuda (math), toadStool (orchestration), songBird (wire) | **Mar 2** | 🔄 Active |

**Tracker**: [`../SOVEREIGN_COMPUTE.md`](../SOVEREIGN_COMPUTE.md) — root-level phase/status dashboard

### Performance & Evolution

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[HYBRID_FP64_CORE_STREAMING.md](./HYBRID_FP64_CORE_STREAMING.md)** | DF64 core streaming — hybrid FP32/FP64 | **Feb 23** | ✅ barraCuda team |
| **[BARRACUDA_PARITY_ROADMAP.md](./BARRACUDA_PARITY_ROADMAP.md)** | Performance evolution, benchmarks | **Feb 16** | ✅ barraCuda team |
| **[FP64_GPU_EVOLUTION.md](./FP64_GPU_EVOLUTION.md)** | Pure-GPU f64 math, fossil functions | **Feb 23** | ✅ barraCuda team |
| **[CROSS_PLATFORM_WORKLOADS.md](./CROSS_PLATFORM_WORKLOADS.md)** | Cross-vendor workload strategy (GPU + NPU) | Feb 13 | ✅ Current |
| **[CROSS_VENDOR_BENCHMARK_SPEC.md](./CROSS_VENDOR_BENCHMARK_SPEC.md)** | Benchmark methodology and validation | Feb 13 | ✅ Current |

### NPU & Multi-Tenant

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[NPU_DRIVER_ARCHITECTURE.md](./NPU_DRIVER_ARCHITECTURE.md)** | Pure Rust VFIO NPU driver design | Feb 8 | ✅ Current |
| **[NPU_MULTI_TENANT_ARCHITECTURE.md](./NPU_MULTI_TENANT_ARCHITECTURE.md)** | Multi-tenant NPU resource partitioning | Feb 8 | ✅ Current |
| **[MULTITENANT_COMPUTE_ARCHITECTURE.md](./MULTITENANT_COMPUTE_ARCHITECTURE.md)** | Compute multi-tenancy across GPU/NPU/CPU | Feb 8 | ✅ Current |
| **[BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md](./BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md)** | Universal tensor ops (CPU, GPU, NPU) | Feb 2 | ✅ Current |

### Hardware Transport & Dual-Fabric (S94b)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[DUAL_FABRIC_ARCHITECTURE.md](./DUAL_FABRIC_ARCHITECTURE.md)** | Multi-machine dual-fabric: hardware backbone (toadStool) + network plane (Songbird) | **Mar 3** | ✅ Specified |
| **[HARDWARE_TRANSPORT_SPEC.md](./HARDWARE_TRANSPORT_SPEC.md)** | `HardwareTransport` trait, frame protocol, DisplayTransport, CaptureTransport, SerialTransport, TransportRouter | **Mar 3** | ✅ Implemented |
| **[DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md)** | DRM/input backend — display hardware control | Jan 19 | ✅ Phase 1 Complete |

### Research & Extensions (Future Work)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md](./RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md)** | Neuromorphic reservoir computing ops | Jan 29 | 📋 Planned |
| **[PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md)** | Capability-based discovery | Nov 2025 | ✅ Implemented |

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

Historical documents preserved in `archive/` (25 files):

| Category | Documents |
|----------|-----------|
| **Evolution Phases** | BARRACUDA_PHASE3/5_EVOLUTION_HOTSPRING (superseded by PARITY_ROADMAP) |
| **Science Audits** | SCIENCE_GAPS_FEB12, SCIENCE_GAPS_AUDIT_FEB12 (completed) |
| **Optimization** | PERFORMANCE_OPTIMIZATION_PLAN/SUMMARY (implemented) |
| **Architecture** | TOADSTOOL_CORE_IMPLEMENTATION_SPEC (completed) |
| **Old Roadmaps** | UNIVERSAL_COMPUTE_*, FRACTAL_COMPOSITION_* (superseded) |

These documents preserve the evolution history and design decisions. Current state is in active specs above.

---

## Design Principles

### Runtime Discovery, Not Hardcoding

```rust
// WRONG
let cache_size = if vendor == "AMD" { 128_MB } else { 6_MB };

// RIGHT
let hierarchy = SubstrateMemoryHierarchy::probe(&device).await;
```

### Math Is Universal, Precision Is Silicon

All math is WGSL primary. One source, any precision via `compile_shader_universal()`:
- 700 WGSL shaders (497 f32 via LazyLock, 182 f64, 21 Df64 — zero f32-only)
- Dual-layer: op_preamble (abstract ops) + naga df64_rewrite (infix→bridge functions)
- 12 universal `{{SCALAR}}` templates
- Same shader → Vulkan (NVIDIA/AMD), Metal (Apple), DX12 (Windows)

### Vendor-Agnostic Results

Same binary, identical results:
- RTX 3090 (NVIDIA) → checksum **5.128010**
- RX 6950 XT (AMD) → checksum **5.128010**
- Zero CUDA, Zero ROCm

### Sibling Validation Projects

BarraCuda is validated by multiple domain-specific projects:

| Project | Domain | Checks | Key Findings |
|---------|--------|:------:|--------------|
| **hotSpring** | Nuclear physics (HFB + lattice QCD) | 664 tests | GPU-resident HFB, consumer QCD validated, 22 papers |
| **wetSpring** | Life science + analytical chemistry | 918 tests | log_f64 bug found+fixed; Shannon/Simpson/Bray-Curtis validated |
| **airSpring** | Precision agriculture (ET₀, soil, IoT) | 468 tests | FAO-56 validated; 918 real station-days; 53-72% water savings |
| **neuralSpring** | Neural network inference | 580 tests | 6 universal ops serve every domain |
| **groundSpring** | Hydrogeology | 154 tests | RAWR bootstrap, regression, hydrology |

**Combined validation**: 4,000+ acceptance checks across physics, chemistry, biology, agriculture, and ML.

All projects evolve compute pipelines from Python to Rust+GPU, validating accuracy
at every step. Bugs discovered by validation projects are fixed immediately in
ToadStool core.

**Cross-spring synergies**:
- hotSpring → airSpring: f64 GPU patterns, dispatch batching, hybrid GPU+Rayon
- airSpring → wetSpring: Spatial interpolation (kriging) for sampling sites
- wetSpring → airSpring: IoT stream processing for real-time sensor data

---

## Quick Links

- **Immediate work:** `../NEXT_STEPS.md`
- **Planning docs:** `docs/planning/`
- **Benchmarks:** `showcase/cross-platform/`
- **Hardware traits:** `crates/toadstool-core/src/` (NpuDispatch, NpuParameterController)
- **GPU backends:** `crates/runtime/universal/src/backends/`
- **Tests:** `cargo test --workspace`
- **barraCuda (math/shaders):** `ecoPrimals/barraCuda/`
