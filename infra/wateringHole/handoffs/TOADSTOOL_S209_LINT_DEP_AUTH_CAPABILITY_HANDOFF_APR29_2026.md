# ToadStool S209 — Deep Debt: Lint Reason + Dep Unification + Auth Capability

**Date**: April 29, 2026
**Session**: S209
**Scope**: Codebase-wide lint evolution, dependency hygiene, auth capability evolution

---

## Changes

### 1. Lint Evolution — `reason =` on all remaining attrs

All crate-level `#![allow]` without `reason =` upgraded (7 crates):
- `specialty/embedded/protocol_engine.rs` — `missing_docs`
- `specialty/embedded/chip_database.rs` — `missing_docs`
- `specialty/embedded/cpu6502/mod.rs` — `missing_docs` + cast/names
- `specialty/embedded/cpuz80.rs` — `missing_docs` + cast
- `neuromorphic/akida-models/src/lib.rs` — `clippy::must_use_candidate`
- `runtime/native/src/lib.rs` — `clippy::no_effect_underscore_binding`
- `testing/src/lib.rs` — `refining_impl_trait`

~30 production `#[expect(deprecated)]`/`#[allow(deprecated)]` upgraded with
`reason =` across: config, auth, ecosystem, storage, GPU backends, CLI,
distributed, server/mocks.

### 2. Workspace Dependency Unification (23 Cargo.toml files)

| Dependency | Crates unified |
|-----------|----------------|
| `sha2` | cli, security/policies, runtime/wasm, integration/storage, runtime/edge |
| `serde_json` | integration-tests, runtime/edge, toadstool-core, management/monitoring |
| `tracing-subscriber` | runtime/gpu, integration-tests, runtime/display, neuromorphic/akida-driver |
| `tokio-test` | testing, server, integration/protocols, runtime/display, distributed, client, integration/primals, integration/storage, runtime/specialty, runtime/orchestration |
| `tracing` + `thiserror` | runtime/edge |

All converted from pinned versions to `{ workspace = true }`.

### 3. Stale Feature Flag Cleanup

Removed unused placeholder features from excluded `runtime/python`:
- `ai-ml = []` — no `cfg(feature)` gate anywhere
- `squirrel-preparation = []` — no `cfg(feature)` gate anywhere

### 4. Auth Backend — Capability-Based Issuer Evolution

**Before**: `validate_token()` fallback issuer was `well_known::BEARDOG`
(hardcoded primal name).

**After**: Fallback issuer is `capabilities::CRYPTO` (capability domain).
Auth backend no longer imports `well_known` module. Crate-level
`#![expect(deprecated)]` removed (no deprecated items used in production).

Test fixtures updated: mock token issuers use `capabilities::CRYPTO`,
audience targets use `capabilities::COORDINATION`.

---

## Files Changed (69)

Major categories:
- 7 crate-level `#![allow]` attrs (specialty, neuromorphic, native, testing)
- ~20 production `#[expect(deprecated)]` sites (config, auth, ecosystem, CLI)
- 23 `Cargo.toml` dep unification files
- `auth_backend.rs` + `auth/tests.rs` (capability evolution)
- `DEBT.md`, `NEXT_STEPS.md` (session documentation)
- Edge crate formatting (rustfmt wrapping long attrs)

## Tests

- **7,842 lib-only**, 0 failures, clippy clean (`-D warnings`), fmt clean
- No test count change — all existing tests pass

## For primalSpring / guideStone

- **Auth issuer change**: Token validation now expects `"crypto"` as default
  issuer (was `"beardog"`). If your tokens use `"beardog"` as issuer, either:
  - Set `TOADSTOOL_AUTH_ISSUER=beardog` env var, or
  - Update token issuer to `"crypto"` (capability-based)
- No wire protocol changes
- No IPC surface changes
