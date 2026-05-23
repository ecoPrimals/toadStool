# toadStool — Wave 43 Neural API `primal.announce` Response

**Date**: May 23, 2026
**Session**: S270
**From**: toadStool
**To**: primalSpring (coordination spring), biomeOS (Neural API authority)
**Priority**: HIGH — completed
**License**: AGPL-3.0-or-later

## Summary

All three Wave 43 asks for toadStool are **RESOLVED**:

| Ask | Status |
|-----|--------|
| Wire `primal_announce()` into JSON-RPC dispatch table | **DONE** |
| Startup self-announcement to biomeOS Neural API | **DONE** |
| Remove `#[allow(dead_code)]` from announce function | **DONE** |

## What Changed

### 1. `primal.announce` Dispatch Wiring

- `primal.announce` added to `DIRECT_JSONRPC_METHODS` array (method #88)
- Direct route in `handle_method` → `core::primal_announce()`
- Semantic alias `primal_announce` in `dispatch_by_impl_name`
- Wire L3 cost estimate: `negligible` (pure in-memory, no I/O)
- MethodGate allows unauthenticated access (same as `identity.get`)

### 2. primal_announce() Payload — Wave 43 Schema

Updated per Neural API Wire Standard:

```json
{
  "primal": "toadstool",
  "version": "0.2.0",
  "domain": "compute",
  "capabilities": ["compute", "science", "inference"],
  "methods": ["compute.cancel", "compute.capabilities", "..."],
  "socket": "$XDG_RUNTIME_DIR/biomeos/compute.sock",
  "signal_tiers": ["node"],
  "cost_hints": {
    "compute": 100.0,
    "science": 50.0,
    "inference": 80.0
  },
  "latency_estimates": {
    "compute": 200,
    "science": 100,
    "inference": 150
  },
  "status": "ready"
}
```

### 3. Startup Self-Announcement

- `self_announce_to_biomeos()` added to `toadstool::ipc_helpers`
- Called in `unibin/mod.rs` after `ready.store(true)` + `sd_notify(READY=1)`
- Sends JSON-RPC `primal.announce` to `$XDG_RUNTIME_DIR/biomeos/neural-api-ecoPrimal.sock`
- Fire-and-forget: if biomeOS is unreachable, logs info and continues in standalone mode
- `ipc_surface::ANNOUNCED_METHODS` constant provides the compute.* method list

### 4. Dead Code Removal

- Removed `#[allow(dead_code)]` from `primal_announce` function
- Function is now actively dispatched via both direct and semantic routes

## Files Modified

| File | Change |
|------|--------|
| `crates/server/src/pure_jsonrpc/handler/core/identity.rs` | Updated `primal_announce()` with Wave 43 fields, removed dead_code allow, added test |
| `crates/server/src/pure_jsonrpc/handler/core/mod.rs` | Added `primal.announce` to DIRECT_JSONRPC_METHODS, re-exported `primal_announce` |
| `crates/server/src/pure_jsonrpc/handler/core/wire_l3.rs` | Added `primal.announce` to negligible-cost group |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | Wired `primal.announce` direct route + `primal_announce` semantic alias |
| `crates/server/src/ipc_surface.rs` | New: `ANNOUNCED_METHODS` constant (compute.* namespace) |
| `crates/server/src/lib.rs` | Registered `ipc_surface` module |
| `crates/server/src/unibin/mod.rs` | Added startup self-announcement call |
| `crates/core/toadstool/src/ipc_helpers/connection.rs` | Added `self_announce_to_biomeos()` |
| `crates/core/toadstool/src/ipc_helpers/mod.rs` | Re-exported `self_announce_to_biomeos` |
| `crates/core/toadstool/src/ipc/mod.rs` | Re-exported `self_announce_to_biomeos` |

## Metrics

| Metric | Before (S269) | After (S270) |
|--------|---------------|--------------|
| JSON-RPC methods | 87 | 88 |
| Lib tests | 9,122 | 9,125 |
| Clippy warnings | 0 | 0 |
| Dead code allows | 1 (primal_announce) | 0 |

## Validation

After toadStool starts with biomeOS running:

```bash
echo '{"jsonrpc":"2.0","method":"neural_api.routing_weights","params":{},"id":1}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/neural-api-ecoPrimal.sock
```

Should show toadStool as a provider for `compute.*` calls with affinity weights.

## Remaining Work

None for Wave 43. toadStool's `primal.announce` is fully wired and active.
