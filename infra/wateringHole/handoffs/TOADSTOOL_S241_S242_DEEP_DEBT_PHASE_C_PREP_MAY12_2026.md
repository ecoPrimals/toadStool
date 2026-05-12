# ToadStool S241–S242 Handoff — Deep Debt + Phase C Preparation

**Date**: May 12, 2026
**Sessions**: S241, S242
**From**: toadStool team
**Status**: 8,286 lib-only tests, zero clippy warnings, zero production debt

---

## S241 — Deprecated Stub Removal, Coverage Expansion, Phase C Planning

### Changes

- **`cuda_impl` removed entirely**: Deprecated `CudaBackend` / `CudaComputeResource`
  stubs and re-exports deleted (zero callers confirmed workspace-wide). Migration
  guidance preserved in `backends/mod.rs` doc comments pointing to `gpu.dispatch.cuda`
  capability IPC
- **SwapOrchestrator coverage expanded** (+3 tests):
  - `orchestrate_swap_release_failure_is_non_fatal` — release errors → `StepStatus::Skipped`
  - `orchestrate_swap_unhealthy_device_fails_at_health_step` — unhealthy swap → `Failed` health
  - `execute_boot_with_unhealthy_swap_reports_failure` — boot failure propagation
- **Phase C coral-driver split plan**: Created `PHASE_C_CORAL_DRIVER_SPLIT_PLAN.md`
  documenting hardware-lifecycle (toadStool absorbs) vs compiler-pipeline (coralReef
  retains) module boundaries. VFIO, DRM enum, AMD GEM/PM4, NVIDIA BAR0/pushbuf/QMD
  → toadStool; GSP firmware, Intel skeleton, compiler IR → coralReef
- **Discovery timeout audit**: Confirmed `DEFAULT_DISCOVERY_TIMEOUT_SECS` (5s) is
  per-source sequential, GPU sysfs scan is synchronous — no timeout issue

### Quality

- 8,281 lib-only tests (65 glowplug, +3 new)
- Zero clippy warnings

---

## S242 — Deep Debt: println→tracing, Magic Constants, Coverage, Dep Cleanup

### Changes

- **auto_config `SystemSummary::display()` → `tracing::info!`**: Last remaining
  `println!` in library code migrated to structured tracing with cpu, memory, gpu,
  storage, performance, ecosystem_services fields. Zero library println/eprintln
  remaining
- **20+ hardcoded Duration literals → named constants** across:
  - `biomeos_integration/types/resources.rs` — health check defaults
  - `runtime_discovery/config.rs` — discovery interval/timeout
  - `security_hardening/rate_limiter.rs` — `SECS_PER_DAY`
  - `performance_hardening/types.rs` — cleanup, sampling, timeout, pool defaults
  - `server/resource_estimator/estimator.rs` — per-operation duration estimates
- **ContiguousBytes direct tests** (+5): `as_bytes_returns_correct_content`,
  `as_bytes_mut_allows_modification`, `empty_region_returns_empty_slice`,
  `empty_region_mut_returns_empty_slice`, `raw_len_matches_as_bytes_len`
- **Python crate Cargo.toml cleaned**: Removed orphan `pyo3 = { workspace = true }`
  and `pyo3-asyncio = { workspace = true }` (workspace deps already deleted per
  ecoBin v3.0); dormant `python-embedded` feature documented

### Quality

- 8,286 lib-only tests (+5 hw-safe)
- Zero clippy warnings
- Zero library println/eprintln
- Zero TODO/FIXME in production code

---

## Current Handoff State

Active handoffs in `infra/wateringHole/handoffs/`:
- `TOADSTOOL_S240_DEEP_DEBT_TEST_REFACTOR_TRACING_MAY12_2026.md`
- `TOADSTOOL_S241_S242_DEEP_DEBT_PHASE_C_PREP_MAY12_2026.md` (this file)
- `PHASE_C_CORAL_DRIVER_SPLIT_PLAN.md` (active planning)

Archived to `ecoPrimals/infra/wateringHole/fossilRecord/`:
- S234 (IPC env expansion), S235 (Wave 8 foundation), S236 (deep debt),
  S237 (Phase A ember), S238 (deep debt + JH-2), S239 (Phase B glowplug)

---

## Next Work

1. **Phase C — cylinder + coral-driver absorption**: Per split plan, absorb VFIO
   BAR0+DMA, nouveau sovereign, amdgpu GEM+PM4, multi-GPU scan. Generalize into
   `toadstool-cylinder` for per-device subprocess isolation and resource fencing
2. **Phase D — local dispatch (Gate 4)**: Wire `compute.dispatch.submit` to execute
   locally through absorbed driver layer
3. **Coverage push 83.6% → 90%**: Hardware mocks (MockSwapExecutor, MockResourceHandle)
   for CI deterministic test coverage
4. **Box<dyn> evolution in runtime/edge**: Communication, discovery, platform modules
   are candidates for enum dispatch
