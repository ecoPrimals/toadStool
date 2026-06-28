# ToadStool Handoff — S321 (Wave 119)

**From**: eastGate primalSpring overwatch
**Date**: Jun 20, 2026
**Covers**: Session S321 — deep debt hygiene pass
**Status**: SHIPPED — all quality gates green

---

## Summary

Single-session deep-debt sprint closing four hygiene gaps: environment literal
centralization, Duration constant deduplication, workspace dependency unification,
and large-file refactoring. Zero functional changes; pure codebase quality evolution.

## Changes

### D-ENV-CENTRALIZE — RESOLVED
Last 3 raw `std::env::var("...")` literals migrated to `socket_env` constants:
- `TOADSTOOL_HEADLESS` — existing constant, wired in `execution.rs`
- `TOADSTOOL_RM_TRIGGER_BIN` — new constant, wired in `rm_trigger.rs`
- `TOADSTOOL_FORENSICS_LOG` — new constant, wired in `forensics.rs`

**Result**: Zero production raw env string literals.

### D-DURATION-DEDUP — RESOLVED
- 4× duplicate `Duration::from_millis(50)` CPU probe unified to shared
  `toadstool_common::constants::timeouts::CPU_USAGE_SAMPLE_WINDOW`
  (evaluator.rs, monitoring/lib.rs, resources/lib.rs, tarpc executor, discovery/core.rs)
- 8 additional inline Duration literals named as module constants:
  `DEFAULT_ESTIMATED_DURATION`, `KMSG_READ_BACKOFF`, `DEVICE_LOST_SETTLE`,
  `FECS_UNHALT_SETTLE`, `FECS_CTXSW_INIT_SETTLE`, `SBR_RESET_SETTLE`,
  `UDEV_POLL_INTERVAL`, `POOL_RETRY_BACKOFF`

### D-DEP-UNIFY — RESOLVED
Workspace-unified 4 dependencies:
- `bytes`: specialty `"1.0"` → workspace `1.11.1`
- `ruzstd`: cylinder + secure_enclave `"0.8"` → workspace
- `serialport`: edge + specialty + display `"4.3"` → workspace
- `ndarray`: akida-reservoir + performance + analytics `"0.16"` → workspace

**Result**: Zero non-workspace version drift.

### D-REAGENT-REFACTOR — RESOLVED
`cylinder/vfio/reagent/mod.rs` (704L, at 750L gate) smart-refactored:
- `mod.rs` (~420L): types, discovery, manifest CRUD, tests
- `capture.rs` (~230L): capture pipeline + helpers
- `mmiotrace.rs` (~92L): mmiotrace distillation

All re-exports preserved; zero external API change.

## Quality Gate Snapshot

| Gate | Status |
|------|--------|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace -- -D warnings` | PASS (0 warnings) |
| `cargo check --workspace` | PASS |
| `cargo test --workspace --lib` (excl. client pre-existing) | PASS |
| Production files >750L | 0 |
| Production raw env literals | 0 |
| Workspace dep version drift | 0 |

## Active Debt (carry forward — unchanged from S315-S320)

1. **D-HW-LEARN-VERIFY** — VRAM/VFIO readback verification (hardware-blocked)
2. **D-EMBEDDED-PROGRAMMER** — real USB/serial transport (hardware-blocked)
3. **D-EMBEDDED-EMULATOR** — decimal-mode 6502, full Z80 (feature-gated)
4. **D-COVERAGE-GAP** — ~85%+ → 90% (integration/GPU/coordination gaps)

## Files Changed (33)

- `Cargo.toml` (workspace deps: ruzstd, serialport, ndarray)
- `crates/core/common/src/constants/timeouts.rs` (CPU_USAGE_SAMPLE_WINDOW)
- `crates/core/common/src/interned_strings/socket_env.rs` (2 new constants)
- `crates/core/cylinder/Cargo.toml` (ruzstd workspace)
- `crates/core/cylinder/src/vfio/reagent/{mod,capture,mmiotrace}.rs` (split)
- `crates/core/cylinder/src/vfio/sovereign_handoff/{forensics,rm_trigger}.rs` (env)
- `crates/core/cylinder/src/vfio/sovereign_handoff/steps/warm_swap_catalyst.rs` (Duration)
- `crates/distributed/src/coordination/discovery/core.rs` (CPU_USAGE_SAMPLE_WINDOW)
- `crates/management/resources/{Cargo.toml,src/lib.rs}` (dep + Duration)
- `crates/neuromorphic/akida-driver/src/setup.rs` (Duration)
- `crates/runtime/{display,edge,gpu,secure_enclave,specialty}/Cargo.toml` (deps)
- `crates/security/{monitoring,policies}/{Cargo.toml,src/*.rs}` (dep + Duration)
- `crates/server/src/{background,pure_jsonrpc,tarpc_server,unibin}/*.rs` (Duration + env)
- Root docs: README, CHANGELOG, CONTEXT, DEBT, DOCUMENTATION, NEXT_STEPS
- `config/capability_registry.toml` (bare "health" method, session stamp)
