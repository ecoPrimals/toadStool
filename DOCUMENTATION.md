# ToadStool Documentation Hub

**Last Updated**: February 11, 2026

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See current status | [STATUS.md](STATUS.md) |
| Quick one-page summary | [QUICK_STATUS.md](QUICK_STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| See all JSON-RPC methods | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#json-rpc-methods-26-total) |
| Try GPU operations | [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Use scientific computing | [docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md](docs/BARRACUDA_MIDDLEWARE_IMPLEMENTATION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |

---

## Current State (February 11, 2026)

- **0 clippy warnings** (down from 453)
- **15,490+ tests passing**, 0 failing, 156 ignored
- **414 WGSL shaders** (up from 401 -- Bessel, eigh, linsolve, spherical harmonics, PRNG, sparse matvec, LOO-CV)
- **Scientific middleware** (6 modules: linalg, numerical, special, optimize, surrogate, sample -- 129 tests, 0 unsafe)
- **17 WorkloadHint variants** with auto-routing (GPU, NPU, CPU) and user preference override
- **26 JSON-RPC methods** across 6 domains (toadstool, compute, gpu, ollama, gate, resources)
- GPU job queue with cross-gate routing
- NPU detection: `/dev/akida*` and IOMMU/VFIO sysfs scan for BrainChip 0x1e7c
- Ollama model lifecycle management
- Shared error tracking across all server transports
- 100% `unsafe` block documentation (35 blocks, all with `// SAFETY:`)
- Zero production `todo!()`, zero production mocks
- Cross-vendor distributed GPU compute validated (NVIDIA + AMD, bit-identical)
- 39.85 tok/s distributed LLM inference with encrypted tensor transport

### Test Coverage

| Crate | Line Coverage |
|-------|-------------|
| `toadstool-server` | ~85% |
| `toadstool-common` | ~84% |
| `toadstool-config` | ~85% |

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, quality gates, evolution roadmap.

**[STATUS.md](STATUS.md)** -- Detailed technical status: quality gates, new features, code quality evolution, shader coverage, evolution gaps, deep debt.

---

## Architecture and Specs

**[specs/](specs/)** -- Technical specifications for compute, crypto, display, fractal composition, and more.

**[docs/architecture/](docs/architecture/)** -- Design documents, ADRs, migration patterns.

**[docs/architecture/adrs/](docs/architecture/adrs/)** -- Architecture Decision Records (WGPU, feature gates, NTT, capability discovery).

---

## Guides

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing.

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

**Last Updated**: February 11, 2026
