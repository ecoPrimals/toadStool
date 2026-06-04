# toadStool S287 — S286 Consolidation + Telemetry Consumer + Trust Test Coverage

**Date**: June 3, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Cascade**: primalSpring on eastGate → strandGate compute trio
**Status**: Software consolidation complete. Hardware validation PAUSED.

## Mission (from primalSpring cascade)

1. Deep debt / hygiene scan on S286 push (33 files, 842 insertions)
2. Ensure enriched telemetry schema is consumable by barraCuda ml.mlp_train
3. Add cross-gate dispatch.verify_trust test coverage

## S286 Audit Findings (P1)

| Issue | Fix |
|-------|-----|
| `verified` too permissive (LocalTransport counted) | Tightened to BtspVerified or MutuallyAuthenticated only |
| `local_gate_id` used PRIMAL_NAME vs env/hostname | Aligned with `resolve_local_gate_id()` + PRIMAL_NAME fallback |
| `auth.peer_info` missing trust fields | Added `gate_id`, `trust_level`, derived `transport` |
| Ownership never cleared on gate.remove/is_owner:false | Added `revert_to_local_owner()`, wired into gate_update + gate_remove |
| `dispatch.telemetry.schema` missing from DIRECT_METHODS | Added to discovery list |

## Telemetry Consumer (barraCuda ml.mlp_train)

- `DispatchTelemetryRecord::to_feature_vector()` → `[f64; 36]`
- String fields hashed via FNV-1a to `[0, 1)` for numeric stability
- Module-level documentation with dimension table and consumption guide
- Schema discoverable via `dispatch.telemetry.schema` JSON-RPC (public)

## Test Coverage Added

| Area | Tests | Notes |
|------|-------|-------|
| `dispatch.verify_trust` | 6 | anonymous, local_transport, btsp_verified, mutual_auth, with/without gate_id |
| `GateOwnership` lifecycle | 4 | anonymous caller, default owner, revert_to_local, false no-op |
| `GateGpuInfo.is_owner` serde | 1 | Omitted field defaults to false |
| Feature vector | 3 | 36 dims, hash range [0,1), determinism |
| `auth.peer_info` | 2 | Updated existing tests for trust fields |

## Remaining Gaps (acknowledged, not in scope)

1. `pre_dispatch_resource_check` not wired into submit/shader/pipeline/fan_out/sovereign paths
2. `compute.submit` local path still bypasses orchestrator
3. `DispatchTelemetryRecord` not yet emitted from submit paths
4. `RemoteDispatcher::forward()` provenance injection has no test
5. Connection-layer trust hint → CallerContext mapping has no test

## Metrics

- 27 targeted tests pass (trust + telemetry + ownership + auth)
- Full workspace clippy -D warnings clean
- All workspace tests pass (exit 0)
