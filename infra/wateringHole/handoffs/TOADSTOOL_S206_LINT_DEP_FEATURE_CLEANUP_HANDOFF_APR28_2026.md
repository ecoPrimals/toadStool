# ToadStool S206 — Lint Evolution + Dep Hygiene + Feature Cleanup

**Date**: April 28, 2026
**Session**: S206
**Scope**: Codebase-wide lint evolution, dependency unification, stale feature removal, mock default policy

---

## Changes

### 1. Lint Evolution — All `#[allow]` with `reason =`

All ~40 production bare `#[allow(...)]` evolved to `#[allow(..., reason = "...")]`:

- **17 `unsafe_code` modules**: hw-safe (10: device_mmap, volatile_mmio, locked_memory, huge_page, vfio_dma, safe_mmap, vfio_setup, exclusive_ptr, aligned_alloc, contiguous), GPU (4: backend, glowplug, buffer/mod, buffer/threading), display (v4l2/ioctl), hw-learn (nouveau_drm), plugin (ffi_loader)
- **~23 clippy/deprecated/async-fn-in-trait**: auto_config, cli (lib + lifecycle_ops + 3 template modules), client, distributed (lib + discovery/client), integration (primals, protocols, security), management (analytics, monitoring, performance), neuromorphic (akida-driver, reservoir-research lib + readout, cross-substrate-validation lib + main), runtime (adaptive, secure_enclave, wasm), security/policies

### 2. Dependency Unification

Added to `[workspace.dependencies]` and unified across 20+ crate Cargo.toml files:

| Dep | Crates Unified |
|-----|---------------|
| `humantime-serde` | common, toadstool, distributed (3) |
| `rand` | distributed, reservoir-research, testing (3) |
| `tokio-util` | distributed, client, server (3) |
| `temp-env` | 13 crates (all dev-deps) |

### 3. Stale Feature Removal

**GPU crate** (`toadstool-runtime-gpu`):
- Removed features: `spirv`, `jit`, `testing` (none referenced in `cfg` source)
- Removed optional deps: `spirv`, `cranelift-jit`, `wasmtime`

**Testing crate** (`toadstool-testing`):
- Removed features: `integration-tests`, `benchmarks` (none referenced in source)
- Removed optional dep: `wiremock`

### 4. Mock Default Policy

- `test-mocks` removed from `toadstool` core `default` features
- Production builds no longer compile `InMemoryAuthBackend` / `InMemoryAgentBackend`
- `toadstool-testing` explicitly enables via `features = ["test-mocks"]`

---

## Files Changed (60)

- `Cargo.toml` (root): added `humantime-serde`, `temp-env` to workspace deps
- 20+ `crates/*/Cargo.toml`: unified dep versions
- 17 `unsafe_code` module files: `reason =` added
- ~23 clippy/deprecated allow files: `reason =` added
- `DEBT.md`, `NEXT_STEPS.md`, `README.md`: S206 documentation

## Tests

- **7,841 lib-only**, 0 failures, clippy clean (`-D warnings`), fmt clean
- No test count change — all existing tests pass unchanged

## For primalSpring / guideStone

- No wire protocol changes
- No IPC surface changes
- `test-mocks` feature must be explicitly requested if needed (no longer default)
- Workspace dep versions unchanged (only source of version truth moved to root)

---

## Next Evolution

- Coverage push 83.6% → 90% (hardware mocks for V4L2/VFIO)
- `crypto_integration` migration (replace deprecated `SecurityClient` direct calls)
- Primal self-registration with Songbird (`ipc.register`)
- Pipeline encryption (extend dispatch encryption to pipeline submit)
