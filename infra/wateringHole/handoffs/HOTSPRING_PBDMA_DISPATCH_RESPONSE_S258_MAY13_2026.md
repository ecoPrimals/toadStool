# hotSpring Compute Trio — Evolution Focus Response (S258)

**Date**: May 13, 2026
**Session**: S258
**Audit**: hotSpring → Compute Trio — Evolution Focus (May 13, 2026)

---

## toadStool — PBDMA Dispatch Wiring (Resolved)

### What was requested

Wire PBDMA dispatch through the `ComputeDevice` trait impl:
- `alloc()` → VFIO DMA buffer allocation
- `upload()` → DMA copy to GPU
- `dispatch()` → GPFIFO pushbuf submission via PBDMA
- `readback()` → DMA copy from GPU

### What was implemented

`NvVfioComputeDevice` now holds optional `VfioDispatchState` containing:
- `VfioDevice` (VFIO fd lifecycle)
- `MappedBar` (BAR0 MMIO access)
- `VfioChannel` (PFIFO channel with all DMA infrastructure)
- `DmaBackend` (IOMMU mapping context)
- GPFIFO ring (512-entry, 4 KiB at IOVA 0x10000)
- USERD page (4 KiB at IOVA 0x11000)
- Buffer map (`HashMap<u32, DmaBuffer>`)
- IOVA allocator (bump from 0x20000, 2 MiB identity-map limit)

#### `open_vfio()` — Device initialization
- Opens `VfioDevice` for the BDF
- Maps BAR0 via `map_bar(0)`
- Allocates GPFIFO + USERD DMA buffers
- Creates `VfioChannel` (warm mode if FECS ready, cold otherwise)
- Warm mode preserves falcon engine state (FECS/GPCCS)

#### `alloc(size, domain)` → `BufferHandle`
- Page-aligned DMA buffer via `DmaBuffer::new(backend, size, iova)`
- IOVA bump allocator from 0x20000 to 0x1FFFFF (2 MiB limit)
- Stored in `HashMap<u32, DmaBuffer>` keyed by handle ID

#### `free(handle)`
- Removes and drops DMA buffer from map (automatic IOMMU unmap + dealloc)

#### `upload(handle, offset, data)`
- Direct host-side `memcpy` into `DmaBuffer::as_mut_slice()`
- Bounds-checked against buffer size

#### `readback(handle, offset, len)` → `Vec<u8>`
- Direct host-side read from `DmaBuffer::as_slice()`
- Bounds-checked against buffer size

#### `dispatch(shader, buffers, dims, info)`
- Allocates a DMA pushbuffer for `shader` bytes
- Builds GPFIFO entry: `[31:2] = pb_iova, [42:32] = length_dwords`
- Writes entry into GPFIFO ring at current GP_PUT position
- Updates USERD GP_PUT via volatile write
- SeqCst memory fence
- Rings doorbell via BAR0 `NOTIFY_CHANNEL_PENDING`
- Tracks pushbuffer in inflight list for cleanup

#### `sync()`
- Polls USERD GP_GET until it matches GP_PUT (1 ms intervals, 1000 iterations)
- Frees all inflight pushbuffers

### Gate model (two-stage)

1. **FECS gate**: alloc + dispatch require `fecs_ready == true`
2. **VFIO gate**: alloc + upload + readback + dispatch require `vfio_state.is_some()`

Without FECS → "FECS compute context" error.
Without VFIO open → "VFIO not opened" error.

### What hotSpring should validate

1. **DMA buffer roundtrip**: `alloc` → `upload` → `readback` on Titan V / K80
2. **GPFIFO submission**: `dispatch` with a NOP pushbuffer, verify GP_GET advances
3. **Warm channel**: After `probe_warm_fecs()` succeeds, verify `open_vfio()` creates warm channel
4. **Sync timing**: Verify `sync()` returns promptly after PBDMA processes the GP entry

The `shader` parameter to `dispatch()` is treated as a raw GPU pushbuffer
(pre-built method headers). For compute kernel dispatch, coralReef builds
the Volta compute class methods (SET_SHADER_PROGRAM, LAUNCH, etc.) and
toadStool submits them. This matches the trio separation: coralReef = HOW
(compiler/methods), toadStool = WHERE (hardware/submission).

### Remaining for E2E compute dispatch

- **Compute QMD construction**: Not in scope — coralReef or the caller
  builds Volta compute class method headers
- **IOVA limit**: Current 2 MiB identity map. For larger workloads,
  extend PT0 entries or add a second page table
- **Multi-channel**: Single channel per device. Multi-channel isolation
  (copy engine vs compute) can use `VfioChannel::create_on_runlist()`

---

## barraCuda — TENSOR_WIRE_CONTRACT.md Gap

The `TENSOR_WIRE_CONTRACT.md` batch ops table lists `create`, `add`, `mul`,
`fma` but does NOT list `sub` or `negate` even though IPC implements them.
**This doc lives in barraCuda's repo, not toadStool.** barraCuda team should
align the contract document with the implementation.

---

## coralReef — Notes

IPC field name mismatch (`"source"` vs `"wgsl_source"`) is a coralReef issue.
We've noted it for awareness but it's not actionable on our side. HMMA codegen
is coralReef's focus — we're ready to submit whatever they build via PBDMA.

---

## Metrics

| Metric | Value |
|--------|-------|
| JSON-RPC methods | 77 |
| Lib tests | 8,837 |
| Clippy warnings | 0 |
| `cargo deny` | clean |
