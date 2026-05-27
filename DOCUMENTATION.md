# ToadStool Documentation Hub

**Last Updated**: May 2026 — S277

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

## Current State (S277 — May 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent, capability-based.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **23,000+ tests** (9,161+ lib-only), 0 failures, 0 clippy warnings, 0 fmt diffs. Full workspace concurrent test suite.
- **88 JSON-RPC methods** (direct) + semantic registry. Wire Standard L3 (partial): `cost_estimates`, `operation_dependencies`. **Recommended caller timeout: ≥3 seconds** for health probes during startup.
- **Phase C complete** (S245–S253) — toadstool-cylinder (153 .rs, 700 tests), DRM/MMIO/AMD/NVIDIA/VFIO hardware modules absorbed from `coral-driver`. `OwnedFd` VFIO fd ownership (S253). SwapOrchestrator real quiesce/persist/restore (S253). `toadstool device` CLI with swap/list/status/warm subcommands (S253). GspBridge trait boundary.
- **Phase D: Sovereign dispatch validated** (S250–S263) — `try_local_dispatch()` via `ComputeDevice` trait before `coral_client` IPC forward. Full buffer lifecycle. AMD DRM dispatch live. **NV VFIO e2e dispatch validated on Titan V** (S263): warm handoff → VFIO open → channel → DMA roundtrip → GR init. Current frontier: FECS PENDING_CTX_RELOAD.
- **Stale socket hygiene** (S264) — CLI daemon SIGTERM + socket cleanup. Display IPC Drop impl. UDS unlink-before-bind audited.
- **sporePrint Wave 28** (S265) — `sporeprint/validation-summary.md` + CI dispatch to sporePrint.
- **Neural API primal.announce wiring** (S270) — `primal.announce` wired into JSON-RPC dispatch, startup self-announcement to biomeOS Neural API with capabilities (compute, science, inference), cost hints, latency estimates, signal tier (node). 88 JSON-RPC methods.
- **Sandbox working_dir production** (S269) — `data_dependencies` pre-dispatch validation with BLAKE3 integrity. `SandboxSpec.working_directory` wired into sandbox manager. 90+ upstream clippy errors absorbed.
- **Deep Debt** (S240–S273) — All Duration literals extracted to named constants. `CORALREEF_*` env vars deprecated with `TOADSTOOL_*` primaries + deprecation warnings (S253). Zero `#[allow(deprecated)]` remaining. All lint attrs have `reason`. Zero production mocks/TODO/FIXME/unreachable!(). All unsafe SAFETY-documented. `cargo deny check bans` passes clean.
- **Deep Debt Evolution** (S273) — Production panic surface eliminated (`kernel_health.rs`, dispatch cache, `ember_client.rs`, `secure_enclave`). `dispatch/mod.rs` 1,638→839L via `dispatch/sovereign.rs` extraction. `warm_init.rs` → module dir. 6 CLI `well_known::*` sites migrated to capability-based discovery. VFIO `activity_tracker().record()` wired. hw-safe abstractions validated.
- **Wave 54: Early Health Responder** (S277) — Health check unresponsive on southGate fixed. Early health responder on pre-bound socket during startup. BTSP not required for health probes.
- **Deep Debt Evolution II** (S276) — Remaining production unwrap/expect/unreachable eliminated. `handler/sovereign.rs` 1,003L → module directory. `memmap2` removed from hw-safe (rustix mmap). 3 primal-name type aliases deprecated. `ipc.register` capability list aligned to Node Atomic set.
- **Capability-based everywhere**: 6 CLI hardcoded primal name sites migrated to capability-based discovery (S273); ~400 intentional legacy-compat refs remain (env fallbacks, serde aliases). 0 production mocks. All production logging via `tracing`.
- **ecoBin v3.0** — Zero C FFI deps. `deny.toml` ring + async-trait + zstd-sys bans active.
- **46 unsafe blocks** (all in hw-safe/GPU/VFIO/display/plugin containment crates); all SAFETY-documented. Workspace `unsafe_code = "deny"`, **41 crates `forbid`**.
- **Dual-socket IPC** — `compute.sock` (JSON-RPC primary) + `compute-tarpc.sock` (tarpc hot-path).

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session history (S43–S277).

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
  daemon/                  -- Daemon mode user guide
  debt/                    -- Debt tracking details
specs/                     -- Technical specifications
```

**Fossil record** — 9 session trackers archived with `_S166` suffix under `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`.
