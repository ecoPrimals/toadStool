# ToadStool S378 — Tokio Vestigial Segmentation

**Date**: Aug 10, 2026 | **Sprint**: S378 | **Gate**: strandGate

## Summary

The "irreducible" ~118-file tokio production surface was largely vestigial — primordial code reimplementing what Tower Atomic primals (songBird, bearDog, cellMembrane) and biomeOS now own. Feature-gated ~35k LOC of dead modules behind non-default features. Migrated remaining safe `tokio::time`/`tokio::sync` to `std` equivalents.

## What Changed

### Feature-gated vestigial modules (preserved as fossil record)

| Feature | Module | LOC | Reimplements |
|---------|--------|-----|--------------|
| `legacy-cloud` | `distributed/cloud/` | ~7.8k | biomeOS graph executor |
| `legacy-security` | `distributed/security/` + `security_provider/` + `crypto_lock/` | ~12k | bearDog via `crypto_integration` |
| `legacy-scheduler` | `distributed/universal/scheduler` + adapter + platform | ~1k | biomeOS + core scheduler |
| `legacy-protocol-client` | `protocols/client/` + root `transport` | ~2.5k | biomeOS capability routing |
| `legacy-security-client` | `protocols/security_client/` | ~2k | bearDog via `crypto_integration` |
| `hardening` | `performance_hardening/async_ops,caching` + `circuit_breaker` + `intrusion` | ~3k | Zero production callers |

### Safe migrations to `std`

- **`tokio::time::Duration` → `std::time::Duration`**: 8 CLI files (same type re-exported)
- **`tokio::time::Instant` → `std::time::Instant`**: 2 files (benchmarking, intrusion)
- **`tokio::sync::RwLock` → `std::sync::RwLock`**: 6 files (guards not held across `.await`)
- **`tokio::sync::Mutex` → `std::sync::Mutex`**: 2 files (sync-only scoped locks)

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Default-build tokio production files | ~118 | ~85 |
| Vestigial LOC in default build | ~35k | 0 (gated) |
| `tokio::time` imports | 12 | 0 |
| `tokio::sync::RwLock` files | ~20 | ~14 |
| `tokio::sync::Mutex` files | ~6 | ~4 |

## Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace --lib` — all pass
- Feature-gated code compiles when features enabled

## Wave 157g Alignment

- **Manifest convergence** — S377 DONE (prerequisite completed last sprint)
- **Tokio debt** — S378 continues S374-S376 deep debt arc. Remaining tokio is genuinely irreducible: networking, task spawning, async I/O, channels, signals in the deployment layer.

## Remaining Irreducible Tokio (~85 files)

Genuinely needed for the async deployment layer:
- `server/` — JSON-RPC server, BTSP, background services, transport
- `core/toadstool` — IPC, workload dispatch (spawn + channels)
- `core/common` — BTSP protocol, service discovery
- `runtime/` — display IPC, GPU dispatch, container BYOB
- `distributed/` — coordination_integration, crypto_integration, substrate
- `cli/` — daemon lifecycle, monitoring

## Not in Scope

- Deleting vestigial code (feature-gating preserves as fossil record)
- `#[tokio::test]` migration (468 instances — separate sprint)
- Wiring `primal_capabilities/` to server startup (future composition work)
- Replacing `RemoteDispatcher` transport with songBird MeshRelay
