# ToadStool S284 — Deep Debt Evolution VI: Large File Splits + Deprecated Cleanup
**Date**: Jun 3, 2026
**Session**: S284
**Status**: Complete — all changes committed and pushed

## Actions Taken

### 1. Large File Smart Refactoring (3 files >800L → all under 500L)

**`sovereign_init.rs` (991L) → 7 modules:**
- `mod.rs` (216L) — orchestration: pre_memory → memory_path → post_memory
- `pre_memory.rs` (215L) — identity probe, PMC enable, PGRAPH reset, CG/PRI, PGOB
- `memory_path.rs` (332L) — boot-state probe, cold early-exit, early falcon, memory training
- `post_memory.rs` (181L) — falcon boot, GR init, verify
- `context.rs` (78L) — shared PipelineCtx
- `result.rs` (60L) — finish/finish_halted helpers
- `engine_ungate.rs` (34L) — GR sequence replay

**`open_vfio.rs` (949L) → 6 modules:**
- `open_vfio.rs` (232L) — main `open_vfio` entry point
- `open_vfio_fecs_probe.rs` (152L) — PMC cold boot, PRI ring, FECS probe
- `open_vfio_pgraph.rs` (149L) — PGRAPH ungating + FECS channel
- `open_vfio_pfifo_recovery.rs` (344L) — PFIFO fault detection and recovery
- `open_vfio_catalyst.rs` (136L) — catalyst warm-handoff path
- `open_vfio_readiness.rs` (55L) — post-init dispatch readiness

**`experiment.rs` (911L) → 5 modules:**
- `experiment.rs` (40L) — re-exports + dispatcher
- `experiment_snapshot.rs` (250L) — SovereignSnapshot, diff/capture
- `experiment_chip.rs` (102L) — chip detection
- `experiment_stage_init.rs` (160L) — stages 1-3 (PFIFO/CG, PGOB, PRI)
- `experiment_stage_ungate.rs` (384L) — stages 4-6 (GPC MMU, FECS, ungating)

### 2. Production Panic Elimination (2 remaining → 0)
- `kernel_sentinel.rs` — thread spawn `.expect()` → `std::io::Result<()>`, callers log+continue
- `visualization_client.rs` — guard `.expect()` → `Option<&T>` with `debug_assert!`

### 3. Env Var Migration
- `akida-setup/main.rs` — `env::var("HOME")` → `socket_env::HOME`

### 4. SAFETY Documentation
- `pmc.rs` — mmap unsafe block SAFETY comment added

### 5. Deprecated Cleanup
**Removed (zero production callers):**
- `BearDogBackend` type alias + re-export
- `capability_to_service`, `service_to_capability`, `get_capability_to_legacy_map`, `legacy_service_name_for_capability`, `capabilities_to_dependencies`, `get_capability_mappings`
- `EcosystemCoordinator::get_primal_status`, `is_primal_available`, `get_primal_capabilities`

**Tightened:**
- 30 `LEGACY_*` socket_env deprecations: added `since = "0.4.0"`
- `discovery_engine`: `well_known::BIOMEOS` → `runtime_types::BIOMEOS`

### 6. Clippy + Test Fixes
- 33 clippy warnings in toadstool-server resolved
- Dead code removed: `FALLBACK_DKMS_VERSION`, `discovered_nvidia_dkms_version()`
- `channel_init.rs`: `#[allow(clippy::too_many_arguments)]` with reason
- Test compilation: fixed `cloud_orchestrator_coverage_tests.rs` duplicate `#[path]`
- Test naming: updated `beardog` → `crypto` in template tests

## Metrics
| Metric | Before | After |
|--------|--------|-------|
| Production files >800L | 3 | 0 |
| Production library panics | 2 | 0 |
| Dead deprecated symbols | 12 | 0 |
| Workspace clippy warnings | 33 | 0 |
| Test suite | failing | 100% pass |
| Env centralization | ~97% | ~98% |

## Remaining Debt (prioritized)
1. **distributed::security → crypto_integration**: `submit.rs` still uses deprecated `toadstool_distributed::security::*`
2. **distributed::coordination**: large module marked deprecated but still compiled/exported
3. **LEGACY_* env fallbacks**: 30 constants still read in production identity chains
4. **rm_trigger bin**: multiple `.unwrap()` on hardware ioctl paths (acceptable for diagnostic CLI)
