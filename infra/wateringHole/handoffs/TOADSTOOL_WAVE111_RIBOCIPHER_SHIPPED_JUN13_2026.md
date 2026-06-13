# toadStool Wave 111 ACK — riboCipher SHIPPED + AUTO-REGISTER Confirmed

**Date**: 2026-06-13
**From**: toadStool (strandGate)
**Re**: Wave 111 — riboCipher convergence + TOADSTOOL-AUTO-REGISTER
**Status**: **BOTH DONE** (S311 + S309)

---

## riboCipher Transport Signal — SHIPPED (S311)

toadStool is now riboCipher-compliant per `RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD.md`.

### Server-side detection (4 accept loops)

| Accept Loop | File | Signal Detection |
|-------------|------|-----------------|
| JSON-RPC Unix (`serve_unix_prebound`) | `connection/unix.rs` | First-byte riboCipher → dispatch; legacy → WARN |
| JSON-RPC TCP (`serve_tcp`) | `connection/tcp.rs` | First-byte riboCipher → dispatch; legacy → WARN |
| BTSP Unix (`handle_btsp_connection`) | `connection/unix.rs` | riboCipher before BTSP peek; WARN on unsignalled |
| BTSP Unix (no-feature) | `connection/unix.rs` | riboCipher before plaintext check; WARN on unsignalled |

**Probe (0x00)**: Returns immediate `{"status":"alive"}` on both Unix and TCP.
**Tier 2/3 (0xED/0xEE)**: Stub — logs warning and closes (HKDF infra not yet wired).
**Legacy fallback**: Active with WARN per Wave 111 deprecation timeline.

### Client-side signal (all outbound IPC)

| Caller | Signal |
|--------|--------|
| `register_with_discovery()` (→ songBird) | `[0xEC, 0x01]` before `ipc.register` |
| `find_by_capability()` (→ songBird) | `[0xEC, 0x01]` before `ipc.find_capability` |
| `self_announce_to_biomeos()` (→ biomeOS) | `[0xEC, 0x01]` before `primal.announce` |
| `UnixJsonRpcClient::call()` (shared client) | `[0xEC, 0x01]` after connect |
| `ConnectedJsonRpcClient::connect()` (persistent client) | `[0xEC, 0x01]` after connect |

### Constants

```rust
pub(crate) mod ribocipher {
    pub const CLEAR: u8 = 0xEC;    // Tier 1: clear (local/trusted)
    pub const MITO: u8 = 0xED;     // Tier 2: mito-obfuscated (cross-gate WAN)
    pub const NUCLEAR: u8 = 0xEE;  // Tier 3: nuclear-sealed (privileged)
}
```

### Commit

`60b0e73d4` — S311: riboCipher transport signal convergence

---

## TOADSTOOL-AUTO-REGISTER — Confirmed DONE (S309)

Previously ACK'd in `TOADSTOOL_WAVE111_AUTO_REGISTER_DONE_JUN12_2026.md` (now archived).
PCI sysfs GPU/NPU hardware enumeration wired into `ipc.register` + `primal.announce`.
Commit: `feb49a291`.

---

## toadStool Convergence Summary

| Item | Status | Session |
|------|--------|---------|
| TOADSTOOL-AUTO-REGISTER | DONE | S309 |
| riboCipher server detect | DONE | S311 |
| riboCipher client signal | DONE | S311 |
| riboCipher Tier 2 (mito) | STUB | Awaiting HKDF infra |
| riboCipher Tier 3 (nuclear) | STUB | Awaiting nuclear_seed |
| PRIMAL-SOCKET-CLEANUP | DONE | S308 |

**toadStool has zero remaining Wave 111 items.**
