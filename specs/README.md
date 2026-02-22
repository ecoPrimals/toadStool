# ToadStool + BarraCuda Specifications

## Current Status (February 18, 2026)

**Quick Start:**
- **`../README.md`** — Project overview, architecture, key achievements
- **`../STATUS.md`** — Detailed status with quality gates
- **`BARRACUDA_PARITY_ROADMAP.md`** — Performance evolution and validation results
- **`FP64_GPU_EVOLUTION.md`** — Pure-GPU f64 math (includes log_f64 bug fix)

**Key Numbers:**
- **15,700+ tests passing**, 0 failing
- **480+ WGSL shaders** (shader-first architecture)
- **82-86% theoretical bandwidth** on both NVIDIA and AMD
- **hotSpring validated**: 169/169 nuclear EOS acceptance checks
- **wetSpring validated**: 48/48 life science checks (Shannon, Simpson, Bray-Curtis)
- Pure-GPU f64 math library with 27+ transcendental functions

**Latest Updates (Feb 18):**

| Update | Impact |
|--------|--------|
| **Sovereign Compute spec** | WGSL optimizer roadmap — Phases 0-5, LatencyModel trait, mycelial deployment |
| **f64 fossil functions removed** | `math_f64.wgsl` calls native WGSL builtins for abs/sqrt/min/max/floor/ceil/round/fract/sign |
| **SM70 latency tables** | DFMA=8cy corrected — Phase 0 NAK contribution complete |
| **Root tracker doc** | `SOVEREIGN_COMPUTE.md` — phase status dashboard |

**Previous (Feb 17):**

| Update | Impact |
|--------|--------|
| **Unidirectional Pipeline** | Zero round-trip architecture exploration — 4 design docs |
| **Hardware Routing Layer** | ToadStool manages PCIe/HDMI/NVLink as data channels |
| **Software Simulation** | 90/10 bandwidth partitioning to validate patterns |
| **Pure Rust syscalls** | akida-driver mmap/mlock migrated to rustix |
| **biomeOS networking** | No reqwest/hyper — Songbird TLS, Beardog crypto |

**Previous Updates (Feb 16):**

| Update | Impact |
|--------|--------|
| **log_f64 bug fix** | Coefficients halved (~1e-3 → ~1e-15 precision) — wetSpring discovery |
| **GPU-Resident Pipeline** | Complete — zero CPU↔GPU round-trips |
| **Device Registry** | Physical device deduplication with backend preference |
| **ecoBin Compliance** | TOML config, XDG paths, rustix signals |
| **NPU Executor** | `NpuExecutor` implements `ComputeExecutor` |

**Sibling Validation Projects:**
- **hotSpring** — Nuclear physics (HFB), 169/169 acceptance checks
- **wetSpring** — Life science (metagenomics) + analytical chemistry (PFAS)

---

## Active Specifications

### Sovereign Compute (Current Priority)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[SOVEREIGN_COMPUTE_EVOLUTION.md](./SOVEREIGN_COMPUTE_EVOLUTION.md)** | WGSL optimizer, LatencyModel, mycelial ToadStool — master roadmap | **Feb 18** | 🔄 Active |

**Tracker**: [`../SOVEREIGN_COMPUTE.md`](../SOVEREIGN_COMPUTE.md) — root-level phase/status dashboard

### Performance & Evolution

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[BARRACUDA_PARITY_ROADMAP.md](./BARRACUDA_PARITY_ROADMAP.md)** | Performance evolution, benchmarks, validated results | **Feb 16** | ✅ Current |
| **[FP64_GPU_EVOLUTION.md](./FP64_GPU_EVOLUTION.md)** | Pure-GPU f64 math, fossil functions, log_f64 bug fix | **Feb 18** | ✅ Current |
| **[CROSS_PLATFORM_WORKLOADS.md](./CROSS_PLATFORM_WORKLOADS.md)** | Cross-vendor workload strategy (GPU + NPU) | Feb 13 | ✅ Current |
| **[CROSS_VENDOR_BENCHMARK_SPEC.md](./CROSS_VENDOR_BENCHMARK_SPEC.md)** | Benchmark methodology and validation | Feb 13 | ✅ Current |

### NPU & Multi-Tenant

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[NPU_DRIVER_ARCHITECTURE.md](./NPU_DRIVER_ARCHITECTURE.md)** | Pure Rust VFIO NPU driver design | Feb 8 | ✅ Current |
| **[NPU_MULTI_TENANT_ARCHITECTURE.md](./NPU_MULTI_TENANT_ARCHITECTURE.md)** | Multi-tenant NPU resource partitioning | Feb 8 | ✅ Current |
| **[MULTITENANT_COMPUTE_ARCHITECTURE.md](./MULTITENANT_COMPUTE_ARCHITECTURE.md)** | Compute multi-tenancy across GPU/NPU/CPU | Feb 8 | ✅ Current |
| **[BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md](./BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md)** | Universal tensor ops (CPU, GPU, NPU) | Feb 2 | ✅ Current |

### Research & Extensions (Future Work)

| Document | Purpose | Updated | Status |
|----------|---------|---------|--------|
| **[RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md](./RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md)** | Neuromorphic reservoir computing ops | Jan 29 | 📋 Planned |
| **[DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md)** | DRM/input backend (Phase 0) | Jan 19 | 📋 Planned |
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
| 5 | **HDMI + capture (hardware)** | 📝 Research | Physical unidirectional with Magewell GPUDirect |

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
| **Display Backend** | DRM/input backend (Phase 0) | 📋 Planned | ToadStool core |
| **Multi-Gate Routing** | Cross-machine GPU job routing | 📋 Planned | ToadStool core |
| **Hardware Routing Layer** | ToadStool manages physical interconnects | 📋 Planned | ToadStool core |

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

### Shader-First Architecture

All math is WGSL primary. ToadStool dispatches to GPU or CPU based on hardware:
- 480+ WGSL shaders
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
| **hotSpring** | Nuclear physics (HFB + MD) | 195/195 | GPU-resident HFB 15% faster than CPU; 2 bugs found+fixed |
| **wetSpring** | Life science + analytical chemistry | 48/48 | log_f64 bug found+fixed; Shannon/Simpson/Bray-Curtis validated |
| **airSpring** | Precision agriculture (ET₀, soil, IoT) | 70/70 Rust, 142 Python | FAO-56 validated; 918 real station-days; 53-72% water savings |

**Combined validation**: 313+ acceptance checks across physics, chemistry, biology, and agriculture.

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
- **Shaders:** `crates/barracuda/src/shaders/`
- **Device layer:** `crates/barracuda/src/device/`
- **Tests:** `cargo test -p barracuda`
