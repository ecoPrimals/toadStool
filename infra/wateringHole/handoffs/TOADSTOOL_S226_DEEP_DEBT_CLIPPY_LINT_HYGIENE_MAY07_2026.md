# ToadStool S226 — Deep Debt Audit: Workspace Clippy Clean + Lint Hygiene

**Date**: May 7, 2026
**Author**: toadStool team (automated session S226)
**Priority**: P3 (internal hygiene)
**Status**: RESOLVED

---

## Scope

Comprehensive codebase-wide audit for deep debt, followed by workspace-wide `cargo clippy --workspace -- -D warnings` cleanup to zero errors.

## Survey Findings (No Action Required)

| Category | Finding |
|----------|---------|
| Large files (>800 LOC) | Zero production files exceed threshold |
| Unsafe code | 46 blocks, all at FFI/syscall boundaries (GPU/VFIO/display/mmap/plugin), all `// SAFETY:`-documented |
| Hardcoded primal names | Legacy compatibility / attribution only — no capability-violating coupling |
| Mocks in production | All mocks are `#[cfg(test)]`-gated or intentional stubs |
| TODOs/FIXMEs/HACKs | Zero in production code |
| `ring` dependency | Not in resolved dependency graph (verified `cargo tree -i ring`); `Cargo.lock` entry is stale; `deny.toml` ban still enforced |

## Fixes Applied

### `crates/integration/primals/` (8 clippy errors)

- **`client.rs`**: `unnecessary Debug formatting` → `PathBuf::display()`; `redundant closure` → `serde_json::Value::as_bool`
- **`orchestrator.rs`**: `map_unwrap_or` → `map_or_else` with qualified paths; `redundant closure` → `AsRef::as_ref`; `unused_async` → removed `async` from `register_primal`, `get_primal` (no `.await` calls)
- **`services.rs`**: `unused_async` → removed `async` from `start_service`, `stop_service`

### `crates/auto_config/src/ecosystem/discoverer.rs`

- `items_after_statements` → moved `use` declarations before `if cfg!(test)` block

### `crates/distributed/src/network/metrics.rs`

- Bare `#[allow(dead_code)]` → added `reason = "stored for lifecycle; consumed in test assertions"`

### `deny.toml`

- Updated `ring` ban comment to clarify it is not in the resolved dependency graph

## Verification

- `cargo clippy --workspace -- -D warnings`: **zero warnings**
- `cargo test --workspace`: **22,843 tests, 0 failures**
- `cargo deny check`: clean

## Cross-Primal Notes

None — purely internal hygiene. No IPC surface changes.

## Files Changed

- `crates/integration/primals/src/client.rs`
- `crates/integration/primals/src/orchestrator.rs`
- `crates/integration/primals/src/services.rs`
- `crates/auto_config/src/ecosystem/discoverer.rs`
- `crates/distributed/src/network/metrics.rs`
- `deny.toml`
- `README.md`, `NEXT_STEPS.md`, `DOCUMENTATION.md`
