# toadStool Wave 111 ACK — TOADSTOOL-AUTO-REGISTER DONE

**Date**: 2026-06-12
**From**: toadStool (strandGate)
**Re**: Wave 111 — `TOADSTOOL-AUTO-REGISTER` (P2)
**Status**: **DONE** (S309, commit feb49a291)

---

## TOADSTOOL-AUTO-REGISTER — CONFIRMED DONE

Wave 111 identified toadStool's lack of hardware auto-registration as blocking
autonomous `gate.bootstrap` for compute gates. toadStool did not include GPU/NPU
inventory in its `ipc.register` or `primal.announce` payloads — songBird and
biomeOS knew toadStool offered "compute" capability but not *what hardware* was
available.

### What S309 Did

| Component | Change |
|-----------|--------|
| `discover_hardware_inventory()` | New function: PCI sysfs enumeration of GPU/NPU devices (VGA 0x030000, 3D 0x030200). Returns BDF, type, vendor/device ID, bound driver. |
| `register_with_discovery()` | `ipc.register` params now include `devices` array with full hardware inventory |
| `self_announce_to_biomeos()` | `primal.announce` outbound params now include `devices` array |
| `primal_announce` handler | Inbound `primal.announce` response now includes `devices` (BDF list from sysfs) |
| Tests | 2 structural validation tests for `discover_hardware_inventory` |

### Wire Format (devices field)

```json
{
  "devices": [
    {
      "bdf": "0000:01:00.0",
      "type": "gpu_vga",
      "vendor_id": "0x10de",
      "device_id": "0x1db1",
      "driver": "nvidia"
    }
  ]
}
```

Included in both `ipc.register` (songBird) and `primal.announce` (Neural API) payloads.
Coordination plane can now build compute topology without manual registration.

### Verification

- `cargo check` — clean
- `cargo clippy` — zero new warnings
- `cargo test -p toadstool -p toadstool-server --lib` — 1060 passed, 0 failed
- New tests: `discover_hardware_inventory_returns_vec`, `discover_hardware_inventory_includes_vendor_and_device_ids`

### Effect

- Compute gates can self-describe hardware at startup
- songBird knows GPU fleet topology per-gate
- Unblocks autonomous `gate.bootstrap` for compute gates
- No manual registration needed on strandGate (2 GPUs auto-discovered)

---

## toadStool Status

| Item | Status |
|------|--------|
| `TOADSTOOL-AUTO-REGISTER` | **DONE** (S309) |
| `TOADSTOOL-SOCKET-CLEANUP` | DONE (S308) |
| Transport (`TRANSPORT_ENDPOINT`) | DONE (S301–S302) |
| Zero production files >750L | DONE (S307) |
| Zero deprecated sync ctors | DONE (S305) |
| Zero production `#[allow]` | DONE (S291) |
| Root docs synchronized | DONE (S308b) |
| Tests | 23,000+ (9,069+ lib), 0 failures |
| P1 | **ZERO** |
| P2 | **ZERO** |

**toadStool is CLEAN. No remaining P1/P2 items from Wave 111.**

---

## Request to Upstream

Please update the Wave 111 ecosystem snapshot:
- `TOADSTOOL-AUTO-REGISTER`: DONE → remove from remaining work
- strandGate "manual registration needed for GPUs": resolved
- toadStool remaining items: **ZERO**
