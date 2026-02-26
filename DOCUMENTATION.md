# ToadStool Documentation Hub

**Last Updated**: February 24, 2026 -- Session 67

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See precision evolution gate | [PRECISION_BOTTLENECK.md](PRECISION_BOTTLENECK.md) |
| See current status | [STATUS.md](STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| See active debt and evolution paths | [DEBT.md](DEBT.md) |
| Universal precision design | [specs/UNIVERSAL_PRECISION_ARCHITECTURE.md](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) |
| Roadmap and next steps | [NEXT_STEPS.md](NEXT_STEPS.md) |
| Sovereign compute roadmap | [SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md) |
| Unidirectional pipeline | [UNIDIRECTIONAL_PIPELINE.md](UNIDIRECTIONAL_PIPELINE.md) |
| See all JSON-RPC methods | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#json-rpc-methods-36-total) |
| Try GPU operations | [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Use scientific computing | [docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |
| Hybrid FP64 core streaming | [specs/HYBRID_FP64_CORE_STREAMING.md](specs/HYBRID_FP64_CORE_STREAMING.md) |

---

## Current State (Session 67 — February 24, 2026)

- **Math is universal, precision is silicon** — `compile_shader_universal()` compiles one shader source to f32/f64/df64. 12 universal `{{SCALAR}}` templates. `downcast_f64_to_f32()`.
- **Sovereign Compiler** — naga-IR optimizer: FMA fusion, dead expression elimination, SPIR-V passthrough. End-to-end Rust GPU compilation pipeline.
- **700 WGSL shaders** — zero orphans, 21 DF64 files, **zero f32-only**. All shaders f64 canonical; f32 generated via LazyLock downcast. 296 f32 WGSL files deleted.
- **Deep debt systematically resolved** — 15 files smart-refactored (20-44% reductions). 3 `#[allow(dead_code)]` remain (feature-gated/Phase 5+).
- **FMA-optimized DF64** — `two_prod` 17→2 ops. DF64 transcendentals at FP32 core speed.
- **f64 polyfill library** — 28 functions, no vendor math library dependency (no libdevice/ocml)
- **2,541 barracuda tests** + 21,599 workspace tests | all quality gates green
- **Zero clippy warnings** | zero fmt diffs | zero TODO/FIXME/HACK | all files under 1000 lines
- **Linalg GPU-dispatched** — solve, cholesky (with SPD validation), QR, SVD, LU
- **Lattice QCD** — 14 GPU shaders + CG solver + HMC trajectory orchestration
- **MD fully GPU** — VV, RDF, MSD, PPPM (GPU FFT), all force fields + DF64 variants
- **36 JSON-RPC methods** across 8 domains

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, quality gates, evolution roadmap.

**[PRECISION_BOTTLENECK.md](PRECISION_BOTTLENECK.md)** -- **Evolution gate**: 50 duplicate f32/f64 pairs to consolidate, ~30 f32-only universals to evolve, ~64 transcendental-dependent shaders. Gate rule: no new spring absorptions until precision debt resolved.

**[STATUS.md](STATUS.md)** -- Detailed technical status: quality gates, session-by-session evolution.

**[DEBT.md](DEBT.md)** -- Active debt register, workarounds, and evolution paths.

**[NEXT_STEPS.md](NEXT_STEPS.md)** -- Roadmap: active work, upcoming infrastructure, completed milestones.

**[CHANGELOG.md](CHANGELOG.md)** -- Full session-by-session evolution history.

---

## Architecture and Specs

**[SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md)** -- Sovereign Compute Evolution: Phases 0–3 complete, Phase 4 roadmap, latency models, Mesa NAK contribution plan.

**[UNIDIRECTIONAL_PIPELINE.md](UNIDIRECTIONAL_PIPELINE.md)** -- GPU-resident unidirectional pipeline architecture.

**[specs/](specs/)** -- Technical specifications (FP64 evolution, hybrid core streaming, NPU, multi-tenant, cross-platform).

**[docs/architecture/](docs/architecture/)** -- Design documents, ADRs, migration patterns.

---

## Guides

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing.

**[docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)** -- NPU driver deployment.

**[docs/guides/BARRACUDA_V2_QUICKSTART.md](docs/guides/BARRACUDA_V2_QUICKSTART.md)** -- BarraCuda quick start.

---

## Scientific Middleware

**[docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md)** -- Comprehensive implementation guide: all modules, functions, tests, algorithms, design principles.

**[docs/PHASE1_COMPLETION_REPORT.md](docs/PHASE1_COMPLETION_REPORT.md)** -- Validation report: test results, metrics, deep debt compliance, architecture.

---

## Audits

**[docs/audits/](docs/audits/)** -- Dependency audits, unsafe code audits, deep debt audits.

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md)

**Computational Scientists**: [docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md) then [QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api)

**System Architects**: [STATUS.md](STATUS.md) then [specs/](specs/)

**DevOps Engineers**: [QUICK_REFERENCE.md](QUICK_REFERENCE.md) then [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)

---

## Directory Structure

```
README.md                  -- Project overview, honest status
PRECISION_BOTTLENECK.md    -- Evolution gate tracker (solve before absorbing)
STATUS.md                  -- Detailed technical status
DEBT.md                    -- Active debt register, evolution paths
NEXT_STEPS.md              -- Roadmap and upcoming work
QUICK_REFERENCE.md         -- Commands, API, constants
CHANGELOG.md               -- Full session history
SOVEREIGN_COMPUTE.md       -- Sovereign compute roadmap
UNIDIRECTIONAL_PIPELINE.md -- GPU-resident pipeline design
DOCUMENTATION.md           -- This file (navigation hub)
docs/
  guides/                  -- Deployment and usage guides
  architecture/            -- Design documents and ADRs
  reference/               -- API reference, constants
  audits/                  -- Security and quality audits
specs/                     -- Technical specifications
```
