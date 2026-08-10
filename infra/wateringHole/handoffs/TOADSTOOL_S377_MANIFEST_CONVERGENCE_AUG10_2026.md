# ToadStool S377 — NUCLEUS Manifest Convergence

**Date**: August 10, 2026
**Sprint**: S377
**Gate**: strandGate
**Wave**: 157g ENMESH (HIGH priority item: "Manifest convergence — 4 divergent BiomeManifest structs → 1 canonical toadstool-core")

---

## Summary

BiomeManifest struct convergence: **5 → 2**. Three divergent `BiomeManifest`
definitions replaced with re-exports of the canonical type from
`toadstool_core::manifest` (shipped S375).

## Before

| Location | Shape | Used By |
|----------|-------|---------|
| `toadstool-core/src/manifest.rs` | Canonical NUCLEUS type (compositions, gossip, federation) | New code |
| `cli/src/biome_model.rs` | CLI operational type with `From` bridge (S375) | CLI subsystem |
| `core/toadstool/biomeos_integration/types/manifest.rs` | biomeOS variant (PrimalsConfig, BiomeSecurity, etc.) | biomeOS integration |
| `integration/primals/src/integration_manifest.rs` | Simplified subset (api_version, kind, metadata, primals) | Integration trait |
| `integration/primals/src/manifest/biome.rs` | Flat layout (name, version directly, no metadata struct) | Orchestrator |

## After

| Location | Status |
|----------|--------|
| `toadstool-core/src/manifest.rs` | **Canonical** — unchanged |
| `cli/src/biome_model.rs` | **Bridge** — CLI operational types + `From<canonical>` |
| `biomeos_integration/types/manifest.rs` | **Re-export** — `pub use toadstool_core::manifest::{BiomeManifest, BiomeMetadata}` |
| `integration/primals/src/integration_manifest.rs` | **Re-export** — `pub use toadstool_core::manifest::{BiomeManifest, BiomeMetadata}` |
| `integration/primals/src/manifest/biome.rs` | **Re-export** — `pub use toadstool_core::manifest::{BiomeManifest, BiomeMetadata}` |

## Key Changes

### integration-primals
- `integration_manifest.rs`: local structs → re-export
- `manifest/biome.rs`: local structs → re-export
- `primal_types.rs`: added `PrimalConfig::from_manifest(name, &ManifestPrimalConfig)` bridge
- `manager.rs`, `orchestrator.rs`, `lib.rs`: updated to construct canonical `BiomeManifest`
- `Cargo.toml`: added `toadstool-core` dependency

### biomeOS integration
- `types/manifest.rs`: local `BiomeManifest`/`BiomeMetadata` → re-export; legacy `ServiceConfig`/`ServiceSource` retained
- `types/mod.rs`: Quick Start doc example updated for canonical type

### CLI (no change this sprint)
- Already converged in S375 via `From<toadstool_core::manifest::BiomeManifest>`

## Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace --lib` — 178 passed, 0 failed (3 ignored)

## Upstream Notes

- Wave 157g listed "Manifest convergence" as HIGH priority for toadStool + biomeOS
- biomeOS now consumes the canonical type; `nucleus.start` sub-graph executor can proceed
- primalSpring modernization can consume `biome.yaml` v1 without translation layer

## Remaining Work

- WASM 38/48 — compute kernel ceiling reached (remaining 10 irreducibly native)
- Gossip injection points for toadStool — not yet identified
- Cross-gate gossip — blocked on songBird MeshRelay + TCP 7800 reachability
