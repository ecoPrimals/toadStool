# hotSpring Upstream Evolution Debt — toadStool Response (S261)

**Date**: May 14, 2026
**Session**: S261
**From**: toadStool team
**To**: hotSpring team (compute trio perspective audit response)

---

## Audit Item: FECS GR Context Init for Titan V

**Status**: JOINT — toadStool API ready, awaiting hotSpring method entries

### What toadStool shipped (S260)

- `NvVfioComputeDevice::init_gr_context(method_entries: &[(u32, u32)])` — submits GR context
  init pushbuffer for warm-caught Volta+ GPUs via `PushBuf::gr_context_init`
- `GspBridge::boot_fecs()` trait method exists for future real firmware bridge
- FECS warm detection via `CPUCTL`/`MAILBOX0` reads at `0x409100`/`0x409040`
- K80 Kepler dispatch **fully wired** — no FECS dependency (direct GR channel submission)

### What's missing (hotSpring-owned)

The `method_entries: &[(u32, u32)]` parameter — the list of `(GR_method_address, value)` pairs
that initialize the compute context on a warm-caught GV100. These come from hotSpring's
experiments 184-190 (warm-catch register sequences).

### Architecture assessment

toadStool explored three potential local extraction paths:

| Path | Assessment |
|------|-----------|
| **FECS IMEM/DMEM PIO readback** | FECS PIO is write-only in-tree; PMU shows read-mode `DMEMC` pattern but FECS IMEM contents ≠ GR method entries |
| **PRAMIN / VRAM extraction** | PRAMIN reads VRAM-backed structures (PTEs, runlists, sentinels), not GR method tables |
| **`GspBridge` / firmware blob parsing** | `boot_fecs()` returns `FalconBootResult` (status registers only), not method entries; `split_for_application` is documented but not implemented |

**Conclusion**: GR method entries are proprietary NVIDIA register sequences discovered empirically
through warm-catch experimentation. hotSpring owns that data; toadStool owns the submission
infrastructure. Neither side alone completes this.

### Next step

hotSpring provides `[(u32, u32)]` method entries from experiments 184-190 → toadStool calls
`init_gr_context(entries)` on the warm-caught Titan V → hotSpring validates compute dispatch
+ readback E2E.

---

## Other toadStool Items — Status

| Item | Status | Notes |
|------|--------|-------|
| K80 Kepler dispatch | **SHIPPED** (S260) | `VfioChannel::create_kepler`, `DoorbellKind::Gk104`, `NoAcr` boot path |
| `health.version` RPC | **SHIPPED** (S260) | Returns session, version, build hash, service name |
| `health.drain` RPC | **SHIPPED** (S260) | Sets drain flag, clears readiness, rejects new dispatches |
| VFIO IPC surface | **SHIPPED** (S259) | `device.vfio.open`, `device.vfio.roundtrip` |
| QMD-based dispatch | **SHIPPED** (S259) | Generation-aware QMD builder via GPFIFO |
| Socket permissions | **SHIPPED** (S259) | `TOADSTOOL_SOCKET_MODE` env var |

---

## S261 Deep Debt Fixes

- 3 `#[expect(clippy::expect_used)]` in testing crate — added `reason`
- `#[allow(dead_code)]` in example + fuzz target — added `reason`
- `#[allow(clippy::await_holding_lock)]` in test — added `reason`
- Hardcoded primal name in `workload_routing/defaults.rs` — neutralized

---

## Metrics

| Metric | Value |
|--------|-------|
| JSON-RPC methods (direct) | 81 |
| Lib tests | 8,841 |
| Clippy warnings | 0 |
| `cargo deny check bans` | Clean |
| Production files >800L | 0 |
| Production mocks outside `cfg(test)` | 0 |
| Production TODO/FIXME/HACK | 0 |
| `#[allow]`/`#[expect]` without `reason` (production) | 0 |

---

## coralReef / barraCuda Items (Not toadStool-owned)

For completeness, the audit identified:

- **coralReef**: `SubgroupSize` builtin (5-line fix), f64 math type resolution, `SubgroupSize`/`NumSubgroups` sys_reg constants → coralReef team
- **barraCuda**: Clear — no upstream blockers
