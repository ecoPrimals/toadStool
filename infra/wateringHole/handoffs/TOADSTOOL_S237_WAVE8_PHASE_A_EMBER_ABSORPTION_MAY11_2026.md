# ToadStool S237 — Wave 8 Phase A: coral-ember Absorption

**Date**: May 11, 2026
**Session**: S237
**Author**: toadStool team
**Status**: Phase A COMPLETE

## Summary

Absorbed coralReef's `coral-ember` hardware lifecycle modules into `toadstool-ember`
as the first phase of the Compute Trio Wave 8 absorption. This gives toadStool
native vendor-specific device lifecycle knowledge (NVIDIA, AMD, Intel, BrainChip)
and the first production `ResourceHandle` implementation (`VfioResourceHandle`).

## What Was Absorbed

### From coral-ember → toadstool-ember

| Module | Source LOC | Description |
|--------|-----------|-------------|
| `vendor_lifecycle/` | ~1,100 | VendorLifecycle trait + 9 implementations (NvidiaKepler, NvidiaVolta+, NvidiaOpen, NvidiaOracle, AmdVega20, AmdRdna, IntelXe, BrainChip, Generic) |
| `vendor_lifecycle/detect.rs` | ~80 | PCI ID tables + auto-detection from sysfs |
| `vendor_lifecycle/steps.rs` | ~100 | LifecycleStep enum + injectable sysfs execution |
| `vendor_lifecycle/tests.rs` | ~280 | 41 vendor lifecycle tests |
| `observation.rs` | ~180 | SwapObservation, SwapTiming, HealthResult, ResetObservation |
| `ring_meta.rs` | ~78 | RingMeta, MailboxMeta, RingMetaEntry |
| `error.rs` | ~65 | SwapError, SysfsError error taxonomy |
| `sysfs.rs` | ~130 | SysfsPort trait, pci_device_path(), pin_power(), read_pci_id() |
| `vfio_handle.rs` | ~150 | VfioResourceHandle implementing ResourceHandle |

### Adaptations from coral-ember

- `coral_driver::linux_paths` → `crate::sysfs::pci_device_path()` (inline path construction)
- `coral_driver::vfio::VfioDevice` → `VfioResourceHandle` (BDF + optional fd + RingMeta)
- `CORALREEF_INTEL_SETTLE_SECS` env var → `TOADSTOOL_INTEL_SETTLE_SECS`
- `BRAINCHIP_VENDOR` constant uses canonical `0x1E7C` (aligned with S235 fix)
- `VendorLifecycle::description()` returns `&'static str` (clippy pedantic)
- All `#[expect(dead_code)]` on pub fields replaced with doc comments

### Wire-only principle maintained

No shared Rust crate between toadStool and coralReef. The `coral-driver` dependency
was replaced with toadStool's own `sysfs` module. `shader_info` from coralReef's
`shader.compile.wgsl` response is deserialized from JSON directly — option 3
(wire-only) per the Iter 96 extraction boundary recommendation.

## Dispatch Path Wiring

`DispatchHandler` now has a `device_pool: HashMap<String, HeldResource<VfioResourceHandle>>`
that tracks device handles per BDF:

1. `dispatch_submit_with_context` calls `acquire_device_handle(bdf)` after thermal
   checks and envelope enforcement, before dispatch
2. Device handles persist across dispatches to the same BDF (reused, not recreated)
3. `compute.dispatch.capabilities` reports `ember.held_devices` count and `ember.phase`

## Test Results

| Suite | Count | Status |
|-------|-------|--------|
| toadstool-ember (total) | 90 | All pass |
| ↳ vendor_lifecycle | 41 | All pass |
| ↳ observation | 6 | All pass |
| ↳ ring_meta | 2 | All pass |
| ↳ sysfs | 3 | All pass |
| ↳ vfio_handle | 7 | All pass |
| ↳ (pre-existing: held_resource, journal, lend_reclaim, metadata) | 31 | All pass |
| dispatch trio_contract (new ember tests) | 4 | All pass |
| dispatch (total) | 76 | All pass |
| workspace (all crates) | ~7,000+ lib | All pass |
| clippy --workspace | 0 warnings | Clean |

## What's Next

### Phase B: glowplug absorption
- `sovereign_boot` → `SwapOrchestrator` 7-step
- `EmberClient` becomes toadStool-internal
- `GpuPersonality` unifies with `DevicePersonality`
- `coralctl` → CLI

### Phase C: cylinder + coral-driver hardware
- Per-device subprocess generalized (GPU + NPU + HSM)
- VFIO channel, GPFIFO/pushbuf, DRM ioctl → `hw-safe` containment

### Phase D: local dispatch (Gate 4)
- `dispatch_submit_with_context` executes locally via absorbed driver layer
- Replaces forwarding to `coral_client`

## Coordination

- **coralReef**: Can begin soft-deprecation of `coral-ember` crate. toadStool has
  absorbed the portable modules. Wait for Phase B (glowplug) before removing
  `coral-ember` entirely.
- **barraCuda**: No changes needed. Sovereign dispatch activates automatically
  when coralReef and toadStool are both discoverable.
- **primalSpring**: Gate tests 1-4 + `compute_trio_smoke.toml` ready to validate.
