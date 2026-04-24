# ToadStool S177 — Deep Debt Evolution Handoff

**Date**: April 2026
**Session**: S177
**Scope**: Production stub evolution, auth issuer overstep fix, dependency hygiene, stale feature cleanup, deprecated code removal, deny.toml tightening

---

## Changes

### Phase 1 — Production Stub Evolution
- `StubRuntimeEngine` error evolved from `not_found` to `configuration` with capability guidance message (`compute.engine.register`), matching `NoopCryptoProvider` / `NoopCloudProvider` pattern.
- `NoopCryptoProvider` already correct — no changes needed.
- OpenCL stub removal deferred to Phase 5.

### Phase 2 — Auth Issuer Overstep Fix
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs`: JWT issuer validation now reads from `TOADSTOOL_AUTH_ISSUER` env var (added to `socket_env.rs`), falling back to `well_known::BEARDOG` for backward compatibility.
- Removes hard wire to another primal's identity; issuer is now capability-based.

### Phase 3 — Dependency Hygiene
- `base64 = "0.22"` added to `[workspace.dependencies]` in root `Cargo.toml`.
- 7 member crates switched from direct `base64 = "0.22"` to `base64 = { workspace = true }`.
- `anyhow` confirmed dev-only in `cli` and `distributed` — already in `[dev-dependencies]`, no action needed.
- `runtime/edge` remains excluded from workspace (pre-existing issues); alignment deferred.

### Phase 4 — Stale Feature Flag Cleanup
~20 unused feature flags removed across 10 crates:
- `crates/server`: `api`
- `crates/integration/protocols`: `networking`
- `crates/runtime/adaptive`: `telemetry`, `testing`
- `crates/runtime/secure_enclave`: `gpu-compute`, `hardware-enclave`
- `crates/runtime/display`: `rustix-backend`
- `crates/runtime/gpu`: `detect-nvidia`, `detect-amd`, `detect-intel`, `detect-apple`, `cuda`, `opencl`
- `crates/integration/primals`: `security`, `squirrel`, `storage`, `full`
- `crates/integration/security`: `testing`, `mock-beardog`
- `crates/auto_config`: `gpu`, `ml-optimization`, `enterprise-features`
- `crates/runtime/wasm`: `unsafe-fast-cache`

### Phase 5 — Deprecated Code Removal
- **OpenCL stubs removed** (deprecated since S198):
  - `crates/runtime/gpu/src/backends/opencl_impl/mod.rs` — deleted
  - `crates/runtime/gpu/src/unified_memory/backends/opencl.rs` — deleted
  - `crates/runtime/universal/src/backends/opencl.rs` — deleted
  - `examples/opencl_gpu_demo.rs` — deleted
  - Parent modules updated (mod declarations, re-exports, ComputeUnitDispatch::OpenCl variant removed)
  - `toadstool-runtime-universal` Cargo.toml: removed optional `toadstool-runtime-gpu` dep and `opencl` feature
- **Deprecated capability discovery types** removed:
  - `DiscoveryMethod::Kubernetes` and `DiscoveryMethod::Consul` variants removed (deprecated since 0.16.0, zero non-test callers)

### Phase 6 — deny.toml Tightening
- Removed dead `webpki` `[[licenses.clarify]]` (webpki absent from Cargo.lock).
- `ring` clarification retained (ring still transitive in lockfile).
- `RUSTSEC-2024-0436` (paste) ignore retained — still transitive via wgpu-hal→metal and statrs→nalgebra→simba.
- `reqwest` policy documented inline (allowed only behind optional `http-downloads` feature on edge crate).

---

## Verification

- `cargo check --workspace --exclude toadstool-runtime-edge` — clean
- `cargo clippy --workspace --exclude toadstool-runtime-edge -- -D warnings` — 0 warnings
- `cargo test --workspace --exclude toadstool-runtime-edge --lib` — 7,789 passed, 0 failed
- `cargo fmt --all --check` — 0 diffs
- `cargo deny check advisories` — ok
- `cargo deny check licenses` — ok

---

## Files Modified

### Root
- `Cargo.toml` — added `base64 = "0.22"` to workspace deps
- `deny.toml` — removed webpki clarify, updated paste ignore comment, documented reqwest policy

### Core
- `crates/core/toadstool/src/execution/stub_runtime_engine.rs` — error evolved to `configuration`
- `crates/core/toadstool/src/biomeos_integration/auth_backend.rs` — configurable JWT issuer
- `crates/core/common/src/interned_strings/socket_env.rs` — added `TOADSTOOL_AUTH_ISSUER`
- `crates/core/common/src/capability_discovery/types.rs` — removed deprecated variants

### Cargo.toml Updates (base64 workspace + feature cleanup)
- `crates/core/toadstool/Cargo.toml`, `crates/core/common/Cargo.toml`, `crates/core/config/Cargo.toml`
- `crates/server/Cargo.toml`, `crates/cli/Cargo.toml`
- `crates/runtime/secure_enclave/Cargo.toml`, `crates/integration/storage/Cargo.toml`
- `crates/runtime/adaptive/Cargo.toml`, `crates/runtime/display/Cargo.toml`
- `crates/runtime/gpu/Cargo.toml`, `crates/runtime/wasm/Cargo.toml`
- `crates/runtime/universal/Cargo.toml`
- `crates/integration/protocols/Cargo.toml`, `crates/integration/primals/Cargo.toml`
- `crates/integration/security/Cargo.toml`, `crates/auto_config/Cargo.toml`
- `examples/Cargo.toml`

### Files Deleted
- `crates/runtime/gpu/src/backends/opencl_impl/mod.rs`
- `crates/runtime/gpu/src/unified_memory/backends/opencl.rs`
- `crates/runtime/universal/src/backends/opencl.rs`
- `examples/opencl_gpu_demo.rs`
