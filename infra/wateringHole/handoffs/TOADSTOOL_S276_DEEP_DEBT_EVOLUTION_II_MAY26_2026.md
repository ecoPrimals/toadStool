# toadStool S276 — Deep Debt Evolution II

**Date**: May 26, 2026
**Session**: S276
**From**: toadStool team
**To**: primalSpring (downstream audit)

---

## Summary

Second deep debt evolution pass. Production unwrap/expect/unreachable
surface eliminated. Large file refactored. External mmap dependency
removed. Primal-name aliases deprecated. Discovery capabilities aligned.

## Production Panic Surface — Fully Eliminated

| File | Issue | Fix |
|------|-------|-----|
| `handler/sovereign.rs` | 2x `.unwrap()` on `as_object_mut()` | `if let` guard |
| `mmio_region.rs` | `.expect()` on null pointer | `assert!` + `new_unchecked` |
| `dma.rs` | `.expect()` in `Drop` | Graceful leak-on-error (avoids double-panic) |
| diagnostic interpreter | 5x `.expect("pt_ok guarantees...")` | `if let` tuple destructure |
| diagnostic interpreter | 1x `try_into().expect()` | `unwrap_or` fallback |
| `permissions.rs` | `.expect("len checked")` | `let-else` |
| `dispatch/mod.rs` | `unreachable!()` | `Option` return + error log |

## Large File Refactoring

`handler/sovereign.rs` (1,003L, 11 RPC handlers) → module directory:

| File | Lines | Handlers |
|------|-------|----------|
| `sovereign/init.rs` | 454 | `sovereign_init`, `sovereign_devinit`, `sovereign_classify_tier`, `sovereign_experiment` |
| `sovereign/snapshot.rs` | 250 | `sovereign_snapshot`, `sovereign_compare`, `sovereign_catalyst_diff` |
| `sovereign/capture.rs` | 304 | `sovereign_kernel_health`, `sovereign_reagent_capture`, `sovereign_recipe_replay`, `sovereign_runtime_services_probe` |
| `sovereign/mod.rs` | 15 | Re-exports |

Public API unchanged.

## External Dependency Evolution

`memmap2` crate removed from `hw-safe`. `safe_mmap.rs` rewritten to use
`rustix::mm::mmap/munmap` directly — same pattern as `device_mmap.rs`
(already rustix-based). `ExclusivePtr` adopted for Send+Sync safety.
Zero external mmap dependencies remain in workspace.

## Primal-Name Deprecation

3 stale public type aliases deprecated with `#[deprecated]`:

- `SongbirdNetworkConfigurator` → `OrchestrationNetworkConfigurator`
- `SongbirdNetworkConfig` → `OrchestrationNetworkConfig`
- `NestGateResult` → `StorageServiceResult`

## Discovery Capabilities Aligned

`ipc.register` capability list updated from stale
`["compute.dispatch", "compute.capabilities"]` to full Node Atomic set
via `DISCOVERY_CAPABILITIES` constant (9 capabilities). Aligned with
`primal.announce` handler. Tests updated.

## Upstream Absorption

13 clippy warnings from upstream VFIO reagent/sovereign expansion absorbed
(collapsible_if, derivable_impls, equatable_if_let, single_match_else,
map_unwrap_or, dead_code, too_many_lines).

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,158+ |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 88+ |
| Clippy warnings | 0 |
| Production unwrap/expect | 0 |
| Production unreachable!() | 0 |
| External mmap deps | 0 |
