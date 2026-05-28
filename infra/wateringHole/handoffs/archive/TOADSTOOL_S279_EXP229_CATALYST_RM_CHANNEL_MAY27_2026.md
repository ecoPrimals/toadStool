# ToadStool S279 — Exp 229: Catalyst RM Channel

**Date:** 2026-05-27
**Session:** S279
**Experiment:** 229 — Catalyst Channel: RM Compute Channel Before Warm Swap
**Status:** Implemented, awaiting hardware validation
**Hardware:** Dual Titan V (GV100) — 0000:02:00.0 + 0000:49:00.0

---

## Summary

Extended `rm_trigger` binary to create a full 16-step NVIDIA RM compute channel
while the catalyst driver (nvidia-470) is loaded, BEFORE warm swap to vfio-pci.
This addresses the FECS ACR blocker from Exp 228 (`pccsr=0x11000001` PENDING).

## Changes

### rm_trigger --channel (16-step Volta RM channel recipe)

New `--channel` flag triggers full compute channel creation:
root → device → subdevice → GR_GET_INFO → VA space → USERD/GPFIFO/notifier
memory → TSG → ctx share → GPFIFO channel → compute → BIND → SCHEDULE →
work submit token. Uses `rm_abi.rs` types for class-specific params, retains
32-byte `Nvos64Parameters` for 470.x `NV_ESC_RM_ALLOC` ioctl.

### Pipeline Integration

- `RmChannelEvidence` struct: captures channel_id, work_submit_token, steps_completed
- `trigger_rm_init()` accepts `create_channel: bool`, passes `--channel` to binary
- `HandoffResult` carries `rm_channel_evidence: Option<RmChannelEvidence>`
- PCCSR scan in Step 4b (catalyst_capture): 64 channel slots scanned for ACTIVE

### Post-Swap Strategy

- **Phase B** (primary): new sovereign channel, hypothesis FECS is primed
- **Phase A** (fallback): `adopt_rm_channel()` + `VfioChannel::adopt_existing()`
  uses RM channel's hardware ID with sovereign DMA infrastructure

## Files Changed (11 files, +734 / -210 lines)

- `cylinder/src/bin/rm_trigger.rs` — 16-step RM channel recipe
- `cylinder/src/nv/compute_device/channel_init.rs` — `adopt_rm_channel()`
- `cylinder/src/nv/compute_device/mod.rs` — `rm_channel_id` field
- `cylinder/src/nv/compute_device/open_vfio.rs` — Phase A/B fallback logic
- `cylinder/src/vfio/channel/mod.rs` — `VfioChannel::adopt_existing()`
- `cylinder/src/vfio/sovereign_handoff/mod.rs` — export `RmChannelEvidence`
- `cylinder/src/vfio/sovereign_handoff/pipeline.rs` — PCCSR scan, evidence
- `cylinder/src/vfio/sovereign_handoff/rm_trigger.rs` — `--channel` flag
- `cylinder/src/vfio/sovereign_handoff/rollback.rs` — type compatibility
- `cylinder/src/vfio/sovereign_handoff/tests.rs` — type compatibility
- `cylinder/src/vfio/sovereign_handoff/types.rs` — `RmChannelEvidence` struct

## Validation

- 705 cylinder + 864 server = 1,569 lib tests pass
- Full workspace `cargo check` clean (1 deprecation warning in server)
- System rebooted, both GPUs clean on vfio-pci

## Next

Run `sovereign.warm_handoff` with `nvidia_catalyst_titanv` on clean GPU.
If Phase B fails (sovereign channel still PENDING), Phase A fallback
adopts RM channel's hardware layout.

## Downstream

- hotSpring: `experiments/229_CATALYST_RM_CHANNEL.md` + `EXPERIMENT_INDEX.md` updated
- primalSpring: will audit on push
