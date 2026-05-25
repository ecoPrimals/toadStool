# TOADSTOOL S266 — PLX Keepalive: Continuous Config Space Heartbeat

**Date**: May 15, 2026
**Status**: IMPLEMENTED — awaiting hardware validation after power cycle
**Supersedes**: S264 PCIe bridge keepalive (pinning + SwapGuard were necessary but insufficient)

## Root Cause Discovery

Analysis of `dmesg` timelines revealed that the Tesla K80's repeated PLX D3cold
events were caused by **inactivity**, not by driver swap events.

The `toadstool-server` PCIe keepalive task's periodic polling ("All device reset
methods disabled" messages every ~5s) was *accidentally* serving as a PLX keepalive.
When this polling ceased (process restart, crash, or intentional stop), the PLX
bridge received no PCIe traffic for ~10 minutes and the kernel's runtime PM put
it into D3cold. Once D3cold hits, the PLX PEX 8747's EEPROM configuration is lost
and only a physical power cycle can recover it.

**Root cause: PLX D3cold is caused by inactivity, not by swap events.**

## What Changed

### ember — `plx_keepalive.rs` (NEW)

| Type | Description |
|------|-------------|
| `PlxKeepalive` | Performs periodic PCI config space reads (4 bytes at offset 0x00) on a device and every upstream bridge in its hierarchy |
| `KeepaliveHandle` | Stop/query handle for a running keepalive task (running flag, heartbeat count) |
| `detect_plx_bridge(bdf)` | Checks vendor ID `0x10b5` (PLX/Broadcom) in bridge ancestry |
| `detect_bridge_chain(bdf)` | Walks sysfs ancestry to find all PCI bridges between device and root |

Behavior per heartbeat:
1. Read PCI config offset 0x00 (Vendor/Device ID) on every BDF in chain
2. If any returns `0xFFFFFFFF` — warn + re-pin `power/control=on`, `d3cold_allowed=0`
3. Increment heartbeat counter

Default interval: 5 seconds. Configurable via `PlxKeepalive::new(bdf, interval)`.

### glowplug — `plx.rs` (NEW)

| Type | Description |
|------|-------------|
| `PlxGuardian` | Fleet-level keepalive manager. Holds `HashMap<String, KeepaliveHandle>` |
| `PlxDeviceStatus` | Serializable status struct (bdf, running, heartbeats) |

Key methods:
- `scan_and_protect(&[DeviceId])` — auto-detect PLX bridges among discovered devices
- `protect(bdf)` — protect a single device
- `release(bdf)` / `release_all()` — stop keepalives
- `status_summary()` → `Vec<PlxDeviceStatus>`

### Integration Point

`toadstool-server` startup sequence should be:

```rust
// After device discovery
let mut plx_guardian = PlxGuardian::new();
let protected = plx_guardian.scan_and_protect(&discovered_devices);
tracing::info!(protected, "PLX keepalive tasks started");
```

## Test Coverage

- ember `plx_keepalive`: 8 unit tests (chain detection, handle lifecycle, nonexistent device safety)
- glowplug `plx`: 8 unit tests (guardian lifecycle, non-PCI filtering, serde roundtrip)
- All 98 ember tests pass, all 95 glowplug tests pass

## Relationship to S264

S264 (pinning + SwapGuard) remains **necessary** — it prevents D3cold during the
critical unbind/rebind window. S266 (continuous keepalive) prevents D3cold during
**idle periods** between operations. Both are required for reliable K80 operation.

```
S264: pin_bridge_hierarchy() + SwapGuard burst  →  swap-time protection
S266: PlxKeepalive continuous heartbeat          →  idle-time protection
```

## Files

| File | Change |
|------|--------|
| `crates/core/ember/src/plx_keepalive.rs` | NEW — PlxKeepalive, KeepaliveHandle, detect_plx_bridge |
| `crates/core/ember/src/lib.rs` | Added plx_keepalive module + re-exports |
| `crates/core/glowplug/src/plx.rs` | NEW — PlxGuardian, PlxDeviceStatus |
| `crates/core/glowplug/src/lib.rs` | Added plx module + re-exports |
| `experiments/193_PLX_D3COLD_KEEPALIVE_K80.md` | Updated with root cause + Phase 2 code |
| `experiments/195_DRIVER_LAB_MESA_VS_VENDOR.md` | Added K80 keepalive cross-reference |
| `NEXT_STEPS.md` | Updated to S266 |
