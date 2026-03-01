# ToadStool Documentation Hub

**Last Updated**: March 1, 2026 -- Session 71

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See current status | [STATUS.md](STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| See active debt and evolution paths | [DEBT.md](DEBT.md) |
| Universal precision design | [specs/UNIVERSAL_PRECISION_ARCHITECTURE.md](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) |
| Roadmap and next steps | [NEXT_STEPS.md](NEXT_STEPS.md) |
| Sovereign compute roadmap | [SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md) |
| Unidirectional pipeline | [UNIDIRECTIONAL_PIPELINE.md](UNIDIRECTIONAL_PIPELINE.md) |
| See all JSON-RPC methods | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#json-rpc-methods-36-total) |
| Try GPU operations | [docs/guides/BARRACUDA_V2_QUICKSTART.md](docs/guides/BARRACUDA_V2_QUICKSTART.md) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Use scientific computing | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |
| Hybrid FP64 core streaming | [specs/HYBRID_FP64_CORE_STREAMING.md](specs/HYBRID_FP64_CORE_STREAMING.md) |

---

## Current State (Session 71 — March 1, 2026)

**Still evolving.** Deep debt swept. Test suite fully concurrent and fast. All production stubs evolved. Transitioning from fp64 shaders to true math — springs will have many interactions to evolve as barracuda owns all precision.

- **Standalone-resilient** — Pull to any machine, `cargo test` works. GPU-optional with CPU fallback. Device-lost recovery.
- **Fully concurrent tests** — All tests run with `--test-threads=8`. Zero `#[serial]`. Zero fixed sleeps in non-chaos tests. 6m30s full workspace.
- **Deep debt: clean** — Zero `chrono`, zero `log` (core), zero production stubs/mocks, 45 justified `unsafe` blocks, zero hardcoded localhost/ports, zero `Box<dyn Error>`, zero blind `.unwrap()`, zero `todo!()`, zero `dbg!()`. All env tests thread-safe via `temp_env`.
- **Dual-layer universal precision** — Layer 1: `op_preamble`. Layer 2: naga-guided `df64_rewrite`. `compile_shader_universal()` + `compile_op_shader()` route to f16/f32/f64/df64.
- **Sovereign Compiler** — naga-IR optimizer: FMA fusion, DCE, df64 infix rewrite, SPIR-V passthrough.
- **671 WGSL shaders** — zero orphans, 25 DF64 files, **zero f32-only**. All f64 canonical.
- **2,773+ barracuda tests** + 5,400+ workspace lib tests (8,200+ total) | all quality gates green (0 warnings)
- **Linalg GPU-dispatched** — solve, cholesky, QR, SVD, LU
- **Lattice QCD** — 14 GPU shaders + CG solver + HMC trajectory
- **MD fully GPU** — VV, RDF, MSD, PPPM (GPU FFT), all force fields + DF64 variants
- **36 JSON-RPC methods** across 8 domains

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, quality gates, evolution roadmap.

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

**[QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api)** -- API reference with usage examples for all middleware modules (linalg, numerical, special, stats, optimize, surrogate, sample, PDE, mixing, grids).

Archived: `BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md` and `PHASE1_COMPLETION_REPORT.md` moved to `ecoPrimals/fossil/`.

---

## Audits

**[docs/audits/](docs/audits/)** -- Dependency audits, unsafe code audits, deep debt audits.

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then [docs/guides/BARRACUDA_V2_QUICKSTART.md](docs/guides/BARRACUDA_V2_QUICKSTART.md)

**Computational Scientists**: [QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api) then `cargo doc -p barracuda --open`

**System Architects**: [STATUS.md](STATUS.md) then [specs/](specs/)

**DevOps Engineers**: [QUICK_REFERENCE.md](QUICK_REFERENCE.md) then [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)

---

## Directory Structure

```
README.md                  -- Project overview, honest status
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
