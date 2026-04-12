# ToadStool Documentation Hub

**Last Updated**: April 12, 2026 — S203e

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

## Current State (S203e — April 12, 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent, capability-based.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **21,600+ tests**, 0 failures, 0 clippy warnings, 0 fmt diffs. Full workspace concurrent test suite.
- **~69 JSON-RPC methods**. Wire Standard L3 (partial): `cost_estimates`, `operation_dependencies`. IPC compliant (`health.liveness` → `{"status":"alive"}`, `health.readiness` → ready+version, `health.check` full envelope, `capabilities.list`, `identity.get`, socket at `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`).
- **Pipeline dispatch** — `compute.dispatch.pipeline.submit` + `.status` for ordered multi-stage workloads (DAG, topological sort, result forwarding). Resolves neuralSpring PG-05.
- **Capability-based everywhere (S202)**: 0 production hardcoded primal names, 0 production mocks, 0 production unwraps, 0 TODOs/FIXMEs. All primal references use `PRIMAL_NAME` constant or capability identifiers. API keys evolved (e.g., `shader_compiler_available`).
- **TS-01 / shader compiler discovery** — `visualization_client.rs` uses unified `capability.discover` (no `CORALREEF_*` env, no coralreef-core.json, no coralreef dir scan).
- **BTSP Phase 2 + Auto-Detect** — Handshake enforced on every UDS accept path; auto-detects plain-text clients (primalSpring) and degrades gracefully.
- **Network constants centralized** — 8 hardcoded values (RFC1918 ranges, gateway, scan suffixes, TEST-NET-3) → `core/config/defaults/network.rs`
- **34 unsafe blocks (all in hw-safe/GPU/VFIO/display containment crates)**; all SAFETY-documented. 41 crates forbid, 6 deny `unsafe_code`.
- **ecoBin v3.0** — Zero C FFI deps. `serialport` feature-gated in specialty crate (S202). Crypto delegated to security service. HTTP delegated to coordination service.
- **Headless GPU** — `TOADSTOOL_HEADLESS=1` env var for pure headless operation. wgpu crash isolation via `catch_unwind` + thread timeout.
- **Fully concurrent tests** — All tests run with unlimited parallelism. Zero `#[serial]`. Zero fixed sleeps in non-chaos tests.
- **AGPL-3.0-or-later** — All Cargo.toml + all .rs files aligned. `deny.toml` enforced.

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
