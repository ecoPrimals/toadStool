# ToadStool Documentation Hub

**Last Updated**: Aug 10, 2026 — S378

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
| Deploy to production | [docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md](docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md) |
| FHE encryption | [docs/guides/QUICK_START_ENCRYPTION.md](docs/guides/QUICK_START_ENCRYPTION.md) |
| Run tests | [docs/guides/TESTING.md](docs/guides/TESTING.md) |
| Deploy NPU drivers | [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md) |
| NPU driver design | [specs/NPU_DRIVER_ARCHITECTURE.md](specs/NPU_DRIVER_ARCHITECTURE.md) |
| Multi-tenant security | [specs/MULTITENANT_COMPUTE_ARCHITECTURE.md](specs/MULTITENANT_COMPUTE_ARCHITECTURE.md) |

---

## Current State (S378 — Aug 2026)

**Post-budding, dependency-sovereign, IPC-first, fully concurrent, capability-based.** barraCuda is a separate primal at `ecoPrimals/barraCuda/`. ToadStool is the hardware infrastructure layer — GPU/NPU/CPU discovery, capability probing, workload orchestration, and shader dispatch.

- **9,008+ lib tests**, 0 failures, 0 clippy warnings, 0 fmt diffs. Full workspace concurrent test suite.
- **16/16 cross-arch native targets** (S369) — x86_64/aarch64/armv7/riscv64/ppc64le/s390x/loongarch across Linux/macOS/Windows/iOS/Android.
- **38/48 crates WASM** (S376) — compute kernel on `wasm32-unknown-unknown` + `wasm32-wasip1`. Tokio optional via `runtime` feature gate. `tokio::fs`/`tokio::process` fully eliminated from production code.
- **NUCLEUS manifest convergence** (S377) — 5→2 `BiomeManifest` structs. All subsystems (CLI, daemon, biomeOS, integration-primals) consume the single canonical type from `toadstool-core`.
- **Tokio vestigial segmentation** (S378) — ~35k LOC feature-gated behind non-default features (`legacy-cloud`, `legacy-security`, `legacy-scheduler`, `legacy-protocol-client`, `legacy-security-client`, `hardening`, `background-monitors`, `cli-monitoring`, `network-scan`). Default-build tokio surface reduced 118→65 production files (45%). `runtime/edge` excluded. GPU/WASM `tokio::sync` → `std::sync`. Server background monitors gated.
- **126 JSON-RPC methods** (18 capability groups) + semantic registry. Wire Standard L3 (partial): `cost_estimates`, `operation_dependencies`. Self-audit verified (S372).
- **138 unsafe blocks** (all in hw-safe/GPU/VFIO/display/plugin containment); SAFETY-documented. Workspace `unsafe_code = "deny"`, **41 crates `forbid`**.
- **G68 platform containment complete** (S365) — zero rustix outside hw-safe.
- **Tokio blast radius reduced** (S376) — `tokio::fs`→`std::fs` (37 files), `tokio::process`→`std::process` (15 files), RwLock 99→20 files, workspace features 9→7. S374 — initial `runtime` feature gate, 20+ needless async→sync.
- **Zero production files >800L** (S373). Smart decomposition.
- **Zero dead deps** — S351: 48 eliminated. ecoBin v3.0 — zero C FFI deps. `deny.toml` ring + async-trait + zstd-sys bans.
- **Phase D: Sovereign dispatch validated** (S250–S263) — NV VFIO e2e on Titan V.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session history (S43–S378).

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

**DevOps Engineers**: [docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md](docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md) then [docs/guides/AKIDA_DRIVER_DEPLOYMENT.md](docs/guides/AKIDA_DRIVER_DEPLOYMENT.md)

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
