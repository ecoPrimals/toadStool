# toadStool S273 — Deep Debt Evolution

**Date**: May 24, 2026
**Session**: S273
**From**: toadStool team
**To**: primalSpring (downstream audit)

---

## Summary

Comprehensive deep debt evolution pass across 6 dimensions: production panics,
large file refactoring, hardcoded primal names, unsafe consolidation, stale
documentation, and dead code.

## Production Panic Surface — Eliminated

| File | Before | After |
|------|--------|-------|
| `kernel_health.rs` | 29 `.unwrap()` on ELF parsing | `?` + `KernelHealthError::ElfParse` helpers |
| `dispatch/mod.rs` | `.expect("just inserted")` on cache lookup | `ok_or_else(JsonRpcError::internal_error)` |
| `ember_client.rs` | 5 `.expect("checked len")` on SCM_RIGHTS fds | `?` + `DriverError::DeviceNotFound` |
| `secure_enclave` | 2 fallible `Default` impls with `.expect()` | Removed — types already expose `::new() -> Result` |

**Zero production `.unwrap()` / `.expect()` / `panic!()` confirmed.**

## Large File Refactoring

| File | Before | After |
|------|--------|-------|
| `dispatch/mod.rs` | 1,638 lines | 839 lines (sovereign.rs: 814 lines) |
| `warm_init.rs` | 1,439 lines | Module dir: mod.rs (372) + seeders.rs (389) + trials.rs (699) |

- 7 sovereign GPU handlers + 2 helpers extracted to `dispatch/sovereign.rs`
- Seeder strategies and trial types extracted to submodules

## CLI Capability-Based Discovery

6 `well_known::*` hardcoded primal name sites migrated to capability-based
discovery with legacy fallback:

1. `cli_root.rs` — manifest validation → `has_primal_with_capability("crypto")`
2. `start.rs` — biome startup → `find_primal_with_capability("crypto")`
3. `checks.rs` — removed pre-seeded primal names, discover from socket scan
4. `config.rs` — socket paths → `get_socket_path_for_capability()`
5. `integrator_impl.rs` — capability-only matching
6. `basic_templates.rs` — `BEARDOG` → `service_names::CRYPTO`

Added: `PrimalConfig::has_capability()`, `BiomeManifest::has_primal_with_capability()`,
`BiomeManifest::find_primal_with_capability()`.

## Dead Code — activity_tracker() Wired

`activity_tracker().record()` wired into 7 VFIO dispatch paths:
- `device_vfio_open`, `device_vfio_roundtrip`
- `sovereign_init_ember`, `sovereign_ce_validate_ember`
- `sovereign_pmu_investigate`, `sovereign_catalyst_boot`
- `sovereign_profile_ember`

`#[allow(dead_code)]` removed from `activity_tracker()`.

## Unsafe Consolidation — Validated

`hw-safe` already contains shared abstractions (`DeviceMmap`, `VolatileMmio`,
`vfio_setup`, `vfio_dma`). Cylinder migration deferred to avoid upstream merge
conflicts. Detailed migration plan documented for future wave.

## Debris Cleanup

- Removed orphan `crates/barracuda/src/shaders/linalg/nmf_f64.wgsl` (last
  remnant of barraCuda budding, not in workspace Cargo.toml)
- Removed stale `squirrel_mcp_coordination_demo.rs` example (stub printing
  "not implemented", referencing non-existent guide)
- Fixed stale `barracuda::ops::fhe_ntt` reference in pending test README

## Documentation

- `CONTEXT.md` health.liveness updated to S272 always-alive behavior
- All root docs (README, CONTEXT, DOCUMENTATION, NEXT_STEPS) updated to S273
- sporeprint/validation-summary.md updated to S273
- docs/README.md, docs/guides/TESTING.md, .env.example updated to S273

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,131+ |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 88 |
| Clippy warnings | 0 |
| Production panics | 0 |
| Cylinder tests | 700 |
| Workspace crates | 47 |

## Files Changed

22 files changed, 2,483 insertions, 2,389 deletions.

---

Ready for downstream primalSpring audit.
