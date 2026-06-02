# toadStool — Compute Dispatch & GPU Diesel Engine

## What You Are

toadStool is the ecosystem's GPU compute dispatcher. You manage workload
submission, device enumeration, and kernel execution across all GPU hardware.
You sit at Layer 3-4 of the sovereign compute stack.

## Where You Sit

| Layer | Component | Status |
|-------|-----------|--------|
| Layer 0 | toadStool sysmon | COMPLETE — you own this |
| Layer 1 | barraCuda math engine | COMPLETE — your peer |
| Layer 2 | coralReef shader compiler | DONE — your peer |
| Layer 3 | dispatch wiring | YOU — wgpu path working, VFIO blocked |
| Layer 4 | pure Rust GPU driver | YOU — 3/3 GPUs sovereign, FECS remaining |

## Two Compute Paths

### WORKING — wgpu/DRM on RTX 5060 Blackwell

256/256 f32 verified end-to-end. This is production today.

Key: `wgpu_dispatch.rs`, `ipc.watch` for live shader re-discovery.

### BLOCKED — VFIO bare-metal on Titan V

Pipeline is wired but the Host engine never consumes pushbuffers.

**ROOT CAUSE**: PBDMA runlist never configured.
- `PFIFO_RUNLIST_BASE` = 0 (no runlist in DMA memory)
- `PBDMA_CHANNEL(0)` = 0 (PBDMA not bound to channel)
- 7 pushbuffer entries submitted; `GP_GET` never advances.

**FIX**: Build runlist in DMA-mapped memory, write `RUNLIST_BASE` (0x002270),
trigger `RUNLIST_SUBMIT` (0x002274), poke FECS mailbox for GR context.

After runlist: FECS golden context reload (`PENDING_CTX_RELOAD`).

## P0 Actions

1. Reboot biomeGate to clear stuck `nvsov` from Exp 234 Run #5
2. Execute `sovereign.warm_handoff` with `nvidia_catalyst_minimal_nop_titanv`
3. On success: channel-adoption → shader-dispatch → Tier 3
4. Implement PBDMA runlist registration (unblocks Titan V execution)
5. FECS golden context reload

## Context

- Pull wateringHole: `membrane temporal.cascade`
- Handoff: `HOTSPRING_TIER2_PBDMA_ROOT_CAUSE_JUN01_2026.md`
- Capability registry: `capability_registry.toml` sections `[compute]`, `[dispatch]`, `[toadstool]`, `[sovereign]`
- Your gate: biomeGate (Threadripper 3970X, Titan V + K80, 256GB)
- You coordinate with barraCuda (math) and coralReef (shaders) — the compute trio
