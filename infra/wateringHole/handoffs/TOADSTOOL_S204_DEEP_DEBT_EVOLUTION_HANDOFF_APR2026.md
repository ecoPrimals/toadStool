# ToadStool S204 — Deep Debt Evolution Handoff

**Date**: April 26, 2026
**Session**: S204
**Scope**: Safety documentation, hardcoded→constant evolution, dependency hygiene, mock isolation, lint modernization, deny.toml cleanup
**Commit**: `S204: deep debt evolution — safety docs, hardcoded→constants, dep hygiene, mock isolation, lint reason, deny cleanup`

---

## Changes

### Phase 1: Unsafe Safety Documentation
- Added `// SAFETY:` comments to all 13 unsafe blocks in `crates/core/toadstool/src/plugin_system/ffi_loader.rs`
- This was the **last file** in the codebase without SAFETY comments
- All 49 unsafe blocks across 16 files are now fully documented

### Phase 2: Hardcoded → Capability-Based
- `INSTANCE_ID` constant added to `constants/primal_identity.rs` (value: `"toadstool-main"`)
- `universal/provider.rs:instance_id()` now returns `INSTANCE_ID` instead of hardcoded string
- `display/ipc/dispatch.rs:display.get_capabilities` now uses `PRIMAL_NAME` instead of `"toadstool-primary"`
- `discovery_engine/mod.rs` mDNS browse now uses `TOADSTOOL_SERVICE_TYPE` constant instead of duplicate string literal

### Phase 3: Dependency Hygiene
- `serde_yaml_ng` unified to `{ workspace = true }` in: cli, integration/primals, testing, management/performance, security/policies
- Removed unused `humantime-serde` from `crates/cli/Cargo.toml`
- `rustix` aligned from `1.0` → `1.1` in `crates/runtime/secure_enclave/Cargo.toml`
- Stale WASM/zstd comment corrected (wasmi is pure Rust, not zstd)

### Phase 4: Mock Isolation
- `InMemoryAgentBackend`, `AgentBackendDispatch::InMemory`, and `AgentDeploymentManager::with_inmemory` gated behind `#[cfg(any(test, feature = "test-mocks"))]`
- Matches existing pattern for `AuthenticationManager::with_inmemory` and `StorageProvisioningManager::with_inmemory`

### Phase 5: Lint Evolution
- Bare `#[allow(...)]` → `#[allow(..., reason = "...")]` in 9 crate `lib.rs` files:
  - `core/toadstool`, `server`, `runtime/gpu`, `runtime/universal`, `distributed`, `runtime/container`, `runtime/orchestration`, `client`
- Plus `byob/resource_metrics.rs` struct-level allow

### Phase 6: deny.toml Cleanup
- Removed stale `BSD-3-Clause-Clear` license allow (no tfhe/FHE crates in dependency tree)
- Activated `zstd-sys` ban (was commented out; narrative said banned but line was `#{ name = "zstd-sys" }`)
- Documented `ring` clarify entry as defensive-only (ring is banned and absent from lockfile)

---

## Audit Findings (Not in Scope)

Five parallel audits found:
- **No production files over 800 lines** (all 9 files >800L are test suites or examples)
- **Zero production unwraps**
- **Zero production TODOs/FIXMEs/HACKs**
- **Zero stale feature flags**
- **Zero unused cfg(feature) references**
- All 47→49 unsafe blocks are necessary FFI/ioctl/mmap/allocator/MMIO (cannot be made safe)
- CLI `println!`/`eprintln!` are intentional user-facing output
- Legacy primal name serde aliases are backward-compatibility shims
- Embedded placeholders (D-EMBEDDED-*) require hardware to resolve

---

## Verification

```bash
cargo check --workspace          # 0 errors
cargo clippy --workspace         # 0 warnings
cargo test -p toadstool -p toadstool-common -p toadstool-display --lib  # 215 passed, 0 failed
cargo test --workspace --lib     # 7,832 passed, 0 failed
cargo fmt --all --check          # 0 diffs
```

---

## Remaining Active Debt (from DEBT.md)

1. **D-HW-LEARN-VERIFY** — nouveau DRM UAPI register query without BAR mmap
2. **D-EMBEDDED-PROGRAMMER** — real hardware adapters, MISO modeling
3. **D-EMBEDDED-EMULATOR** — fuller 6502/Z80, peripherals, GDB
4. **D-COVERAGE-GAP** — ~83.6% line coverage vs 90% target

---

## Files Changed (30)

- `DEBT.md`, `DOCUMENTATION.md`, `NEXT_STEPS.md`, `README.md`, `CONTEXT.md`, `CHANGELOG.md`
- `crates/cli/Cargo.toml`, `crates/integration/primals/Cargo.toml`, `crates/testing/Cargo.toml`, `crates/management/performance/Cargo.toml`, `crates/security/policies/Cargo.toml`, `crates/runtime/secure_enclave/Cargo.toml`
- `crates/core/common/src/constants/{mod,primal_identity}.rs`, `crates/core/common/src/universal_adapter/discovery_engine/mod.rs`
- `crates/core/toadstool/src/{lib,plugin_system/ffi_loader,universal/provider,biomeos_integration/{mod,agent_backend/mod,agents/manager},byob/resource_metrics}.rs`
- `crates/{server,client,distributed,runtime/{gpu,universal,container,orchestration}}/src/lib.rs`
- `crates/runtime/display/src/ipc/dispatch.rs`
- `deny.toml`, `.cleanignore`, `docs/README.md`
