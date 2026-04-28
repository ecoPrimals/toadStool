# ToadStool S208 — Deep Debt: Unsafe Allow + Feature Hygiene + Expect→Result

**Date**: April 28, 2026
**Session**: S208
**Scope**: Codebase-wide deep debt audit and evolution

---

## Audit Findings (no remaining actionable items)

- **No production files >800 lines**
- **No `std::mem::transmute`** usage anywhere
- **No `libc` or `nix` calls** — all migrated to `rustix`
- **No ungated mocks** in production code
- **No production `TODO`/`FIXME`/`HACK`**
- **All 49 unsafe blocks** have `// SAFETY` documentation
- **Zero `Box<dyn Error>`** in production
- **Zero `.clone().clone()`** patterns
- **No stale `-sys` crate** dependencies in core path

## Changes

### 1. Removed unnecessary `#[allow(unsafe_code)]`

`crates/runtime/gpu/src/glowplug/mod.rs` — module contains zero unsafe
code. The `#![allow(unsafe_code, reason = "...")]` was preemptive/redundant.

### 2. CLI feature flag cleanup

Removed 4 empty no-op features from `crates/cli/Cargo.toml`:
- `ecosystem`, `universal`, `monitoring`, `templates` — declared but never
  used as `#[cfg(feature)]` gates in production `src/`
- `full` simplified: `["daemon", "wasm", "nautilus"]`
- `pure-rust` simplified: `[]` (was `["ecosystem", "universal", "monitoring"]`)
- `gpu-ai` comment corrected: "WebGPU + AI/ML extensions" (was stale "CUDA for Python AI")
- 5 test modules ungated (they tested always-compiled code)

### 3. Production `expect()` → idiomatic alternatives

| Location | Before | After |
|----------|--------|-------|
| `InputManager::subscribe_events` | `expect("...")` (panic) | `Result<Receiver>` via `ok_or_else` |
| `ProtocolEngine::build_avr_*` | `.expect("just set")` | `Option::insert()` (returns `&T`) |
| `ProtocolEngine::build_pic_connect` | `.expect("set")` | `Option::insert()` |
| Transport handshake | `hdr[4..8].try_into().expect(...)` | `[hdr[4], hdr[5], hdr[6], hdr[7]]` |

### 4. Edge discovery port constants

Hardcoded port literals (22, 80, 8080) extracted to `well_known_ports`
module constants in `crates/runtime/edge/src/discovery/network.rs`.

---

## Files Changed (10)

| File | Change |
|------|--------|
| `crates/runtime/gpu/src/glowplug/mod.rs` | Remove `#![allow(unsafe_code)]` |
| `crates/cli/Cargo.toml` | Remove 4 empty features, fix stale comments |
| `crates/cli/tests/cli_coverage_s155_tests.rs` | Ungate 2 test modules |
| `crates/cli/tests/cli_coverage_s155b_tests.rs` | Ungate 3 test modules |
| `crates/runtime/display/src/input/mod.rs` | `subscribe_events` panic → Result |
| `crates/runtime/specialty/src/embedded/protocol_engine.rs` | `expect` → `Option::insert` |
| `crates/integration/protocols/src/transport.rs` | `expect` → array indexing |
| `crates/runtime/edge/src/discovery/network.rs` | Port literals → constants |
| `DEBT.md` | S208 paragraph |
| `NEXT_STEPS.md` | S208 header |

## Tests

- **7,842 lib-only**, 0 failures, clippy clean (`-D warnings`), fmt clean
- No test count change — all existing tests pass unchanged

## For primalSpring / guideStone

- No wire protocol changes
- No IPC surface changes
- `InputManager::subscribe_events` signature changed: now returns `Result`
  (callers must handle the error case)
