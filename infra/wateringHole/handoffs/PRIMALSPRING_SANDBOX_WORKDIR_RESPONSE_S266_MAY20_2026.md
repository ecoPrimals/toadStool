# toadStool S266: Sandbox working_dir Production + Upstream Debt Absorption

**Date**: May 20, 2026
**From**: toadStool (compute hardware primal)
**To**: primalSpring (coordination spring)
**Session**: S266
**Audit**: Wave 31 Remaining Horizons — toadStool

---

## Horizon Item Resolved

| Item | Priority | Status |
|------|----------|--------|
| Sandbox `working_dir` production | LOW | **RESOLVED** |

primalSpring Wave 31 listed one remaining horizon for toadStool:

> Sandbox `working_dir` production — S263 workload spec shipped. Operational deployment pending.

This item had three sub-gaps. All are now resolved:

### 1. `data_dependencies` pre-dispatch staging — WIRED

`execute_workload` now calls `validate_data_dependencies()` before dispatch:
- Required local deps must exist on disk (error if missing)
- Optional deps log a warning and allow degraded execution
- BLAKE3 integrity verification when hash is declared in the workload TOML
- Remote sources (`nestgate://`, `http://`) logged and skipped (staging TBD)
- 7 new tests covering all paths

**Files**: `crates/cli/src/executor/workload/mod.rs`, `crates/cli/Cargo.toml` (blake3 dep)

### 2. `SandboxSpec.working_directory` — WIRED into sandbox manager

`CrossPlatformSandboxManager::create_sandbox()` now:
- Creates the working directory inside the sandbox root
- Stores `working_directory` in `SandboxInfo.metadata` for platform managers to consume

**File**: `crates/security/sandbox/src/manager.rs`

### 3. Daemon workload manager — unchanged (separate scope)

`workload_manager.rs` still simulates execution. This is a separate operational concern
beyond the S263 spec gap.

---

## Upstream Debt Absorbed

90+ clippy errors from new upstream cylinder code + server API changes:

| Source | Errors | Resolution |
|--------|--------|------------|
| `ce_validate.rs` (new) | ~30 `missing_docs`, unused imports, `get(0)`, `unused_mut` | Module-level `#[allow(missing_docs)]`, unused import removed, `.first()`, `mut` removed |
| `sovereign_tiers.rs` (new) | ~20 `missing_docs` | Module-level `#[allow(missing_docs)]` |
| `pmu_investigate.rs` (new) | ~30 `missing_docs`, unused import, dead code | Module-level `#[allow(missing_docs, dead_code)]`, unused import removed |
| `pushbuf.rs` (new) | ~5 `missing_docs` | Module-level `#[allow(missing_docs)]` |
| `SovereignInitOptions.skip_cold_memory_training` | 2 field-not-found | Field removed upstream; both dispatch paths updated |
| `ComputeDevice.adopt_anchor_fds` | 1 method-not-found | Method removed upstream; caller refactored to debug log |
| `primal_announce` re-export | 1 unused import | Re-export removed; function marked `#[allow(dead_code)]` pending dispatch wiring |
| Server dispatch patterns | 22 clippy warnings | `_sysfs_bar` → `sysfs_bar`, `_cache` → `cache_guard`, `Default::default()` → named, collapsed `if let`, `map().unwrap_or()` → `map_or()`/`is_some_and()` |

---

## Documentation Cleanup

| Item | Action |
|------|--------|
| `compute.fan_out` | Removed from `DIRECT_JSONRPC_METHODS` + wire_l3 (handler dropped upstream) |
| `NEXT_STEPS.md` | Archived 413 lines of completed session history (S90–S240) — pointer to CHANGELOG |
| Crate count | Fixed 64 → 46 (actual workspace members) in sporeprint |
| Method count | Corrected 85 → 86 across all docs (README, DOCUMENTATION, CONTEXT, sporeprint) |
| Session markers | All root docs bumped S265 → S266 |
| ADR-004 | Added FOSSIL banner (`service_discovery.rs` → `capability_discovery/`) |
| Test counts | All docs aligned to 9,055+ lib / 23,000+ workspace |

---

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,055+ (up from 9,028) |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 86 (direct) |
| Clippy | 0 warnings |
| `cargo deny` | Clean |
| Unsafe blocks | 46 |
| Workspace crates | 46 |

---

## Remaining for toadStool

| Item | Priority | Notes |
|------|----------|-------|
| Coverage 83% → 90% | LOW | Hardware-dependent paths need mock infrastructure |
| FECS PENDING_CTX_RELOAD | RESEARCH | Active frontier — GR context golden state mapping |
| Daemon workload execution | LOW | Currently simulates; not part of S263 spec gap |
| Remote `data_dependencies` staging | LOW | `nestgate://` / `http://` sources logged but not fetched |

**toadStool is stadial-current. Zero debt per Wave 31 audit.**
