# ToadStool Documentation Hub

**Last Updated**: February 21, 2026

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See current status | [STATUS.md](STATUS.md) |
| Quick one-page summary | [QUICK_STATUS.md](QUICK_STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| Deep debt progress | [DEEP_DEBT_STATUS.md](DEEP_DEBT_STATUS.md) |
| Unidirectional pipeline | [UNIDIRECTIONAL_PIPELINE.md](UNIDIRECTIONAL_PIPELINE.md) |
| See all JSON-RPC methods | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#json-rpc-methods-26-total) |
| Try GPU operations | [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Use scientific computing | [docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |
| Phase 5 evolution (complete) | [specs/archive/BARRACUDA_PHASE5_EVOLUTION_HOTSPRING.md](specs/archive/BARRACUDA_PHASE5_EVOLUTION_HOTSPRING.md) |
| Phase 3 evolution (complete) | [specs/archive/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md](specs/archive/BARRACUDA_PHASE3_EVOLUTION_HOTSPRING.md) |

---

## Current State (Session 31c — February 21, 2026)

- **16,100+ tests passing**, 0 failing | ~65% line coverage | all quality gates green
- **480+ WGSL shaders** — shader-first architecture, any GPU via WGPU
- **313+ three springs validation** — hotSpring (195), wetSpring (48), airSpring (70)
- **Deep debt resolved** — 10 deps removed, all hardcoded paths evolved, production stubs eliminated
- **GpuExecutor** — 31 MathOps fully wired; **CpuExecutor** — full dispatch
- **Smart refactoring** — LU -61%, SVD -60%, QR -48% via WGPU helper extraction
- **Sovereign Compute Phases 0–3** — `WgslOptimizer` live in `compile_shader_f64()` hot path
- **Scientific middleware** — 14 modules, 400+ tests, 0 unsafe
- **36 JSON-RPC methods** across 8 domains
- **Integration tests** — 13 suites, 167 tests
- Zero production placeholders, zero production mocks

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, quality gates, evolution roadmap.

**[STATUS.md](STATUS.md)** -- Detailed technical status: quality gates, new features, code quality evolution, shader coverage, evolution gaps, deep debt.

**[DEBT.md](DEBT.md)** -- Active debt register, workarounds, and evolution paths.

**[SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md)** -- Sovereign Compute Evolution tracker: Phases 0–3 complete, Phase 4 roadmap, latency models, Mesa NAK contribution plan.

---

## Architecture and Specs

**[specs/](specs/)** -- Technical specifications for compute, crypto, display, fractal composition, and more.

**[docs/architecture/](docs/architecture/)** -- Design documents, ADRs, migration patterns.

**[docs/architecture/adrs/](docs/architecture/adrs/)** -- Architecture Decision Records (WGPU, feature gates, NTT, capability discovery).

---

## Guides

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing.

**[crates/integration-tests/](crates/integration-tests/)** -- Workspace integration test crate. 13 active suites, 167 tests (chaos, error paths, security, fault, runtime execution, and more). Pending suites tracked in `tests/pending/README.md`.

**[docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)** -- NPU driver deployment.

**[docs/guides/BARRACUDA_V2_QUICKSTART.md](docs/guides/BARRACUDA_V2_QUICKSTART.md)** -- BarraCUDA quick start.

---

## Scientific Middleware

**[docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md)** -- Comprehensive implementation guide: all 5 modules, functions, tests, algorithms, design principles.

**[docs/PHASE1_COMPLETION_REPORT.md](docs/PHASE1_COMPLETION_REPORT.md)** -- Validation report: test results, metrics, deep debt compliance, architecture.

**[docs/MIDDLEWARE_COMPLETION_SUMMARY.md](docs/MIDDLEWARE_COMPLETION_SUMMARY.md)** -- Technical summary: deliverables, design decisions, capabilities.

**[DEEP_DEBT_STATUS.md](DEEP_DEBT_STATUS.md)** -- Deep debt compliance verification (modern Rust, zero unsafe, pure dependencies).

---

## Audits

**[docs/audits/](docs/audits/)** -- Dependency audits, unsafe code audits, deep debt audits.

**[specs/BARRACUDA_SCIENCE_GAPS_AUDIT_FEB12_2026.md](specs/BARRACUDA_SCIENCE_GAPS_AUDIT_FEB12_2026.md)** -- hotSpring audit response (all items resolved).

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md)

**Computational Scientists**: [docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md) then [QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api)

**System Architects**: [STATUS.md](STATUS.md) then [specs/](specs/)

**DevOps Engineers**: [QUICK_REFERENCE.md](QUICK_REFERENCE.md) then [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)

**Security Engineers**: [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) then [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md)

---

## Directory Structure

```
README.md                  -- Project overview, honest status
STATUS.md                  -- Detailed technical status
DOCUMENTATION.md           -- This file (navigation hub)
QUICK_STATUS.md            -- One-page summary
QUICK_REFERENCE.md         -- Commands and API reference
docs/
  guides/                  -- Deployment and usage guides
  architecture/            -- Design documents and ADRs
  planning/                -- Roadmaps and evolution plans
  reference/               -- API reference, constants
  audits/                  -- Security and quality audits
specs/                     -- Technical specifications
```

---

**Last Updated**: February 21, 2026 — Session 31c
