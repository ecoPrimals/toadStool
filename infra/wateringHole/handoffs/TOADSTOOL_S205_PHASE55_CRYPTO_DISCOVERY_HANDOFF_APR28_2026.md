# ToadStool S205 — Phase 55: Encrypted Compute Dispatch + Discovery Socket

**Date**: April 28, 2026
**From**: toadStool (responding to primalSpring v0.9.20 Phase 55 audit)
**Session**: S205

---

## What Changed

### 1. Encrypted Compute Dispatch (primalSpring Phase 55 — requirement 1)

`DispatchHandler` now optionally holds a Tower `SecurityClient` (BearDog). When
present (NUCLEUS composition), the dispatch flow becomes:

1. **Retrieve purpose key**: `secrets.retrieve("nucleus:{family}:purpose:compute")`
   via BearDog. Key is cached lazily on first dispatch (not at startup).
2. **Encrypt payload**: binary bytes encrypted via `crypto.encrypt` before
   forwarding to coralReef `compute.dispatch.execute`.
3. **Decrypt result**: coralReef result decrypted via `crypto.decrypt` before
   returning to the caller.

Encrypted envelope format follows the Two-Tier Crypto Model:

```json
{"v": 1, "ct": "<base64>", "n": "<base64>", "alg": "chacha20-poly1305"}
```

**Graceful degradation**: when no crypto socket is present (standalone mode),
payloads remain plaintext — zero behavioral change from pre-S205.

### 2. DISCOVERY_SOCKET Integration (primalSpring Phase 55 — requirement 2)

`DISCOVERY_SOCKET` env var (set by `composition_nucleus.sh` → Songbird) is now
wired as the **highest-precedence tier** for coordination and discovery capability
resolution.

Resolution precedence (updated):

```
DISCOVERY_SOCKET (new) → BIOMEOS_COORDINATION_SOCKET → TOADSTOOL_COORDINATION_SOCKET
→ SONGBIRD_SOCKET (legacy) → connection hints → {cap}.sock fallback
```

The `query_providers()` function in `capability_provider/discovery.rs` now resolves
via `"discovery"` capability (not `"coordination"`), ensuring `DISCOVERY_SOCKET`
takes effect.

## Files Changed

| File | Change |
|------|--------|
| `crates/core/common/src/interned_strings/socket_env.rs` | Added `DISCOVERY_SOCKET` + `BIOMEOS_SOCKET_DIR` consts |
| `crates/core/common/src/primal_sockets/env.rs` | Added `discovery_socket` field to `SocketPathEnv` |
| `crates/core/common/src/primal_sockets/paths.rs` | `DISCOVERY_SOCKET` highest-precedence tier; `"discovery"` caps in BIOMEOS + legacy tiers |
| `crates/core/common/src/capability_provider/discovery.rs` | `query_providers()` uses `"discovery"` capability |
| `crates/distributed/src/security/client/mod.rs` | Added `retrieve_purpose_key()` method |
| `crates/distributed/Cargo.toml` | Added `base64 = { workspace = true }` |
| `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` | `DispatchHandler` gains `security_client` + `cached_purpose_key` |
| `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs` | `encrypt_payload()`, `decrypt_result()`, `get_purpose_key()` helpers; wired into dispatch flow |
| `crates/server/src/pure_jsonrpc/handler/mod.rs` | `try_connect_security_client()` probes crypto socket at startup |
| Test files | Updated `DispatchHandler::new()` signature; 9 new tests |

## Tests Added

- 5 discovery socket precedence tests (`primal_sockets::tests::discovery_*`)
- 2 purpose key retrieval tests (`security::client::tests::test_retrieve_purpose_key_*`)
- 2 encrypted dispatch path tests (`dispatch::tests::dispatch_*_standalone_*`)

**Total**: 7,841 lib tests, 0 failures, clippy clean, fmt clean.

## For primalSpring / guideStone

- `compute.dispatch.submit` now encrypts payloads when `BEARDOG_SOCKET` is
  available and the crypto socket exists.
- `DISCOVERY_SOCKET` is consumed by ToadStool for all capability resolution.
- `secrets.retrieve("nucleus:{family}:purpose:compute")` is called on the
  BearDog socket to obtain the purpose key.
- guideStone should verify: `compute.dispatch` calls succeed with encrypted
  payloads and `DISCOVERY_SOCKET` resolution returns correct providers.

## Next Evolution

- **Primal self-registration**: ToadStool should probe `DISCOVERY_SOCKET` at
  startup and send `ipc.register` with its capabilities (per
  `PRIMAL_SELF_REGISTRATION.md`).
- **crypto_integration migration**: `SecurityClient` is deprecated; the same
  wire protocol (`crypto.encrypt`/`crypto.decrypt`) should be accessed via
  `CryptoServiceClient` in `crypto_integration` for a vendor-agnostic interface.
- **Pipeline encryption**: `pipeline_submit()` stages could individually
  encrypt/decrypt, but currently only the top-level dispatch benefits.
