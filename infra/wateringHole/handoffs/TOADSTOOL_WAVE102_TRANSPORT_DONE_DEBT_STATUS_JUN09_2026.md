# toadStool Wave 102 Response — Transport DONE, Deep Debt Status

**Date**: June 9, 2026
**From**: toadStool team (strandGate)
**To**: primalSpring (eastGate), cellMembrane (ironGate)
**Sessions**: S301–S306
**Status**: Transport evolution COMPLETE. Scorecard entry is STALE — toadStool shipped S301–S302.

---

## Transport Evolution: DONE (shipped S301–S302, Jun 8)

The Wave 102 scorecard lists toadStool as "the only non-exempt primal without TransportEndpoint adoption." This is incorrect — all 5 requirements were shipped before Wave 102 was written.

| Requirement | Status | Commit | Where |
|---|---|---|---|
| 1. Local `TransportEndpoint` enum (`#[serde(tag = "transport")]`) | **DONE** | `f74f7a4` S301 | `common/transport_endpoint.rs` — `Uds`, `Tcp`, `MeshRelay` |
| 2. `connect_transport()` locally (~30 lines) | **DONE** | `f74f7a4` S301 | `ipc/platform/mod.rs` + `ipc/client.rs` |
| 3. Accept `TRANSPORT_ENDPOINT` env var at startup | **DONE** | S301–S302 | `unibin/execution.rs`, `daemon/server.rs`, `byob_server.rs` |
| 4. `--port` as Tier 5 fallback | **DONE** | S301 | Preserved in all 3 server paths |
| 5. No `sourdough-core` import | **DONE** | — | Zero cross-primal deps |

**Wire compatibility**: Our `TransportEndpoint` uses identical `#[serde(tag = "transport", rename_all = "snake_case")]` — wire-compatible with sourDough canonical format. Ready for `sourdough validate transport`.

**Recommendation**: Update scorecard to **DONE (LOCAL pattern)** for toadStool. 10/11 non-exempt primals now compliant.

---

## Deep Debt Sprint: S300–S305 Summary

| Session | Focus | Key Deliverables |
|---|---|---|
| S300 | Deep Debt XI | Migration stubs → `NotImplemented`, `/tmp/` elimination, dep feature narrowing |
| S301 | Transport Phase 1 | `TransportEndpoint` type, `TRANSPORT_ENDPOINT` env injection in server + daemon |
| S302 | Transport Phase 2 | BYOB transport injection, `IpcClient::from_transport_endpoint`, `0.0.0.0` → `127.0.0.1` |
| S303 | Deep Debt XII | `#[allow]` → `#[expect]` elimination, `page_tables.rs` split (V2/Kepler), OS identifier constants |
| S304 | Deprecated Elim | 15 Category A deprecated items removed (-300L dead code) |
| S305 | Deprecated Evol | Sync ctor migration (auth/agents → async), `OpenCl` `#[deprecated]` attr, remaining `#[allow]` → `#[expect]` |
| S306 | Deep Debt XIII | `bar_cartography.rs` split (4 files), `amd/ioctl.rs` split (3 files), `ServiceMeshType` vendor variant removal |

---

## Remaining Work — Upstream Alignment Needed

### P2: Depot Refresh (cellMembrane action)

toadStool binary is **6 sessions ahead** of last depot build. Transport evolution, deprecated removals, and deep debt work are all in git but not in depot.

**Action**: Include toadStool in the depot refresh sweep. Binary should be rebuilt from current HEAD (`276aa00aa`+).

### P2: `sourdough validate transport` (sourDough action)

Once depot is refreshed, run `sourdough validate transport` against toadStool to formally verify wire compliance. We expect PASS — our `TransportEndpoint` is wire-identical to the canonical format.

### P3: Outbound `TcpStream::connect` Migration (~16 call sites)

16 remaining direct `TcpStream::connect` calls in GPU dispatch, orchestration IPC, and health probes. These should migrate to `connect_transport()` as peers adopt `TransportEndpoint`. Not blocking — peers need to publish their transport endpoints first.

**Dependency**: songBird `ipc.resolve` returning structured `TransportEndpoint` JSON (Phase 2 M1 per Wave 102).

### P3: Coverage (~85% → 90%)

+174 tests since S291 (now 9,069+ lib-only). Remaining gap is hardware-dependent paths (VFIO, DRM, GPU probing) that can't run in CI without real hardware.

### P3: `distributed::security` → `crypto_integration` Migration

The deprecated `security` module has ~50 production+test callers. `crypto_integration` is the replacement but needs callers migrated. This is the largest remaining deprecated-symbol cleanup.

**Dependencies**: None — this is internal refactoring.

### P3: CallerContext Full Threading

`identity` and `envelope` fields not yet populated from ionic tokens. Requires biomeOS auth token format spec.

### P4: LEGACY Env Staged Removal (23 reads)

23 `LEGACY_*` env reads now emit deprecation tracing. Safe to remove after one deployment cycle confirms no consumers. Deployment coordination needed.

### P4: `tarpc` Default Feature Removal

`tarpc` is still in server default features. Deep coupling — dedicated session needed.

---

## Quality Gates (all green)

| Gate | Status |
|---|---|
| `cargo clippy -D warnings` | Zero warnings (maintained S292–S305) |
| `cargo test --workspace --lib` | 9,069+ pass, 0 fail |
| Zero production `#[allow]` | Achieved (all → `#[expect]` with reasons) |
| Zero production panics/unwraps | Achieved |
| Zero `/tmp/` hardcoding | Achieved S300 |
| `TRANSPORT_ENDPOINT` accepted | Achieved S301 |
| BYOB default bind `127.0.0.1` | Achieved S302 |
| Zero production files >750L | Achieved S303, maintained S306 (2 more splits) |
| `capability_registry.toml` | Shipped S291 |

---

**toadStool is transport-compliant, VPS-ready, and debt-clean. The scorecard gap is a stale read — update to DONE.**
