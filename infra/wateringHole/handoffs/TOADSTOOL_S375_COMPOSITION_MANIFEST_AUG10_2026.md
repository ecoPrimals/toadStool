# ToadStool S375 — NUCLEUS Composition Manifest + WASM Push

**Date**: Aug 10, 2026 | **Wave**: 157e | **Sprint**: S375

## Summary

Wave 157e introduced the NUCLEUS Composition Graph paradigm. ToadStool's
deliverable: formalize `biome.yaml` as the NUCLEUS sub-graph definition and
fix CLI divergence. Also continued the WASM push from S374.

## Work Completed

### 1. WASM Push: 26 → 31/48 Crates

Five "easy-win" crates feature-gated for WASM compatibility:

| Crate | Pattern |
|-------|---------|
| `toadstool-integration-storage` | `runtime` feature; client/pipelines/artifacts gated; types always available |
| `toadstool-management-performance` | `tokio::sync::RwLock` → `std::sync::RwLock`; tokio removed |
| `toadstool-management-analytics` | `runtime` feature; implementation + statrs gated; types/engine trait always available |
| `toadstool-runtime-specialty` | `runtime` feature; mainframe/embedded/industrial/realtime gated (hold guards across await → keep tokio::sync inside gate); types/config/emulation always available |
| `toadstool-security-policies` | `runtime` feature; file-based manager gated (uses tokio::fs); evaluator/types always available |

### 2. Canonical BiomeManifest (toadstool-core)

Created `toadstool_core::manifest::BiomeManifest` — the single canonical schema
that all ToadStool subsystems (CLI, daemon, biomeOS, integration-primals) should
converge on.

**Key additions over existing schemas:**
- `compositions: Vec<CompositionGraph>` — NUCLEUS sub-graph definitions
- `CompositionKind` — Tower, Nest, Node, Custom
- `CompositionReadiness` — health-based readiness gates
- `ManifestPrimalConfig.gossip_events` — swarmVine gossip injection declaration
- `ManifestFederation` — cross-gate federation config
- Generous `#[serde(default)]` on all optional fields for minimal manifests

Previous state: 4 divergent `BiomeManifest` structs in CLI, biomeOS integration,
and integration-primals (2 copies).

### 3. CLI Wiring

- `cli_root.rs::load_biome_manifest()` now tries canonical format first, then
  falls back to legacy CLI format. Transparent to existing manifests.
- `biome_model.rs` has `From<toadstool_core::manifest::BiomeManifest>` for
  lossless conversion from canonical to CLI's internal representation.

### 4. Example + Gossip Spec

- `examples/biome-strandgate.yaml` — checked-in reference manifest showing
  full composition graph with Tower Atomic and Node Atomic sub-graphs.
- `specs/GOSSIP_EVENTS.md` — taxonomy of events toadStool should announce
  to swarmVine (hardware, silicon, workload, runtime, node lifecycle).

### 5. Pre-existing Fix

- Fixed stale `.await` on `find_primals_by_capability()` in examples
  (left over from S374's needless async removal).

## Gaps for Upstream Overwatch

1. **Remaining 7 "potentially WASM" crates** — auto-config, client, protocols,
   monitoring, distributed, runtime-wasm, runtime-gpu — need deeper feature-gating
2. **Integration-primals manifest convergence** — 2 internal `BiomeManifest`
   structs should alias or convert to `toadstool_core::manifest::BiomeManifest`
3. **biomeOS integration manifest convergence** — same as above
4. **Daemon `biome_yaml` execution** — still stubbed (`workload_manager.rs`)
5. **`toadstool init` template update** — should generate canonical format
6. **primalSpring** needs to consume this manifest for composition lifecycle

## Artifact Summary

| Metric | Before | After |
|--------|--------|-------|
| WASM crates | 26/48 (54%) | 31/48 (65%) |
| BiomeManifest definitions | 4 divergent | 1 canonical + 3 legacy (bridged) |
| Composition graph spec | None | toadstool-core + example + CLI loader |
| Gossip injection spec | None | specs/GOSSIP_EVENTS.md |
