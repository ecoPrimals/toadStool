# ToadStool S285 — Deep Debt Evolution VII: Security Migration + Stub Evolution + Capability Naming
**Date**: Jun 3, 2026
**Session**: S285
**Status**: Complete

## Actions Taken

### 1. Server Crypto Migration (top debt item)
Migrated server JSON-RPC encrypt/decrypt off deprecated `distributed::security` → `crypto_integration`:
- `SecurityClient` → `CryptoServiceClient` (with `from_local_socket()`)
- `EncryptionRequest` → `CryptoRequest` (+ `metadata` field)
- `EncryptionOperation` → `CryptoOperation`
- `SecurityLevel::Enhanced` → `SecurityLevel::High`
- All `#[expect(deprecated)]` suppressions removed
- `distributed::security` now has zero production callers outside its own module

### 2. Production Stub/Noop Evolution → Typed Errors
- `NoopCryptoProvider` — all ops return `CryptoError::NoProviderRegistered`
- `StubRuntimeEngine` — execute/metrics return `ExecutionError::NoEngineRegistered`
- `NoopCloudProvider` — already returned `CloudError::ProviderUnavailable` (verified)

### 3. Embedded Placeholder Default Removal
- Removed `embedded-placeholder-impls` from `runtime/specialty` default features
- Added `Unregistered` dispatch variant with `AdapterNotRegistered` errors
- Conditional `allow(dead_code)` on struct fields when feature is off

### 4. Hardcoded Name → Constant
- `health.rs` — `"toadstool"` → `PRIMAL_NAME`
- `os_keyring.rs` — `SERVICE_NAME` → `PRIMAL_NAME`
- `coordination/transport.rs` — exchange → `PRIMAL_NAME`
- `coordination/integration.rs` — exchange → `PRIMAL_NAME`

### 5. Dead Code Removal (~100L)
- `catalyst_watchdog::routine_quench()` + `read_intr_en_safe()` (Exp 233 disabled)
- `module_patch::patch_module()` (superseded by `patch_module_with_rename`)
- `driver_ops::sysfs_read_guarded()` (zero callers)

### 6. Production expect() → Safe Patterns
- `regions.rs` — `expect("4-byte slice")` → `let [a, b, c, d]` pattern match with `MemoryError::OutOfBounds`
- `matrix_support.rs` — `expect` → match with safe fallback

## Metrics
| Metric | Before | After |
|--------|--------|-------|
| Deprecated security callers (production) | 5 files | 0 |
| Silent-success stubs | 3 types | 0 |
| Dead `#[allow(dead_code)]` in prod | 4 sites | 0 |
| Production `expect()` | 3 sites | 0 |
| Workspace clippy warnings | 0 | 0 |
| Test suite | pass | pass |

## Remaining Debt (prioritized)
1. **`distributed::coordination` module** — deprecated but still compiled/exported; large internal migration to `coordination_integration`
2. **`networking` feature off by default** — `ecosystem/communication` uses `ServiceClient::Disabled` + `fallback_response()` in default builds
3. **`LEGACY_*` env fallbacks** — 30 deprecated constants still read in production identity chains
4. **`distributed::security` module cleanup** — now test-only callers; consider moving behind `#[cfg(test)]`
5. **`serialport`/`modbus`/`bollard`** default dependencies — external FFI crates that could be feature-gated or replaced
