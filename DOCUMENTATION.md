# ToadStool Documentation Hub

**Last Updated**: April 4, 2026 — S180

---

## Fossil Record

These root documents were **fully resolved** and **fossilized** in wateringHole (see `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`): **UNSAFE_AUDIT_REPORT**, **SOVEREIGN_COMPUTE_GAPS**, **PURE_RUST_TRACKING**, **STATUS**, **EVOLUTION_TRACKER**, **QUICK_REFERENCE**, **SOVEREIGN_COMPUTE**, **SPRING_ABSORPTION_TRACKER**, **BREAKING_CHANGES** — all renamed with `_S166` suffix. Use those paths when citing historical audit, sovereign-gap, or pure-Rust tracking content.

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See active debt and evolution paths | [DEBT.md](DEBT.md) |
| Universal precision design | [specs/UNIVERSAL_PRECISION_ARCHITECTURE.md](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) |
| Roadmap and next steps | [NEXT_STEPS.md](NEXT_STEPS.md) |
| Full session-by-session changelog | [CHANGELOG.md](CHANGELOG.md) |
| Hardware Transport Layer | [specs/HARDWARE_TRANSPORT_SPEC.md](specs/HARDWARE_TRANSPORT_SPEC.md) |
| Dual-Fabric Architecture | [specs/DUAL_FABRIC_ARCHITECTURE.md](specs/DUAL_FABRIC_ARCHITECTURE.md) |
| GPU operations | See barraCuda (`ecoPrimals/barraCuda/`) |
| FHE encryption | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |

---

## Current State (S180 — April 4, 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **21,853 tests**, 0 failures, 0 clippy warnings. Full workspace concurrent test suite.
- **~67 JSON-RPC methods**. IPC compliant (`health.liveness/readiness/check`, `capabilities.list`, socket at `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`).
- **glowPlug/ember subsystem** — toadStool-native hardware lifecycle (absorbed from coralReef). `toadstool-glowplug`, `toadstool-ember`, `toadstool-hw-safe` crates.
- **~59 unsafe blocks** — 48 in containment zones (`hw-safe`, `v4l2::ioctl`), 11 in consumer/driver code. 23 crates forbid, 20 deny `unsafe_code`.
- **ecoBin v3.0** — Zero C FFI deps. Crypto delegated to BearDog. HTTP delegated to Songbird.
- **Capability-based discovery** — Primals discover each other by capability, not name. Self-knowledge principle. S176-180: deprecated primal-named APIs removed, env_config evolved to capability names, production log/error strings evolved to capability-first.
- **Fully concurrent tests** — All tests run with `--test-threads=8`. Zero `#[serial]`. Zero fixed sleeps in non-chaos tests.
- **AGPL-3.0-only** — All Cargo.toml + all .rs files aligned. `deny.toml` enforced.

---

## Core Documentation

**[README.md](README.md)** -- Project overview, architecture, quality gates, evolution roadmap.

**[DEBT.md](DEBT.md)** -- Active debt register, workarounds, and evolution paths.

**[NEXT_STEPS.md](NEXT_STEPS.md)** -- Roadmap: active work, upcoming infrastructure, completed milestones.

**[CHANGELOG.md](CHANGELOG.md)** -- Full session-by-session evolution history.

---

## Architecture and Specs

**[specs/](specs/)** -- Technical specifications (FP64 evolution, hybrid core streaming, NPU, multi-tenant, cross-platform).

**[docs/architecture/](docs/architecture/)** -- Design documents, ADRs, migration patterns.

---

## Guides

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing.

**[docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)** -- NPU driver deployment.

BarraCuda guides have been fossilized to `ecoPrimals/infra/wateringHole/fossilRecord/`. See the barraCuda primal for current docs.

---

## Scientific Middleware

Scientific computing middleware (linalg, numerical, special, stats, optimize, surrogate, sample, PDE) has moved to **barraCuda** (`ecoPrimals/barraCuda/`). Legacy API examples are preserved in the fossil record (`ecoPrimals/infra/wateringHole/fossilRecord/toadstool/TOADSTOOL_QUICK_REFERENCE_S166.md`).

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then see barraCuda (`ecoPrimals/barraCuda/`)

**Computational Scientists**: See barraCuda (`ecoPrimals/barraCuda/`) for scientific middleware

**System Architects**: [README.md](README.md) then [specs/](specs/)

**DevOps Engineers**: [README.md](README.md) then [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)

---

## Directory Structure

```
README.md                  -- Project overview, honest status
DEBT.md                    -- Active debt register, evolution paths
NEXT_STEPS.md              -- Roadmap and upcoming work
CHANGELOG.md               -- Full session history
CONTEXT.md                 -- Public surface summary
DOCUMENTATION.md           -- This file (navigation hub)
docs/
  guides/                  -- Deployment and usage guides
  architecture/            -- Design documents and ADRs
  reference/               -- API reference, constants
specs/                     -- Technical specifications
```

**Fossil record** — 9 session trackers archived with `_S166` suffix under `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`.
