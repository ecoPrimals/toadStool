# hotSpring Tier 2 — PBDMA Root Cause Analysis

**Date**: June 1, 2026
**From**: primalSpring cascade (eastGate) → toadStool (biomeGate)
**Session**: Post-S282, Exp 234 Run #5 aftermath
**Gate**: biomeGate (Threadripper 3970X, Titan V + K80, 256GB)

---

## Root Cause: PBDMA Runlist Never Configured

The Titan V VFIO dispatch pipeline is fully wired — channel creation, DMA
allocation, pushbuffer encoding, GPFIFO submission all work. But the Host engine
never consumes submitted pushbuffers because the PFIFO scheduler has no runlist
to schedule from.

### Evidence

| Register | Expected | Actual | Meaning |
|----------|----------|--------|---------|
| `PFIFO_RUNLIST_BASE` (0x2270+rl*0x10) | `(RUNLIST_IOVA >> 12)` = 0x4 | 0x00000000 | No runlist in DMA memory |
| `PBDMA_CHANNEL(0)` | channel_id bound | 0x00000000 | PBDMA not bound to any channel |
| `GP_GET` (USERD) | advances after doorbell | stuck at 0 | Host never reads GPFIFO entries |

7 pushbuffer entries submitted via `dispatch()` in `open_vfio.rs`. Doorbell
rung via `NOTIFY_CHANNEL_PENDING`. `GP_GET` never advances — PBDMA has no
runlist telling it which channel to service.

### Diagnostic Commit (ac1d357e5)

`submit_runlist()` now verifies `RUNLIST_BASE` readback is non-zero after write.
`open_vfio()` post-init diagnostic checks:
- `RUNLIST_BASE` = 0 → runlist write didn't stick (PRI fault or wrong slot)
- PCCSR status < 5 → scheduler didn't load channel
- FECS not alive → deferred boot failed
- No GR context → `fecs_ready` was false

### The Write Exists — Why Didn't It Stick?

`submit_runlist()` in `runlist.rs` writes `gv100_runlist_base_value(RUNLIST_IOVA)`
to `runlist_base(runlist_id)`. The code is correct per nouveau's `gv100_runl_commit()`.

Possible failure modes:
1. **PRI fault on runlist register** — the per-runlist PRI domain may not be
   enumerated after PFIFO reset. The PFIFO re-init path in `open_vfio.rs`
   does a PMC reset of bit 8 (PFIFO) but the PRI ring master enumerate
   that follows may not re-register all runlist satellites.
2. **Wrong runlist slot** — `runlist_id` from `init_channel_buffers` may not
   match the GR engine's actual runlist. GV100 has per-engine runlists;
   PBDMA→runlist mapping comes from `PTOP` discovery.
3. **nvsov module stuck** — Exp 234 Run #5 left `nvsov` loaded. The nvidia-470
   DKMS module may hold PRI ownership over PFIFO registers, causing writes
   to be absorbed by the driver's own context and not reflected in BAR0.

### Fix Path

1. **Reboot biomeGate** — clear `nvsov` from Exp 234 Run #5
2. **`sovereign.warm_handoff`** with `nvidia_catalyst_minimal_nop_titanv`
   - This establishes catalyst warm state: FECS alive, GPCCS alive, 63K+ regs
3. **Verify RUNLIST_BASE sticks** — after catalyst, the per-runlist PRI domain
   should be active (nvidia-470 initialized it). Write `RUNLIST_BASE`, read
   back, confirm non-zero.
4. **If stuck**: explicit `PPRIV_RING_MASTER_COMMAND = 1` (enumerate) after
   PFIFO reset, then retry runlist submission.
5. **Channel adoption** — bind existing catalyst channel or create new one on
   the active runlist. Submit runlist with TSG + channel entries.
6. **FECS golden context reload** — poke `FECS_HOST_INT_CLEAR` +
   `CTXSW_MAILBOX(0) = PENDING_CTX_RELOAD` to trigger FECS context switch
   into our channel's GR context.

### After Runlist

Once `GP_GET` advances:
- PBDMA will fetch pushbuffer entries from GPFIFO
- FECS dispatches GR methods from pushbuffers to SMs
- Shader execution becomes possible (Tier 2 → Tier 3 transition)

### Relationship to Spec

This handoff is referenced by `specs/COMPUTE_DISPATCH_ENGINE.md` P0 Actions.
The spec's "Two Compute Paths" section describes the VFIO path as BLOCKED
pending this exact fix.

### Working Path (for context)

The wgpu/DRM path on RTX 5060 Blackwell is production today:
- 256/256 f32 verified end-to-end (commit 1f1bec4e9)
- SPIR-V validation + naga/WGSL fallback
- Device-lost guard + panic-abort resilience
- `shader.dispatch` → `wgpu_dispatch.rs` → Vulkan compute pipeline

---

## For primalSpring

### Validate
- [ ] Reboot biomeGate clears `nvsov` (lsmod shows no nvidia-470 residue)
- [ ] `sovereign.warm_handoff` completes without PRI faults
- [ ] `RUNLIST_BASE` readback is non-zero after `submit_runlist()`
- [ ] `GP_GET` advances after doorbell ring
- [ ] FECS `PENDING_CTX_RELOAD` triggers context switch

### Risk
- If catalyst warm state doesn't preserve per-runlist PRI domain ownership,
  we may need to probe `PTOP_DEVICE_INFO` to discover which runlist slots
  are active and skip cold-init of ones the catalyst already configured.
- K80 (Kepler) uses `gk104_runlist_base_value` with explicit target bits —
  different code path, separate validation needed.
