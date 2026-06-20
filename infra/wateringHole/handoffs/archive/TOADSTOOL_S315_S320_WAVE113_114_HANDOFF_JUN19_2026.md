# ToadStool Handoff — S315–S320+ (Wave 113–114)

**From**: eastGate primalSpring overwatch
**Date**: Jun 19, 2026
**Covers**: Sessions S315–S320 + eastGate deep-debt evolution pass
**Status**: SHIPPED — all quality gates green

---

## Summary

Ten sessions of evolution bringing toadStool to full primalSpring genetics compliance.
Wave 113 REJECT, Wave 114 MitoBeacon acceptance, gRPC/OpenCL purge, and a comprehensive
deep-debt pass achieving 100% SPDX compliance, zero-copy dispatch, and zero files >800L.

---

## Session Highlights

### S315 — Wave 113: bare `"health"` + riboCipher REJECT (Jun 14)
- Bare `"health"` method returns `{status, primal, version}` (112 total methods)
- riboCipher REJECT: unsignalled connections get `-32600` instead of legacy WARN fallback

### S316–S318 — Hygiene (Jun 15)
- `cpu_resource.rs`, `glowplug_client.rs`, `handler/router.rs` splits (750L gate)
- `PRIMAL_SOCKET` env var deleted; `TOADSTOOL_ENABLE_GRPC` deleted
- 6 deprecated sync constructors removed; SecurityClient/SocketStorage async-migrated
- Last production `#[allow]` removed; 7 unfulfilled `#[expect]` fixed

### S319 — gRPC + OpenCL DELETED (Jun 15)
- `CoordinationTransport::GRPC` enum variant, config, health checks, stubs — ALL removed
- `GpuFramework::OpenCl`, detection, compiler guards — ALL removed
- ~60 files touched, −458 lines net

### S320 — Wave 114: MitoBeacon `0xED` Acceptance (Jun 16)
- `0xED` mito-beacon accepted on all accept loops (Unix, TCP, BTSP, early-health)
- HMAC tag read + logged; validation deferred to Wave 115 HKDF
- Shared dispatch with `0xEC` CLEAR
- Nuclear `0xEE` still rejects (Wave 115 tiered access)

### S320+ — eastGate Deep-Debt Evolution Pass (Jun 19)
- **SPDX**: 100% compliance (2,764/2,764 files); 3 wrong license fixed, 3 missing added, 30 duplicates cleaned
- **Zero-copy dispatch**: `Arc<EncryptionKey>` cache (pointer bump vs key clone), `binary_size` telemetry (freed payload lifetime), pipeline first-stage borrow, error consolidation
- **warm_swap.rs** smart refactor: 818L → 479L + 305L catalyst helpers
- **Hardcoding eliminated**: `coordination/discovery/client.rs` cross-primal names → `LEGACY_*_PASCAL` constants
- **Unused C deps removed**: `cc`/`bindgen` from edge + specialty Cargo.toml
- **Dead `PRIMAL_SOCKET` test references**: all evolved to `BIOMEOS_SOCKET_DIR`
- **`hw_learn/helpers.rs`**: hardcoded `"toadstool"` → `PRIMAL_NAME` constant

---

## Quality Gate Snapshot

| Gate | Status |
|------|--------|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace -D warnings` | PASS (0 warnings) |
| `cargo check --workspace` | PASS |
| `cargo test --workspace --exclude toadstool-client` | PASS |
| `cargo doc` | PASS |
| SPDX compliance | 100% (2,764/2,764 files) |
| Production files >800L | 0 |
| Production `#[allow(` | 0 |
| Production `.unwrap()`/`.expect()` | 0 |
| Production `todo!()`/`FIXME`/`HACK` | 0 |
| Hardcoded cross-primal names | 0 (legacy wire compat via constants) |
| Production mocks | 0 (all feature-gated behind `test-mocks`) |

---

## Capability Registry

- **17 capability groups**, **112 JSON-RPC methods**
- Transport: `["uds", "tcp"]`
- Sockets: `compute.sock`, `compute-tarpc.sock`
- riboCipher: CLEAR (`0xEC`) + MitoBeacon (`0xED`) accepted; Nuclear (`0xEE`) rejects
- Health: bare `"health"` + triad (`liveness`/`readiness`/`check`)
- Auto-register: PCI GPU/NPU inventory in `primal.announce` (S309)

---

## Active Debt (4 items)

| ID | Status | Scope |
|----|--------|-------|
| D-HW-LEARN-VERIFY | Active (evolved) | nouveau DRM UAPI register query |
| D-EMBEDDED-PROGRAMMER | Active | USB/serial/parallel transport stubs |
| D-EMBEDDED-EMULATOR | Active | 6502 decimal mode, Z80 prefix tables |
| D-COVERAGE-GAP | Active | ~85%+ line coverage (target 90%) |

---

## Upstream Gaps for Primal Teams

### For upstream overwatch audit
- **Wave 115**: Nuclear tier `0xEE` + HKDF HMAC validation — blocked on cross-gate key distribution
- **PBDMA runlist**: `PFIFO_RUNLIST_BASE=0` RCA still active — blocks Phase D mixed command streams
- **Multi-primal E2E**: `integration-tests/tests/pending/e2e_composition_workflow.rs` quarantined
- **Live-cluster chaos**: 6 tests `#[ignore]` in `chaos_engineering_scenarios.rs`

### For cellMembrane team
- eastGate 13/13 NUCLEUS LIVE (biomeos + nestgate fixed, no sudo, 28 sockets)
- Fresh binary needed from pepti (SSH→forgejo fix blocks HEAD builds)

### For primalSpring
- 75 scenarios passing; toadStool dispatch pipeline validated
- MitoBeacon genetics compliance: `0xEC` + `0xED` accepted
- `capability_registry.toml` v0.2.0 — 17 groups, ready for primalSpring validation

---

## Files Changed (this handoff period)

### Production
- `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` — zero-copy optimizations
- `crates/server/src/pure_jsonrpc/handler/dispatch/pipeline.rs` — first-stage borrow
- `crates/server/src/pure_jsonrpc/handler/dispatch/telemetry.rs` — `binary_size` API
- `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` — `Arc<EncryptionKey>`
- `crates/server/src/pure_jsonrpc/handler/hw_learn/helpers.rs` — `PRIMAL_NAME`
- `crates/server/src/unibin/format.rs` — XDG fallback fix
- `crates/core/cylinder/src/vfio/sovereign_handoff/steps/warm_swap.rs` — refactored
- `crates/core/cylinder/src/vfio/sovereign_handoff/steps/warm_swap_catalyst.rs` — NEW
- `crates/distributed/src/coordination/discovery/client.rs` — capability constants
- `crates/runtime/edge/Cargo.toml` — removed unused cc/bindgen
- `crates/runtime/specialty/Cargo.toml` — removed unused cc/bindgen
- 36 files — SPDX header fixes

### Tests
- `crates/server/tests/server_unibin_background_s155_tests.rs` — PRIMAL_SOCKET→BIOMEOS_SOCKET_DIR
- `crates/server/tests/unibin_deep_coverage_s172_tests.rs` — same evolution

---

*Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.*
