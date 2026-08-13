# ToadStool S381 — Deep Debt + Overstep Cleanup + Evolution

**Session**: S381
**Gate**: strandGate (golgi)
**Date**: Aug 13, 2026
**Focus**: Deep Debt + Overstep Cleanup + Evolution

## Completed

### Smart Refactor: `hw-safe/platform_backends.rs` → 7-Module Directory
- **805-line monolith split** — `crates/core/hw-safe/src/platform_backends.rs` replaced by directory module with 7 focused files: `memory.rs`, `event.rs`, `device_io.rs`, `system.rs`, `isolation.rs`, `ipc.rs`, `kmod.rs`.
- **Public API unchanged** — all symbols re-exported from `mod.rs`. Unsafe containment zones preserved. Linux-only gating retained per submodule.
- **Follows S373 partial decomposition** — completes the platform backend split started in prior deep-debt pass.

### Smart Refactor: `akida-driver/capabilities.rs` (813→760L)
- **Generic hwmon helper** — duplicated hwmon directory walk pattern extracted to `read_hwmon_sensor<T>()`.
- **`query()` / `from_sysfs()` merge** — identical bodies consolidated; single code path for sysfs capability discovery.
- **45 lines removed** — no behavior change, reduced duplication surface for future NPU capability extensions.

### Inter-Primal Overstep: `coral_client` → `shader_service`
- **8 dispatch handler files updated** — `shader_dispatch.rs`, `submit.rs`, `capabilities.rs`, `mod.rs`, `state.rs`, `handler/mod.rs`, `dispatch/tests/shader.rs`, `shader_dispatch_tests.rs`.
- **Naming reflects capability, not primal identity** — eliminates hardcoded coralReef identity knowledge from dispatch layer. `SharedVisualizationClient` field and availability checks now use `shader_service` throughout.
- **Tests renamed** — e.g. `shader_dispatch_drm_without_shader_service_returns_failed_envelope`.

### Legacy Test File Renames (History Preserved via `git mv`)
- `nestgate_pipeline_tests.rs` → `storage_pipeline_tests.rs`
- `nestgate_types_tests.rs` → `storage_types_tests.rs`
- `beardog_async_integration_tests.rs` → `protocol_async_integration_tests.rs`
- **nestGate/beardog naming retired** from test filenames; capability-aligned names (`storage`, `protocol`) match current primal identities.

### Hardcoded Paths → Env Discovery
- **Script BDFs** — `scripts/run-hardware-tests.sh` now uses `${AMD_BDF:-0000:25:00.0}`, `${NVIDIA_BDF:-0000:41:00.0}`, `${AKIDA_BDF:-0000:e2:00.0}` instead of inline PCI slot literals.
- **Akida module path** — `akida-setup/src/main.rs` reads `AKIDA_MODULE_PATH` with default fallback instead of hardcoded kernel module location.
- **Fleet-portable** — eastGate/westGate/southGate nodes can override BDFs without script edits.

### String Param Modernization
- **`EcosystemMessage`** — `{new, heartbeat, error}` accept `impl Into<String>`.
- **`SelfIdentity::with_network`** — `impl Into<String>` for network fields.
- **`ToadStoolIdentity::add_metadata`** — `impl Into<String>` for key/value pairs.
- **`ServiceEndpoint::with_capabilities`** — `impl IntoIterator` for capability lists.
- **Call-site cleanup** — unnecessary `.to_string()` removed where types now accept borrowed/coercible inputs.

### Orphan Documentation: `runtime/edge/DEPRECATED.md`
- **Documents orphaned status** — `toadstool-runtime-edge` excluded from workspace (S378), zero dependents, never built by default.
- **Future sprint decision recorded** — preserve as fossil vs. excise; modules listed (communication, deployment, discovery, platforms, serial_transport, toolchain, udev_pure).

### Deprecated Env Audit
- **Legacy fallback chains verified** — `LEGACY_BEARDOG_*`, `LEGACY_SONGBIRD_*`, and related constants correctly used as migration fallback paths with runtime warnings (`security_client/config.rs`, `security/discovery.rs`, `unibin/execution.rs`, `primal_discovery_complete/mod.rs`).
- **No premature removal** — deprecated env vars remain until all fleet nodes migrate to canonical `TOADSTOOL_*` names.

## Verification

- `cargo check --workspace` — **0 errors, 0 warnings**
- `cargo test --workspace --lib` — **8,446 passed, 0 failed** (full workspace, no exclusions)
- Rust edition 2024 throughout, MSRV 1.92
- All refactors preserve public API surface; re-exports verified

## Remaining (Tier 2 — for future sprints)

| Item | Priority | Notes |
|------|----------|-------|
| **Gossip injection (0/17 events)** | P2 | Spec exists (`specs/GOSSIP_EVENTS.md`); zero production wiring. Coordinate with swarmVine team for socket discovery. |
| **parking_lot evaluation** | P3 | Not currently a workspace dep; server + GPU coordinator would benefit from faster locks. |
| **Deprecated module cleanup** | P3 | `distributed/` + `integration/protocols/` legacy features (~35k LOC, feature-gated S378). Removal timeline TBD. |
| **Test coverage** | P2 | ~85% current → 90% target. |
| **VFIO PBDMA runlist** | P1 | PRI ring enumerate wired (`pri_enumerate.rs`), runlist diagnostic JSON-RPC added (`runlist_diagnostic.rs`). **PFIFO_RUNLIST_BASE=0 root cause still open.** |
| **macOS/Windows sandbox** | P3 | Not yet implemented. Linux-only sandbox paths today. |
| **AMD sovereign path** | P2 | Not yet supported; NVIDIA VFIO/sovereign path is primary. |
