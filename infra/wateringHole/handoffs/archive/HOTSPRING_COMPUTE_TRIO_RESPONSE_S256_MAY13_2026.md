# hotSpring compute trio — toadStool response (S256, May 13, 2026)

Response to hotSpring's "Remaining Upstream Work" audit (May 13, 2026).

---

## toadStool action: FECS warm-state init wired

### What shipped

1. **`NvVfioComputeDevice::probe_warm_fecs()`** — BAR0 probe for warm-preserved
   FECS state from nouveau/nvidia-470 handoff. Reads:
   - `PMC_ENABLE` (0x200): popcount ≥ 8 confirms engines are warm
   - `FECS CPUCTL` (0x409100): bit 5 (HALTED) = firmware halted cleanly
   - `FECS MAILBOX0` (0x409040): non-zero = firmware communication active
   - `BOOT0` (0x0): chip identity for capability population

   When HALTED + MAILBOX0 ≠ 0 detected: `fecs_ready = true`, device is
   compute-ready without firmware cold-boot.

2. **VFIO path in device factory** — `create_cylinder_device_factory()` now has
   two detection paths:
   - **DRM path**: render node → driver probe → `AmdDevice` (amdgpu)
   - **VFIO path**: no render node → check sysfs `vfio-pci` binding → BAR0
     warm FECS probe → `NvVfioComputeDevice` if warm

   A VFIO-bound NVIDIA GPU with warm FECS is now returned as a live
   `ComputeDevice` for Phase D dispatch.

3. **CPUCTL HALTED bit fix** — Reconciled inconsistent HALTED bit across the
   codebase:
   - `falcon.rs` (authoritative): `CPUCTL_HALTED = 1 << 5` (0x20) — correct
   - `mmio.rs` (server handler): was `0x10` (bit 4 = HRESET) → fixed to `0x20`
   - `firmware.rs` (glowplug): was `0x10` → fixed to `0x20`
   
   Bit 4 is HRESET (hard reset), bit 5 is HALTED (software halt / context-switch
   freeze). For warm detection, bit 5 is the correct signal.

4. **Tests**: +2 tests verifying warm FECS gate behavior:
   - `warm_fecs_enables_alloc_gate`: cold → FECS error, warm → PBDMA stub
   - `warm_fecs_enables_dispatch_gate`: warm device passes FECS gate

### What hotSpring should do now

1. **Re-enable `sovereign-dispatch` feature** — `NvVfioComputeDevice` now returns
   a live `ComputeDevice` when warm FECS is detected
2. **Run `exp184_k80_gr_sovereign`** — K80 warm-catch should now flow through to
   the PBDMA dispatch stub (returns `Unsupported("PBDMA not yet wired")` but the
   FECS gate is open)
3. **Run `validate_vfio_sovereign`** on Titan V — same: FECS gate open, PBDMA
   stub is the next layer
4. **Set `HOTSPRING_TITAN_V_BDF` / `HOTSPRING_K80_BDF` env vars** — factory uses
   PCI BDF to resolve VFIO-bound devices

### Remaining PBDMA work (next layer)

Once hotSpring confirms warm FECS detection works on hardware, the next step is
wiring actual PBDMA dispatch through the `ComputeDevice` trait methods:
- `alloc()` → VFIO DMA buffer allocation
- `upload()` → DMA copy to GPU
- `dispatch()` → GPFIFO pushbuf submission via PBDMA
- `readback()` → DMA copy from GPU

These are the VFIO channel operations already in `cylinder/src/vfio/channel/` —
they need to be wired through `NvVfioComputeDevice`'s trait impl.

## coralReef items — not toadStool's domain

HMMA codegen for tensor-core GEMM is coralReef compiler work. No action for us.

## barraCuda items — not toadStool's domain

`TensorSession`, `stats.entropy`, multi-GPU OOM recovery are barraCuda items.

---

**Quality metrics (S256)**:

| Metric | Value |
|---|---|
| Lib tests | 8,834 |
| Clippy warnings | 0 |
| `cargo deny check bans` | clean |
| JSON-RPC methods (direct) | 77 |
| Cylinder tests | 528 (521 + 7 compute_device) |

**Filed by**: toadStool S256 | **Quality gates**: all green | **Push**: SSH
