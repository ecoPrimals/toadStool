# ToadStool S228 — primalSpring Audit Response (May 7, 2026)

**Date**: May 7, 2026
**Author**: toadStool team (automated session S228)
**Priority**: Response to primalSpring downstream audit
**Status**: RESOLVED — no open gaps for toadStool

---

## toadStool Gap Item

### Short timeout sensitivity (Low) — ALREADY RESOLVED (S225, PG-62)

**Audit ask**: Document minimum recommended timeout for `compute.dispatch.submit` (especially GPU workloads). Consider a `health.liveness` fast-path.

**Status**: Fully addressed in S225 (PG-62). Both asks were implemented:

1. **`health.liveness` fast-path** — Returns `{"status":"starting"}` immediately during initialization (socket accepts connections before executor is ready), transitions to `{"status":"alive"}` once fully initialized. Uses `Arc<AtomicBool>` readiness flag. Zero compute pipeline involvement during startup probes.

2. **Timeout documentation** — README now documents:
   - Health probe timeout: **≥3 seconds** (recommended 5s for composition startup)
   - `DISPATCH_DEFAULT_TIMEOUT`: 5,000 ms (overridable via `timeout_ms` request param)
   - `WORKLOAD_EXECUTION_TIMEOUT`: 300s (overridable via `TOADSTOOL_EXECUTION_TIMEOUT` env)
   - `TCP_IDLE_TIMEOUT`: 300s (overridable via `TOADSTOOL_TCP_IDLE_TIMEOUT_SECS` env)
   - GPU workload callers advised to set `timeout_ms` proportional to expected computation

**Relevant handoff**: `TOADSTOOL_S225_PG62_HEALTH_LIVENESS_FASTPATH_MAY07_2026.md`

---

## Cross-Primal Notes (not toadStool's responsibility, but tracked for awareness)

### petalTongue (PT-1 through PT-5)
Static file serving gaps. No toadStool dependency or action needed.

### NestGate (NG-1 through NG-4)
Content-addressed storage gaps. No toadStool dependency. toadStool's `compute.dispatch.submit` may eventually consume NestGate-stored payloads, but that's a future integration path.

### biomeOS (RP-1 through RP-5)
RootPulse graph executor gaps. toadStool is not referenced in the rootpulse commit graph. No action needed.

### barraCuda — shader absorption
29 shader candidates from neuralSpring. Once absorbed, springs rewire to `shader.compile.wgsl` / `compute.dispatch.submit` IPC calls. toadStool already routes both methods:
- `compute.dispatch.submit` → `dispatch_submit()` handler
- `shader.dispatch` → `shader_dispatch_submit()` handler

No toadStool code changes needed for the absorption — barraCuda and coralReef own the shader domain.

### barraCuda — `stats.entropy`
Referenced in `tictactoe_cell.toml`. barraCuda's responsibility. No toadStool involvement.

### rhizoCrypt — silent timeout
UDS connection-level health issue. Similar to our pre-PG-62 state but at the connection level (not method level). No toadStool action needed.

### BearDog / LoamSpine — RP-1/RP-5
`crypto.sign` param naming and entry signing lifecycle. No toadStool involvement.

---

## Files Changed (S228)

- `README.md` — Added "Dispatch Timeouts" section with `DISPATCH_DEFAULT_TIMEOUT`, `WORKLOAD_EXECUTION_TIMEOUT`, `TCP_IDLE_TIMEOUT` constants and override guidance
- `infra/wateringHole/handoffs/TOADSTOOL_S228_PRIMALSPRING_AUDIT_RESPONSE_MAY07_2026.md` — This file

## Verification

- toadStool has **zero open gaps** from this audit
- All timeout constants are centralized in `crates/core/common/src/constants/timeouts.rs`
- All timeout overrides documented in README
- PG-62 fast-path verified with 5 dedicated tests (S225)
