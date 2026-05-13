# toadStool S253 — Deep Debt Sweep

**Date**: May 13, 2026
**Session**: S253
**Scope**: Legacy environment variables, socket naming, bind address alignment, `#[allow(deprecated)]` cleanup

---

## Summary

Comprehensive deep debt audit and cleanup following Phase C completion. All quality gates green: 8,827 lib tests, 0 clippy warnings, deny clean.

---

## Changes

### 1. CORALREEF_* Environment Variables Deprecated

Five legacy `CORALREEF_*` env vars now have `TOADSTOOL_*` primaries with deprecation warnings:

| Legacy Env Var | New Primary | File |
|---|---|---|
| `CORALREEF_SYSFS_ROOT` | `TOADSTOOL_SYSFS_ROOT` | `cylinder/src/linux_paths.rs` |
| `CORALREEF_PROC_ROOT` | `TOADSTOOL_PROC_ROOT` | `cylinder/src/linux_paths.rs` |
| `CORALREEF_DATA_DIR` | `TOADSTOOL_DATA_DIR` | `cylinder/src/linux_paths.rs` |
| `CORALREEF_DRI_RENDER_PREFIX` | `TOADSTOOL_DRI_RENDER_PREFIX` | `cylinder/src/drm.rs` |
| `CORALREEF_EMBER_SOCKET` | `TOADSTOOL_EMBER_SOCKET` | `cylinder/src/vfio/ember_client.rs` |
| `CORALREEF_EMBER_GATE` | `TOADSTOOL_EMBER_GATE` | `cylinder/src/vfio/ember_gate.rs` |

Each legacy var emits `tracing::warn!` when used as fallback.

### 2. Ember Socket Naming

Default socket path evolved from `coral-ember-{family}.sock` to `toadstool-ember-{family}.sock` in `ember_client.rs`.

### 3. Bind Address Alignment

`DEFAULT_BIND_ADDR` in `config/src/constants.rs` changed from `"0.0.0.0"` to `"127.0.0.1"` — resolves conflict with `BIND_ADDRESS_DEFAULT` and enforces loopback-only default.

### 4. `#[allow(deprecated)]` → `#[expect(deprecated, reason)]`

13 instances across 6 files evolved:

| File | Count |
|---|---|
| `crates/core/config/tests/config_utils_expanded_tests.rs` | 1 |
| `crates/distributed/src/security/tests/discovery.rs` | 6 |
| `crates/cli/tests/discovery_coverage_tests.rs` | 3 |
| `examples/runtime_engines_demo.rs` | 1 |
| `crates/core/toadstool/src/byob/byob_impl/byob_impl_tests/validation_creation.rs` | 2 |

**Zero `#[allow(deprecated)]` remaining in codebase.**

---

## Quality Gates

- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings
- `cargo test --workspace --lib`: 8,827 tests, 0 failures
- `cargo deny check bans`: clean

---

## Downstream Impact

- **hotSpring**: Socket paths shift from `coral-ember-*` to `toadstool-ember-*`. Env var migration is backward-compatible (legacy vars still work, with warnings).
- **coralReef**: No impact (env vars were coralReef-domain but lived in toadStool's absorbed cylinder code).
- **primalSpring**: Audit can mark all deep debt items resolved through S253.
