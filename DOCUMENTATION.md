# ToadStool Documentation Hub

**Last Updated**: March 13, 2026 -- S152

---

## Quick Navigation

| I Want To... | Document |
|--------------|----------|
| Get started | [README.md](README.md) |
| See current status | [STATUS.md](STATUS.md) |
| Commands and API reference | [QUICK_REFERENCE.md](QUICK_REFERENCE.md) |
| See active debt and evolution paths | [DEBT.md](DEBT.md) |
| Track evolution progress (single source) | [EVOLUTION_TRACKER.md](EVOLUTION_TRACKER.md) |
| Universal precision design | [specs/UNIVERSAL_PRECISION_ARCHITECTURE.md](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) |
| Roadmap and next steps | [NEXT_STEPS.md](NEXT_STEPS.md) |
| Sovereign compute roadmap | [SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md) |
| See all JSON-RPC methods | [QUICK_REFERENCE.md](QUICK_REFERENCE.md#json-rpc-methods-94-total-dynamically-built) |
| Hardware Transport Layer | [specs/HARDWARE_TRANSPORT_SPEC.md](specs/HARDWARE_TRANSPORT_SPEC.md) |
| Dual-Fabric Architecture | [specs/DUAL_FABRIC_ARCHITECTURE.md](specs/DUAL_FABRIC_ARCHITECTURE.md) |
| Try GPU operations | See barraCuda (`ecoPrimals/barraCuda/`) |
| Learn FHE | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Use scientific computing | See barraCuda (`ecoPrimals/barraCuda/`) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| Understand NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |
| Hybrid FP64 core streaming | [specs/HYBRID_FP64_CORE_STREAMING.md](specs/HYBRID_FP64_CORE_STREAMING.md) |

---

## Current State (S152 — March 13, 2026)

**Post-budding.** barraCuda is now a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration. coralReef shader compilation proxy with capability-based discovery.

- **Standalone-resilient** — Pull to any machine, `cargo test` works. GPU-optional with CPU fallback. Device-lost recovery via `poll_safe()`.
- **ecoBin v3.0** — Zero C FFI deps. `toadstool-sysmon` pure Rust `/proc` parser replaces sysinfo. PyO3 feature-gated only.
- **AGPL-3.0-only** — All Cargo.toml + all .rs files aligned to single license. `deny.toml` enforced.
- **Fully concurrent tests** — All tests run with `--test-threads=8`. Zero `#[serial]`. Zero fixed sleeps in non-chaos tests.
- **Deep debt: clean** — Zero `chrono`, zero `log` (core), zero `pollster`, zero `serde_yaml`, zero `sysinfo`, zero `libc` (akida-driver→rustix), zero production stubs/mocks, ~70+ justified `unsafe` blocks (all SAFETY documented), zero hardcoded localhost/ports, zero `Box<dyn Error>`, zero blind `.unwrap()`, zero `todo!()`, zero `unimplemented!()`, zero FIXME/HACK, zero `dbg!()`, zero stale TODOs. 5 crates migrated to native AFIT. All env tests thread-safe via `temp_env`.
- **Sovereignty RESOLVED** — All production callers use `get_socket_path_for_capability()` or `interned_strings::primals::*`. Legacy name-based APIs fully deprecated.
- **Sovereign pipeline** — `HardwareFingerprint`, `is_sovereign_capable()`, `safe_allocation_limit` (NVK PTE guard), 12-variant `SubstrateCapabilityKind`, 8-variant `SubstrateType`.
- **NPU dispatch** — Generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter. `NpuParameterController` trait (absorbed from hotSpring).
- **GPU capability probing** — `GpuAdapterInfo` struct exposes detailed adapter info for barraCuda. Multi-adapter selection via `TOADSTOOL_GPU_ADAPTER`.
- **NestGate integration** — Real JSON-RPC `storage.artifact.store`/`retrieve` with graceful fallback.
- **Module architecture** — 45+ large files refactored into domain modules. Wildcard re-exports narrowed in 13 crates.
- **Capability-based discovery** — Primals discover each other by capability, not name. Edge platforms probe real hardware. All primal names via `interned_strings::primals::*`.
- **coralReef shader proxy** — `shader.compile.*` handlers proxy to coralReef with capability-based discovery and graceful naga fallback.
- **Cross-spring provenance** — `toadstool.provenance` JSON-RPC method exposes 17+ documented cross-spring flows for ecosystem introspection.
- **20,262 workspace tests** (S152) | ~86% line coverage (llvm-cov verified) | all quality gates green (0 warnings, clippy pedantic clean)
- **95+ JSON-RPC methods** (dynamically built from semantic registry)
- **JSON-RPC only** — REST API + middleware removed (S90/S92). All IPC via JSON-RPC 2.0.

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



**[specs/](specs/)** -- Technical specifications (FP64 evolution, hybrid core streaming, NPU, multi-tenant, cross-platform).

**[docs/architecture/](docs/architecture/)** -- Design documents, ADRs, migration patterns.

---

## Guides

**[docs/guides/TESTING.md](docs/guides/TESTING.md)** -- Testing strategy: unit, integration, property-based, fault, chaos testing.

**[docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)** -- NPU driver deployment.

BarraCuda guides have been fossilized to `ecoPrimals/fossil/toadStool/docs-S94b/`. See the barraCuda primal for current docs.

---

## Scientific Middleware

Scientific computing middleware (linalg, numerical, special, stats, optimize, surrogate, sample, PDE) has moved to **barraCuda** (`ecoPrimals/barraCuda/`). Legacy API examples are preserved in [QUICK_REFERENCE.md](QUICK_REFERENCE.md#scientific-computing-middleware-api) for reference.

---

## By Role

**ML/AI Engineers**: [README.md](README.md) then see barraCuda (`ecoPrimals/barraCuda/`)

**Computational Scientists**: See barraCuda (`ecoPrimals/barraCuda/`) for scientific middleware

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
DOCUMENTATION.md           -- This file (navigation hub)
docs/
  guides/                  -- Deployment and usage guides
  architecture/            -- Design documents and ADRs
  reference/               -- API reference, constants
specs/                     -- Technical specifications
```
