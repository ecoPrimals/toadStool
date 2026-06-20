# ToadStool Documentation Hub

**Last Updated**: Jun 20, 2026 — S321+

---

## Fossil Record

These root documents were **fully resolved** and **fossilized** in the ecosystem fossil record (see [github.com/ecoPrimals/fossilRecord](https://github.com/ecoPrimals/fossilRecord)): **UNSAFE_AUDIT_REPORT**, **SOVEREIGN_COMPUTE_GAPS**, **PURE_RUST_TRACKING**, **STATUS**, **EVOLUTION_TRACKER**, **QUICK_REFERENCE**, **SOVEREIGN_COMPUTE**, **SPRING_ABSORPTION_TRACKER**, **BREAKING_CHANGES** — all renamed with `_S166` suffix.

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

## Current State (S321+ — Jun 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent, capability-based.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **23,000+ tests** (9,069+ lib-only), 0 failures, 0 clippy warnings, 0 fmt diffs. Full workspace concurrent test suite.
- **112 JSON-RPC methods** (17 capability groups) + semantic registry. Wire Standard L3 (partial): `cost_estimates`, `operation_dependencies`.
- **100% SPDX AGPL-3.0-or-later** headers across all `.rs` files (S320+).
- **Zero-copy dispatch** (S320+) — `Arc<EncryptionKey>` cache, pipeline first-stage borrow, `binary_size` telemetry, error consolidation.
- **riboCipher genetics** — CLEAR `0xEC` + MitoBeacon `0xED` accepted on all accept loops (S311+S320). REJECT on unsignalled (Wave 113). Nuclear `0xEE` deferred to Wave 115.
- **gRPC + OpenCL DELETED** (S319) — enum variants, config, stubs, detection all removed (−458 lines).
- **Bare `"health"` method** (S315) — returns `{status, primal, version}` per GuideStone standard.
- **Zero hardcoded cross-primal names** — `LEGACY_*_PASCAL` constants for wire compat (S320+).
- **Zero production files >750L** — `warm_swap.rs` 818L → 479L + 305L catalyst helpers (S320+). `reagent/mod.rs` 704L → 3 files (S321).
- **Env centralization complete** (S321) — zero production raw `env::var("...")` literals; `TOADSTOOL_RM_TRIGGER_BIN` + `TOADSTOOL_FORENSICS_LOG` added to `socket_env`.
- **Duration dedup** (S321) — `CPU_USAGE_SAMPLE_WINDOW` unified across 5 crates; 8 additional named constants.
- **Dep unification** (S321) — `bytes`/`ruzstd`/`serialport`/`ndarray` workspace-unified; zero version drift.
- **TOADSTOOL-AUTO-REGISTER** (S309, Wave 111) — PCI sysfs GPU/NPU in `ipc.register` + `primal.announce`.
- **PRIMAL-SOCKET-CLEANUP** (S308, Wave 107) — `BIOMEOS_SOCKET_DIR` wired; zero `/tmp` writes.
- **Coverage Push I–IV** (S294–S298) — +174 tests, `--socket` wired, `--headless` mode, musl-static VPS binary.
- **Phase D: Sovereign dispatch validated** (S250–S263) — NV VFIO e2e on Titan V. Frontier: FECS PENDING_CTX_RELOAD.
- **ecoBin v3.0** — Zero C FFI deps. `deny.toml` ring + async-trait + zstd-sys bans. Unused `cc`/`bindgen` removed (S320+).
- **44 unsafe blocks** (all in hw-safe/GPU/VFIO/display/plugin containment); SAFETY-documented. Workspace `unsafe_code = "deny"`, **41 crates `forbid`**.
- **Dual-socket IPC** — `compute.sock` (JSON-RPC primary) + `compute-tarpc.sock` (tarpc hot-path).

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session history (S43–S321+).

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
specs/                     -- Technical specifications
```

**Fossil record** — 9 session trackers archived with `_S166` suffix under `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`.
