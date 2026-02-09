# ToadStool Documentation Hub

**Last Updated**: February 9, 2026

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See current status | [STATUS.md](STATUS.md) |
| Quick one-page summary | [QUICK_STATUS.md](QUICK_STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| Try GPU operations | [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |

---

## Latest: Distributed GPU Compute (February 9, 2026)

Cross-vendor, cross-machine distributed compute validated in production:

- **Bit-identical results** on NVIDIA RTX 4070, NVIDIA RTX 3090, AMD RX 6950 XT
- **39.85 tok/s** pipeline-parallel LLM inference across LAN
- **BearDog-encrypted** tensor transport (ChaCha20-Poly1305)
- Single binary, zero vendor SDK

See [docs/sessions/feb-9-2026/DISTRIBUTED_GPU_COMPUTE_HANDOFF.md](docs/sessions/feb-9-2026/DISTRIBUTED_GPU_COMPUTE_HANDOFF.md) for the full handoff document.

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, honest status, evolution roadmap.

**[STATUS.md](STATUS.md)** -- Detailed technical status: build, shader coverage, hardware interface, scientific computing, evolution gaps, deep debt.

**[CHANGELOG.md](CHANGELOG.md)** -- Version history with all major sessions documented.

---

## Quick Start Guides

**[docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md)** -- GPU operations: matrix multiplication, transformer attention, object detection.

**[docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md)** -- Fully Homomorphic Encryption: NTT/INTT, GPU acceleration (21.1x speedup).

**[docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)** -- NPU driver deployment: kernel vs userspace backends, systemd install, multi-tenant setup.

---

## Architecture and Specs

**[specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md)** -- Dual-backend NPU driver: kernel (DMA, interrupts) and userspace (mmap, polling). Runtime capability discovery.

**[specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md)** -- Multi-tenant design: owner vs tenant access, sandboxing via userspace drivers, resource isolation.

**[docs/architecture/](docs/architecture/)** -- GPU strategy, daemon evolution, CPU ops, hardware discovery, migration patterns.

---

## Testing

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing. FHE suite at 79% coverage.

---

## Session Archives

Session documentation is organized by date under `docs/sessions/`:

| Date | Key Topic |
|------|-----------|
| [feb-9-2026](docs/sessions/feb-9-2026/) | Distributed GPU compute, cross-vendor validation |
| [feb-8-2026](docs/sessions/feb-8-2026/) | ToadStool pure Rust, NPU drivers, RBF scientific computing, hardware wiring |

Older sessions archived in `docs/archive/`.

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then [docs/guides/QUICK_START_GPU.md](docs/guides/QUICK_START_GPU.md)

**System Architects**: [STATUS.md](STATUS.md) then [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md)

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
CHANGELOG.md               -- Version history
docs/guides/
  QUICK_START_GPU.md         -- GPU quick start
  QUICK_START_ENCRYPTION.md  -- FHE quick start
  TESTING.md                 -- Testing guide

docs/
  sessions/                -- Session archives by date
  architecture/            -- Design documents
  planning/                -- Roadmaps
  guides/                  -- Deployment guides
  reference/               -- API reference
  archive/                 -- Historical documentation
  audits/                  -- Security audits

specs/                     -- Technical specifications
```

---

**Last Updated**: February 9, 2026
