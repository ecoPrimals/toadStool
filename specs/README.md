# ToadStool + BarraCUDA Specifications

## Current Status (February 14, 2026)

**Quick Start:**
- **`../README.md`** — Project overview, architecture, key achievements
- **`BARRACUDA_PARITY_ROADMAP.md`** — Performance evolution and validation results

**Key Numbers:**
- **15,700+ tests passing**, 0 failing
- **396 WGSL shaders** (shader-first architecture)
- **82-86% theoretical bandwidth** on both NVIDIA and AMD
- Pure-GPU f64 math library with 27+ transcendental functions
- Runtime cache discovery for intelligent workload tiling

**New: hotSpring MD Integration**
- 9/9 Yukawa OCP cases validated (0.000% energy drift)
- 3.7× GPU speedup at N=2000 (RTX 4070)
- See `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md` for evolution targets

---

## Active Specifications

### Performance & Evolution

| Document | Purpose |
|----------|---------|
| **[BARRACUDA_PARITY_ROADMAP.md](./BARRACUDA_PARITY_ROADMAP.md)** | Performance evolution, benchmarks, validated results |
| **[FP64_GPU_EVOLUTION.md](./FP64_GPU_EVOLUTION.md)** | Pure-GPU f64 math, Naga/WGSL gotchas, precision validation |
| **[CROSS_PLATFORM_WORKLOADS.md](./CROSS_PLATFORM_WORKLOADS.md)** | Cross-vendor benchmark specifications |
| **[CROSS_VENDOR_BENCHMARK_SPEC.md](./CROSS_VENDOR_BENCHMARK_SPEC.md)** | Benchmark methodology and validation |

### Architecture & Infrastructure

| Document | Purpose |
|----------|---------|
| **[BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md](./BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md)** | Universal tensor ops (CPU, GPU, NPU) |
| **[BARRACUDA_SCIENTIFIC_COMPUTING_OPS.md](./BARRACUDA_SCIENTIFIC_COMPUTING_OPS.md)** | FFT, complex arithmetic, physics primitives |
| **[NPU_DRIVER_ARCHITECTURE.md](./NPU_DRIVER_ARCHITECTURE.md)** | NPU driver design |
| **[NPU_MULTI_TENANT_ARCHITECTURE.md](./NPU_MULTI_TENANT_ARCHITECTURE.md)** | Multi-tenant NPU |
| **[MULTITENANT_COMPUTE_ARCHITECTURE.md](./MULTITENANT_COMPUTE_ARCHITECTURE.md)** | Compute multi-tenancy |

### Platform & Integration

| Document | Purpose |
|----------|---------|
| [PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md) | Capability-based discovery |
| [UNIVERSAL_COMPUTE_PLATFORM.md](./UNIVERSAL_COMPUTE_PLATFORM.md) | Platform architecture |
| [UNIVERSAL_UNIFIED_MEMORY.md](./UNIVERSAL_UNIFIED_MEMORY.md) | Unified memory model |
| [SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md](./SOVEREIGN_SCIENCE_GRADE_ACHIEVEMENT.md) | Quality standards |
| [RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md](./RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md) | Neuromorphic extensions |

### Molecular Dynamics Evolution (hotSpring Integration)

| Priority | Target | Status |
|----------|--------|--------|
| HIGH | f64 Yukawa force with PBC + PE | **DONE** — `yukawa_f64.wgsl` |
| HIGH | Cell-list neighbor search | **DONE** — `yukawa_celllist_f64.wgsl` |
| MEDIUM | Split Velocity-Verlet (kick-drift-kick) | **DONE** — `velocity_verlet_split.wgsl` |
| MEDIUM | Berendsen thermostat | **DONE** — `berendsen.wgsl` + Rust op |
| MEDIUM | GPU observables (KE, RDF) | **DONE** — `kinetic_energy.wgsl`, `rdf_histogram.wgsl` |
| MEDIUM | CPU observables (VACF, SSF) | **DONE** — `compute_vacf()`, `compute_ssf()` |
| MEDIUM | Nosé-Hoover thermostat | Pending — NVT production |
| HIGH | PPPM/Ewald for long-range Coulomb | Pending — uses existing FFT |
| FUTURE | MSU HPC comparison benchmark | Planning — Murillo collaboration |

**Reference:** `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md`

### Other

| Document | Purpose |
|----------|---------|
| [TOADSTOOL_CORE_IMPLEMENTATION_SPEC.md](./TOADSTOOL_CORE_IMPLEMENTATION_SPEC.md) | Core implementation |
| [TOADSTOOL_LOCAL_SHOWCASE_SPEC.md](./TOADSTOOL_LOCAL_SHOWCASE_SPEC.md) | Local showcase demos |
| [UNIVERSAL_COMPUTE_ORCHESTRATOR.md](./UNIVERSAL_COMPUTE_ORCHESTRATOR.md) | Orchestration |
| [DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md) | Display backend |
| [CRYPTO_LOCK_ARCHITECTURE.md](./CRYPTO_LOCK_ARCHITECTURE.md) | Security |
| [PHASE2_CONFIGURATION_MANAGEMENT_COMPLETE.md](./PHASE2_CONFIGURATION_MANAGEMENT_COMPLETE.md) | Phase 2 config |

---

## Archive

Historical documents preserved for context are in `archive/`:
- Evolution phase documents (superseded by PARITY_ROADMAP)
- Science gap audits (completed)
- Old optimization plans (implemented)
- Fractal composition specs (different focus)

---

## Design Principles

### Runtime Discovery, Not Hardcoding

**WRONG:**
```rust
let cache_size = if vendor == "AMD" { 128_MB } else { 6_MB };
```

**RIGHT:**
```rust
let hierarchy = SubstrateMemoryHierarchy::probe(&device).await;
// The silicon tells us what it can do
```

### Shader-First Architecture

All math is WGSL primary. ToadStool dispatches to GPU or CPU based on hardware:
- 396 WGSL shaders
- Same shader → Vulkan (NVIDIA/AMD), Metal (Apple), DX12 (Windows)
- CPU fallback via wgpu software rasterizer

### Vendor-Agnostic Results

Same binary, identical results:
- RTX 3090 (NVIDIA) → checksum **5.128010**
- RX 6950 XT (AMD) → checksum **5.128010**
- Zero CUDA, Zero ROCm

---

## Quick Links

- **Benchmarks:** `showcase/cross-platform/`
- **Shaders:** `crates/barracuda/src/shaders/`
- **Device layer:** `crates/barracuda/src/device/`
- **Tests:** `cargo test -p barracuda`
