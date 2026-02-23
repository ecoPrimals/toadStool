# WebSocket Deprecation Audit

**Date:** 2026-02-22  
**Scope:** ToadStool Rust codebase (`crates/`)

## Summary

This audit documents remaining WebSocket API usage after the R-011 deprecation (WebSocket removed; use JSON-RPC 2.0 polling). All production WebSocket usage has been deprecated; no active WebSocket connections remain.

---

## 1. Total Remaining WebSocket References

| Category | Count | Notes |
|----------|-------|-------|
| Deprecated constants/APIs | 8 | `WS_PROTOCOL`, `WSS_PROTOCOL`, `ws_url`, `wss_url`, `protocols::WEBSOCKET`, `protocols::WSS`, `ServiceEndpoint::websocket()`, `WS_PROTOCOL_VERSION` |
| Config fields (deprecated) | 4 | `enable_websocket`, `websocket_timeout` (client), `enable_websocket` (server, api) |
| Trait methods (deprecated) | 1 | `setup_websocket_federation` (returns error) |
| Error variant (deprecated) | 1 | `ClientError::WebSocket` |
| Capability/taxonomy references | 2 | `NetworkFeature::WebSocket`, `MessagingWebsocket` |
| Tests (with `#[allow(deprecated)]`) | ~25 | Validate deprecation or backward compat |
| Comments/docs | ~15 | "WebSocket removed", migration notes |

---

## 2. Tests vs Production

### Tests (leave with `#[allow(deprecated)]`)

- **`crates/client/tests/comprehensive_client_tests.rs`** — Client config defaults (`enable_websocket`, `websocket_timeout`)
- **`crates/client/tests/client_error_comprehensive_tests.rs`** — `ClientError::WebSocket` display
- **`crates/client/tests/client/container_builders.rs`** — `ClientError::WebSocket` display
- **`crates/cli/tests/universal_federation_comprehensive_tests.rs`** — `setup_websocket_federation` returns error
- **`crates/server/tests/*`** — Server config `enable_websocket` builder
- **`crates/core/common/src/constants/network.rs`** — `WS_PROTOCOL`, `wss_url` tests
- **`crates/core/common/src/interned_strings.rs`** — `protocols::WEBSOCKET` test
- **`crates/core/common/src/service_discovery/endpoint.rs`** — `ws://`, `wss://` URL parsing

### Production (deprecated, pending removal)

- **`crates/core/common/src/constants/network.rs`** — `WS_PROTOCOL`, `WSS_PROTOCOL`, `ws_url`, `wss_url`
- **`crates/core/common/src/constants/versions.rs`** — `WS_PROTOCOL_VERSION` (unused, deprecated)
- **`crates/core/common/src/interned_strings.rs`** — `protocols::WEBSOCKET`, `protocols::WSS`
- **`crates/core/common/src/primal_identity/types.rs`** — `ServiceEndpoint::websocket()`
- **`crates/client/src/client/config.rs`** — `websocket_timeout`, `enable_websocket`
- **`crates/server/src/config/mod.rs`** — `enable_websocket`
- **`crates/api/src/types.rs`** — `enable_websocket`
- **`crates/cli/src/universal/operations/federation.rs`** — `setup_websocket_federation` (returns error)
- **`crates/client/src/client/error.rs`** — `ClientError::WebSocket` (never constructed in prod)

---

## 3. WebSocket Dependencies Removed

| Location | Dependency | Status |
|----------|------------|--------|
| **`crates/*/Cargo.toml`** | `tokio-tungstenite`, `tungstenite` | **None found** — no WebSocket deps in crates |
| **`examples/Cargo.toml`** | `tokio-tungstenite` | **REMOVED** — was unused |

`deny.toml` bans `tungstenite` and `tokio-tungstenite` in core paths (R-011).

---

## 4. Changes Made

### Deprecations added

1. **`WS_PROTOCOL_VERSION`** (`crates/core/common/src/constants/versions.rs`) — Unused; marked deprecated.
2. **`ClientError::WebSocket`** (`crates/client/src/client/error.rs`) — Never constructed in production; marked deprecated. `From<ClientError>` impl gets `#[allow(deprecated)]` for backward compat.

### Removals

1. **`tokio-tungstenite`** — Removed from `examples/Cargo.toml` (unused).

### Test cleanups

1. **`crates/integration/protocols/tests/transport_coverage_tests.rs`**
   - Removed redundant "WebSocketTransport" section (duplicated TRpcTransport tests).
   - Renamed `test_transport_enum_websocket_variant` → `test_transport_enum_trpc_variant_alias`.
   - Renamed `test_transport_enum_send_message_websocket` → `test_transport_enum_send_message_trpc`.
   - Removed duplicate `test_transport_enum_send_message_trpc`.
   - Updated test summary counts (37 tests).

### Documentation

1. **`NetworkFeature::WebSocket`** — Added deprecation note in doc comment.
2. **`crates/client/tests/client_error_comprehensive_tests.rs`** — Added `#![allow(deprecated)]`.
3. **`crates/client/tests/client/container_builders.rs`** — Added `#[allow(deprecated)]` on WebSocket test.
4. **`crates/cli/tests/universal_federation_comprehensive_tests.rs`** — Added `#![allow(deprecated)]`.

---

## Next Steps

1. **Future removal** — After a deprecation period, remove deprecated config fields, constants, and `ClientError::WebSocket`.
2. **`MessagingWebsocket` capability** — Consider deprecating in taxonomy or mapping to JSON-RPC 2.0.
3. **`ServiceEndpoint::from_url_string("ws://...")`** — Keep for parsing legacy config; no active WebSocket transport.
