# ToadStool S220 — primalSpring Phase 58 Audit Response

**Date**: May 4, 2026
**Session**: S220
**Trigger**: primalSpring Phase 58 Debt Handoff (4 items for toadStool)

---

## Audit Items Received

### 1. Phase 3 transport encryption (HIGH)
> `btsp.negotiate` advertises AEAD — server needs to switch transport post-negotiate.

**Status: RESOLVED (S215+S218)**

Verified all 6 checkpoints:
- `try_handle_negotiate` exists → returns `NegotiateOutcome::Negotiated(keys)` with real `Phase3SessionKeys`
- Server `unix.rs`: `Negotiated(keys)` → `handle_encrypted_session` → exclusively `read_encrypted_frame`/`write_encrypted_frame`
- Daemon `jsonrpc_server.rs`: `Negotiated(keys)` → `daemon_encrypted_loop` → exclusively encrypted framing
- `phase3.rs`: `Phase3SessionKeys::derive` (HKDF-SHA256) + encrypt/decrypt (ChaCha20-Poly1305) exist
- `framing.rs`: `read_encrypted_frame`/`write_encrypted_frame` exist
- CHANGELOG.md + DEBT.md: S215 + S218 entries confirm implementation + verification

NDJSON continues only for `NullCipher` / `NotNegotiate` fallbacks — not after AEAD negotiate.

### 2. Coverage 83.6% → 90% target (MEDIUM)
**Status: ACTIVE — pushed +22 tests this session (22,560 total)**

New tests added:
- `crates/runtime/wasm/src/metrics.rs`: 7 tests (was 0) — atomic counters, averages, edge cases
- `crates/core/toadstool/src/execution/stub_runtime_engine.rs`: 7 tests (was 0) — error msg, capabilities, lifecycle
- `crates/core/toadstool/src/os_layer/manager.rs`: 4 tests (was 8) — initialize, disabled mode, execute routing
- `crates/runtime/container/src/engine.rs`: 4 tests (was 9) — initialize, non-container rejection, metrics, shutdown

### 3. `display.composite` multi-layer (LOW)
**Status: NOT IMPLEMENTED — needs spec**

- Zero occurrences of `display.composite` in any .rs file
- Display IPC dispatch handles: create_window, destroy_window, resize_window, get_window_info, present, subscribe_input, poll_events, get_capabilities
- `DISPLAY_BACKEND_SPEC.md` does not define `display.composite`
- PG-42 does not appear in the repo; only reference is S207 handoff "Next Evolution" bullet
- Implementation would require: spec definition, compositor/multi-layer logic, DRM plane blending, dispatch extension

### 4. `transport.bridge` (LOW)
**Status: NOT IMPLEMENTED — needs spec**

- Zero occurrences of `transport.bridge` in any .rs file
- Transport JSON-RPC methods: discover, list, route, open, stream, status — no `bridge`
- `HARDWARE_TRANSPORT_SPEC.md` does not mention `transport.bridge`
- "bridge" only appears as PCIe topology language (`common_bridge`) in display crate
- Implementation would require: spec definition of purpose (chaining? topology-assisted pairing?), handler extension

---

## Production Stub Evolved

**`OSLayerManager::execute_with_os_layer` fallback**:
- Before: returned synthetic `Ok(ExecutionResponse { status: Success, stdout: "Default OS layer execution" })` when no layer matched
- After: returns `Err(ToadStoolError::not_supported("No compatibility layer can handle this request..."))`
- Pattern: same as S219 stub evolution (coordination health, legacy compat)

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | **22,560 tests, 0 failures** |
| `cargo clippy --workspace` | 0 new warnings (fixed float_cmp + unit struct default) |
| `cargo fmt --all --check` | 0 diffs |

---

## Files Modified

| File | Change |
|------|--------|
| `crates/runtime/wasm/src/metrics.rs` | +7 tests |
| `crates/core/toadstool/src/execution/stub_runtime_engine.rs` | +7 tests |
| `crates/core/toadstool/src/os_layer/manager.rs` | +4 tests, evolved fallback stub, cleaned imports |
| `crates/runtime/container/src/engine.rs` | +4 tests (initialize, execute, metrics, shutdown) |
| `DEBT.md` | S220 entry |
| `README.md` | Updated test count, added S220 to recently completed |
| `CONTEXT.md` | Updated test count |
| `NEXT_STEPS.md` | Updated to S220 |
