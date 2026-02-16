# ToadStool + BarraCUDA Specifications

## Current Status (February 15, 2026)

**Quick Start:**
- **`../README.md`** — Project overview, architecture, key achievements
- **`../NEXT_STEPS.md`** — Immediate work from hotSpring (GPU-resident pipeline)
- **`BARRACUDA_PARITY_ROADMAP.md`** — Performance evolution and validation results

**Key Numbers:**
- **15,700+ tests passing**, 0 failing
- **480+ WGSL shaders** (shader-first architecture)
- **82-86% theoretical bandwidth** on both NVIDIA and AMD
- **hotSpring validated**: 169/169 nuclear EOS acceptance checks
- Pure-GPU f64 math library with 27+ transcendental functions

**Latest: GPU-Resident Pipeline (Feb 16)**
- hotSpring Exp 005: 95% GPU utilization, but CPU still 70× faster (small matrices)
- Root cause: Amdahl's Law — eigensolve is 1% of iteration, CPU physics is 99%
- Solution: GPU-resident iteration loop with zero CPU↔GPU round-trips
- See `../NEXT_STEPS.md` and `docs/planning/GPU_RESIDENT_PIPELINE_FEB16_2026.md`

---

## Active Specifications

### Performance & Evolution (Primary Focus)

| Document | Purpose | Updated |
|----------|---------|---------|
| **[BARRACUDA_PARITY_ROADMAP.md](./BARRACUDA_PARITY_ROADMAP.md)** | Performance evolution, benchmarks, validated results | Feb 13 |
| **[FP64_GPU_EVOLUTION.md](./FP64_GPU_EVOLUTION.md)** | Pure-GPU f64 math, Naga/WGSL gotchas, precision validation | Feb 14 |
| **[CROSS_PLATFORM_WORKLOADS.md](./CROSS_PLATFORM_WORKLOADS.md)** | Cross-vendor workload strategy (GPU + NPU) | Feb 13 |
| **[CROSS_VENDOR_BENCHMARK_SPEC.md](./CROSS_VENDOR_BENCHMARK_SPEC.md)** | Benchmark methodology and validation | Feb 12 |

### NPU & Multi-Tenant

| Document | Purpose | Updated |
|----------|---------|---------|
| **[NPU_DRIVER_ARCHITECTURE.md](./NPU_DRIVER_ARCHITECTURE.md)** | Pure Rust VFIO NPU driver design | Feb 11 |
| **[NPU_MULTI_TENANT_ARCHITECTURE.md](./NPU_MULTI_TENANT_ARCHITECTURE.md)** | Multi-tenant NPU resource partitioning | Feb 11 |
| **[MULTITENANT_COMPUTE_ARCHITECTURE.md](./MULTITENANT_COMPUTE_ARCHITECTURE.md)** | Compute multi-tenancy across GPU/NPU/CPU | Feb 11 |
| **[BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md](./BARRACUDA_NPU_UNIVERSAL_COMPUTE_V2.md)** | Universal tensor ops (CPU, GPU, NPU) | Feb 2 |

### Research & Extensions

| Document | Purpose | Updated |
|----------|---------|---------|
| **[RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md](./RESERVOIR_COMPUTING_BARRACUDA_EXTENSIONS.md)** | Neuromorphic reservoir computing ops | Jan 29 |
| **[DISPLAY_BACKEND_SPEC.md](./DISPLAY_BACKEND_SPEC.md)** | DRM/input backend (Phase 0) | Jan 18 |
| **[PRIMAL_CAPABILITY_SYSTEM.md](./PRIMAL_CAPABILITY_SYSTEM.md)** | Capability-based discovery | Jan 7 |

---

## GPU-Resident Pipeline Targets (Feb 16)

| # | Item | Status | Impact |
|:-:|------|:------:|--------|
| 1 | Max Abs Diff Reduction | Planned | Convergence check |
| 2 | Persistent Buffer Management | Planned | Pin for solver lifetime |
| 3 | Batched Bisection | Planned | GPU BCS pairing |
| 4 | Grid Quadrature GEMM | Planned | GPU Hamiltonian |
| 5 | Multi-Kernel Pipeline | Planned | Buffer chaining |

**Target**: GPU-resident SCF → ~40s for 791 nuclei (vs 35s CPU)

---

## Completed Evolution (Reference)

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

Historical documents preserved in `archive/`:
- Evolution phase documents (superseded by PARITY_ROADMAP)
- Science gap audits (completed)
- Old optimization plans (implemented)
- Older architecture specs (core implementation complete)

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

---

## Quick Links

- **Immediate work:** `../NEXT_STEPS.md`
- **Planning docs:** `docs/planning/`
- **Benchmarks:** `showcase/cross-platform/`
- **Shaders:** `crates/barracuda/src/shaders/`
- **Device layer:** `crates/barracuda/src/device/`
- **Tests:** `cargo test -p barracuda`
