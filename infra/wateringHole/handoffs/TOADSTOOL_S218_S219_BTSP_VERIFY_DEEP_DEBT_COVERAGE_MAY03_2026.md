# ToadStool S218+S219 Handoff — May 3, 2026

## Session S218: BTSP Phase 3 Transport Switch Verification

**Trigger**: primalSpring downstream audit — "Phase 3 transport switch verification:
verify that after `btsp.negotiate`, the connection transitions to encrypted frame I/O
for subsequent messages."

### Verification Result

Transport switch logic confirmed correct. After `btsp.negotiate` returns
`Negotiated(keys)`, both server (`unix.rs`) and daemon (`jsonrpc_server.rs`)
exclusively use `read_encrypted_frame`/`write_encrypted_frame` for all subsequent
I/O. The negotiate JSON-RPC response is the last NDJSON message; no NDJSON fallback
exists inside the encrypted loop. No interop gap in the code path.

### New Tests (15)

- `framing::encrypted_frame_round_trip` — server→client encrypted frame write+read
- `framing::encrypted_frame_directional_keys` — bidirectional encrypted request/response
- `framing::encrypted_frame_wrong_keys_rejects` — wrong keys yield `InvalidData`
- `framing::encrypted_frame_multiple_round_trips` — sequential encrypted frames
- `json_line::negotiate_chacha20_returns_negotiated_with_keys` — negotiate success
- `json_line::negotiate_null_cipher_when_unsupported` — AES-256-GCM falls back to null
- `json_line::negotiate_not_negotiate_for_other_methods` — non-negotiate pass through
- `json_line::negotiate_not_negotiate_for_empty_line` — empty lines pass through
- `json_line::negotiate_null_cipher_when_no_client_nonce` — missing nonce → null
- `json_line::negotiate_preferred_cipher_hyphen_variant` — `chacha20-poly1305` accepted
- `json_line::negotiate_preferred_cipher_underscore_variant` — `chacha20_poly1305` accepted
- `json_line::negotiate_then_encrypted_frame_exchange` — **full E2E** negotiate→encrypted

### Other S218 Changes

- `NegotiateOutcome` manual `Debug` impl (redacts keys)
- `try_handle_negotiate` doc — BufReader pipelining hazard documented
- 3 flaky `primal_sockets::discovery` tests fixed with `temp_env` isolation

---

## Session S219: Deep Debt — Production Stubs + Lock Safety + Coverage

### Production Stub Evolution (3 stubs → typed errors)

| Stub | File | Before | After |
|------|------|--------|-------|
| Coordination gRPC TCP health | `distributed/coordination/connection.rs` | `Ok(())` | `not_supported` |
| Coordination MQ health | same | `Ok(())` | `not_supported` |
| Legacy compat execute | `core/toadstool/os_layer/compat/legacy.rs` | `Ok(default)` | `not_supported` |
| Monitoring mutex lock | `management/monitoring/reporting.rs` | `.expect("poisoned")` | `LockPoisoned` error |

### Hardcoding Evolution

- `/tmp/biomeos-runtime` fallback → configurable via `BIOMEOS_RUNTIME_DIR` env var
- Resolution: `XDG_RUNTIME_DIR` → `/run/user/{uid}` → `BIOMEOS_RUNTIME_DIR` → `/tmp/biomeos-runtime`

### Test Coverage Expansion (+98 tests)

- **ember** (26 new): `HeldResource` lifecycle, `LendState`/`LendReceipt`, `MetadataStore` edges, `SwapJournal` serde
- **glowplug** (45 new): `DeviceId` all variants, `DeviceSlot` state machine, `HealthStatus`, `Unbound`, `NoFirmwareInterface`, `SwapOrchestrator`/`SwapObservation`

---

## Metrics

- **22,538 tests**, 0 failures
- Clippy clean, fmt clean
- All quality gates green
- Zero production panics/expects
- BTSP Phase 3 fully verified (S215 impl + S218 verification)
- PG-46 resolved (S214)
