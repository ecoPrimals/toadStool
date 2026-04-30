# ToadStool S211 — Lint Reason + Dep Unification + Feature Cleanup + hw-safe Expect→Result

**Date**: April 30, 2026
**Session**: S211
**Commit**: `268e82b78`
**Tests**: 7,842 lib-only, 0 failures, clippy clean, fmt clean

---

## Scope

Comprehensive deep-debt pass across lint hygiene, workspace dependency
unification, stale feature flag removal, and production panic elimination.

---

## Changes

### Phase 1: Lint Evolution (`reason =`)

~30 production `#[expect(...)]` attributes across 25 files evolved to include
`reason = "..."`. Two categories:

- **Trailing `//` comments → `reason =`**: Existing explanatory comments after
  the closing `]` were migrated into the structured `reason` parameter
  (e.g. `clippy::option_if_let_else`, `clippy::needless_collect`, `deprecated`
  migration shims).
- **Bare attributes → `reason =`**: Attributes with no explanation received
  appropriate reason text describing why the lint is suppressed.

Files touched: `config/types/network.rs`, `config/runtime_defaults.rs`,
`config/env_overrides/features.rs`, `cli/ecosystem/*`, `cli/templates/*`,
`distributed/security/client/mod.rs`, `distributed/cloud/cost/mod.rs`,
`distributed/primal_capabilities/registry.rs`, `common/capability_provider/provider.rs`,
`common/universal_adapter/discovery_engine/mod.rs`, `common/config_bases/resources_validation.rs`,
`toadstool/encryption/types.rs`, `toadstool/security_hardening/intrusion.rs`,
`toadstool/performance_hardening/memory.rs`, `runtime/adaptive/profiler.rs`,
`runtime/gpu/cpu_resource.rs`, `runtime/gpu/unified_memory/buffer/lifecycle.rs`,
`runtime/orchestration/orchestrator.rs`, `runtime/secure_enclave/isolated_memory.rs`,
`runtime/display/input/device.rs`, `integration/primals/manager.rs`,
`neuromorphic/akida-driver/device.rs`, `neuromorphic/neurobench-runner/metrics.rs`,
`neuromorphic/akida-reservoir-research/readout.rs`.

### Phase 2: Dependency Unification

- `crates/runtime/edge/Cargo.toml`: `tokio`, `serde`, `uuid` converted from
  pinned versions to `{ workspace = true }` (uuid adds `features = ["v5"]`).
- `crates/neuromorphic/akida-driver/Cargo.toml`: `tokio` dev-dep converted
  from `version = "1"` to `{ workspace = true, features = ["test-util"] }`.

### Phase 3: Stale Feature Flag Cleanup

- `crates/cli/Cargo.toml`: Removed `pure-rust = []` (empty, zero `cfg` gates).
- `crates/runtime/specialty/Cargo.toml`: Removed `embedded-hw = []` and
  `industrial = []` (empty, zero `cfg` gates in source).

### Phase 4: hw-safe `expect()` → `Result`

- `crates/core/hw-safe/src/huge_page.rs`: `NonNull::new(...).expect(...)` after
  `mmap_anonymous` replaced with `.ok_or(HugePageError::NullPointer)?`. New
  `NullPointer` variant added to `HugePageError`.
- `crates/core/hw-safe/src/device_mmap.rs`: `NonNull::new(...).expect(...)` after
  `rustix::mm::mmap` replaced with `.ok_or(DeviceMmapError::NullPointer)?`. New
  `NullPointer` variant added to `DeviceMmapError`.

---

## For primalSpring / guideStone

No wire-protocol changes. No new JSON-RPC methods. No behavioral changes.
Internal hygiene only — lint attributes, dep versions, feature flags, error
handling evolution. All existing integration points unchanged.

---

## Remaining Debt (Post-S211)

- `safe_mmap.rs` `as_volatile()` and `as_ptr()` still use `expect()` for
  post-mmap null checks. These return non-Result types; changing them would
  be a public API break. The checks are defense-in-depth assertions on
  invariants guaranteed by `memmap2`.
- `burn-inference` crate is excluded from workspace — cannot use
  `{ workspace = true }` deps. Pinned versions remain.
- Legacy serde aliases in `config/types/network.rs` still embed primal names
  for backward-compatible deserialization.
