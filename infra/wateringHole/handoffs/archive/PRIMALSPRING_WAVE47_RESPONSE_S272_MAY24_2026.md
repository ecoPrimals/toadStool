# toadStool — Wave 47 Behavioral Convergence Response

**Date**: May 24, 2026
**Session**: S272
**From**: toadStool
**To**: primalSpring (coordination spring), plasmidBin
**Priority**: MEDIUM — resolved
**License**: AGPL-3.0-or-later

## Summary

Wave 47 identified toadStool as MEDIUM: `health.liveness` returned `{"status":"starting"}`
during boot, which fails the `== "alive"` check in nucleus health sweeps. **RESOLVED**.

## What Changed

### health.liveness — Always Alive

**Before**: `health.liveness` returned `{"status":"starting"}` while the `ready` flag was
`false` (during discovery registration, biomeOS scan, etc.). Nucleus health sweeps failed
during this boot window because they check `jq -r .status == "alive"`.

**After**: `health.liveness` always returns `{"status":"alive"}`. If the caller can reach
the handler, the socket is listening and the process is alive — that's the definition of
liveness. The `ready` parameter was removed from `health_liveness()`.

**Boot-phase signaling**: `health.readiness` continues to return `{"status":"starting"}`
until the server is fully initialized, then switches to `{"status":"ready"}`. This is
the correct endpoint for readiness checks (load balancer drain, dependency ordering).

### Semantic Separation

| Endpoint | Before | After |
|----------|--------|-------|
| `health.liveness` (not ready) | `{"status":"starting"}` | `{"status":"alive"}` |
| `health.liveness` (ready) | `{"status":"alive"}` | `{"status":"alive"}` |
| `health.readiness` (not ready) | `{"status":"starting"}` | `{"status":"starting"}` |
| `health.readiness` (ready) | `{"status":"ready"}` | `{"status":"ready"}` |

This matches the DEPLOYMENT_BEHAVIOR_STANDARD: liveness = process up, readiness = fully initialized.

### Upstream Debt Absorbed

49 clippy errors from upstream rebase:
- 27 in `toadstool-cylinder` (module_patch.rs, sovereign_handoff.rs): dead code, collapsible_if, too_many_arguments, unused vars, format!, unnecessary casts
- 22 in `toadstool-server` dispatch/mod.rs: map_unwrap_or, used_underscore_binding, default_trait_access, collapsible_if, needless_borrow
- 1 in `toadstool-glowplug` (ModuleSource derive regression)

## Files Modified

| File | Change |
|------|--------|
| `crates/server/src/pure_jsonrpc/handler/core/health.rs` | `health_liveness()` always returns `"alive"`, removed `ready` param |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | Updated call site, doc comments |
| `crates/server/src/pure_jsonrpc/handler/core/mod.rs` | Updated tests |
| `crates/server/src/pure_jsonrpc/handler/mod_tests.rs` | Updated integration test |
| `crates/server/src/unibin/mod.rs` | Updated comment |
| `crates/core/cylinder/src/vfio/module_patch.rs` | 10 clippy fixes |
| `crates/core/cylinder/src/vfio/sovereign_handoff.rs` | 17 clippy fixes |
| `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` | 22 clippy fixes |
| `crates/core/glowplug/src/warm_init.rs` | ModuleSource derive fix |

## Metrics

| Metric | Before (S271) | After (S272) |
|--------|---------------|--------------|
| Lib tests | 9,126 | 9,131 |
| Clippy warnings | 0 | 0 |
| health.liveness during boot | `"starting"` (fails sweep) | `"alive"` (passes sweep) |

## Remaining Work

None for Wave 47. toadStool's deployment behavior is aligned with DEPLOYMENT_BEHAVIOR_STANDARD.
