# ToadStool Documentation Hub

**Last Updated**: April 21, 2026 — S175

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

## Current State (S175 — April 21, 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent, capability-based.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **20,000+ tests** (7,818 lib-only S175), 0 failures, 0 clippy warnings, 0 fmt diffs. Full workspace concurrent test suite.
- **65 JSON-RPC methods** (incl. `compute.execute` direct route S203f). Wire Standard L3 (partial): `cost_estimates`, `operation_dependencies`. IPC compliant (`health.liveness` → `{"status":"alive"}`, `health.readiness` → ready+version, `health.check` full envelope, `capabilities.list`, `identity.get`).
- **Dual-socket IPC** — `compute.sock` (JSON-RPC primary, biomeOS routes here) + `compute-tarpc.sock` (tarpc hot-path). Override: `TOADSTOOL_SOCKET` / `TOADSTOOL_TARPC_SOCKET`. Family: `compute-{fid}.sock` / `compute-{fid}-tarpc.sock`.
- **Pipeline dispatch** — `compute.dispatch.pipeline.submit` + `.status` for ordered multi-stage workloads (DAG, topological sort, result forwarding). Resolves neuralSpring PG-05.
- **Capability-based everywhere**: 0 production hardcoded primal names, 0 production mocks, 0 production unwraps, 0 TODOs/FIXMEs. All primal references use `PRIMAL_NAME` constant or capability identifiers.
- **52 production files refactored (S203c/e/g/i)** via test extraction. 6 deprecated zero-caller items removed (S203g). 25 production files remain >500 lines (pure production code — hardware drivers, type defs, crypto managers; no extractable test blocks, all <700L).
- **TCP idle timeout (S203h)** — `TCP_IDLE_TIMEOUT_SECS` (300s configurable), `TCP_NODELAY` on all accepted streams. Resolves primalSpring benchScale exp082.
- **BTSP Phase 2 + Auto-Detect** — Handshake enforced on every UDS accept path; auto-detects plain-text clients (primalSpring) and degrades gracefully.
- **async-trait DEPRECATED** (S203r) — fully removed and banned in `deny.toml`. All ~91 annotations evolved to manual `Pin<Box<dyn Future>>` (dyn-dispatched) or native AFIT (non-dyn), and subsequently enum dispatch + RPITIT (S203s). Zero runtime behavior change. Transitive only via axum/config/wiggle.
- **`deny.toml` ring ban active** — ecoBin v3 compliant. `ring` absent from lockfile.
- **49 unsafe blocks (all in hw-safe/GPU/VFIO/display containment crates)**; all SAFETY-documented with `debug_assert!` pre-conditions. Workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow]`.
- **Edge discovery evolved (S203m)** — USB via `/sys/bus/usb/devices/`, Bluetooth via sysfs adapter enumeration, IPv6 via `/proc/net/if_inet6`. All gracefully degrade on non-Linux.
- **Scheduler queuing (S203m)** — `schedule_job` → `UniversalJobQueue::add_job` inserts into per-priority queues (was metadata-only). `schedule_local_job` logs post-enqueue telemetry.
- **Hardcoding sweep (S203m–p)** — sysfs/procfs paths centralized to `platform_paths`; all `TOADSTOOL_*` env var literals interned to `socket_env` constants (~55 new in S203p). `env_overrides` subsystem fully converted.
- **I/O-logic separation (S203o)** — 5 modules refactored to extract pure parsers from I/O wrappers (detection, GPU, DNS, storage, OS). +38 tests for pure logic. Real monitoring via `toadstool_sysmon` + `rustix::fs::statvfs` replaces hardcoded metrics. `StorageStatus::LocalOnly` replaces fake success on RPC failure.
- **Coverage push (S203n–p)** — +188 tests across 20+ modules (server, distributed, CLI, integration, WASM, resource optimizer/estimator, semantic methods, workload routing).
- **ecoBin v3.0** — Zero C FFI deps. `serialport` feature-gated in specialty crate. Crypto delegated to security service. HTTP delegated to coordination service.
- **Headless GPU** — `TOADSTOOL_HEADLESS=1` env var for pure headless operation. wgpu crash isolation via `catch_unwind` + async timeout (S203g: evolved from blocking poll).
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
