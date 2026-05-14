# hotSpring Sovereign Compute Trio Evolution — toadStool Response S260

**Date**: May 14, 2026
**Audit**: hotSpring → toadStool: Sovereign Compute Trio Evolution — May 14, 2026
**Responding primal**: toadStool
**Session**: S260

---

## Audit Items — Resolution Status

### 1. FECS compute context init for GV100 (Titan V) — PARTIALLY RESOLVED

**What hotSpring asked**: Wire context slot allocation + GR init writes into `NvVfioComputeDevice`.

**What toadStool shipped**:

- **`NvVfioComputeDevice::init_gr_context(method_entries)`**: Submits GR context init method entries via pushbuffer on subchannel 0. This is the infrastructure to receive warm-catch GR method entries from hotSpring experiments 184-190.
- Method entries are `(addr, value)` pairs for GR class methods. `PushBuf::gr_context_init` encodes them with the correct compute class from the generation profile.

**What remains**:
- The warm-catch cycle (nouveau NOP-patch → VFIO bind → toadStool catch) is working. The missing piece is the **source of GR context method entries** — which hotSpring has documented in experiments 184-190. Three paths:
  - **(a) Warm-handoff method replay**: hotSpring captures FECS method entries during the warm phase and provides them to `init_gr_context()` before first dispatch.
  - **(b) `compute.firmware.load` RPC**: A future endpoint that accepts FECS firmware state and replays SEC2/ACR boot.
  - **(c) Local FECS absorption**: toadStool reads FECS IMEM/DMEM state from BAR0 and replays independently.
- Path (a) is closest — `init_gr_context()` is ready to receive entries now.

### 2. K80 Kepler dispatch — RESOLVED

**What hotSpring asked**: Confirm Kepler PM4-style submissions are wired.

**What toadStool shipped**:

- **`NvVfioComputeDevice::open_vfio()` generation branching**: Detects Kepler GPUs via `PageTableFormat::V1TwoLevel` and creates a Kepler-specific PFIFO channel via `VfioChannel::create_kepler` (2-level PTs, GK104 global runlist, simple instance block).
- **`DoorbellKind` enum**: `submit_pushbuffer()` now selects the correct doorbell — `NOTIFY_CHANNEL_PENDING` (BAR0 0x81_0090) for Volta+, `gk104_doorbell(channel_id)` (BAR0 0x3000 + ch×8) for Kepler.
- **`try_vfio_nvidia()` factory**: Recognizes Kepler `NoAcr` devices by checking `device_name` after `probe_warm_fecs()`. Kepler devices are marked `fecs_ready = true` without needing a warm FECS handoff (FECS boots via PIO).
- **QMD builder**: Already handles SM37 via `QmdVersion::V21` with `KEPLER_COMPUTE_B` class (0xA1C0) and `LaunchMethod::Pcas`.

**K80-specific kernel state assumptions**:
- K80 must be in D0 power state (not D3cold — PLX PEX 8747 wedges in D3cold)
- GDDR5 must be trained (5 GPCs active)
- FECS boots via PIO without ACR — no warm handoff needed
- Doorbell: GK104 per-channel at 0x3000 + ch×8

### 3. `health.version` RPC method — RESOLVED

**What hotSpring asked**: Return `{ "session": "S260", "build_hash": "abc123" }` for upgrade verification.

**What toadStool shipped**:

- **`health.version`** direct JSON-RPC method returning:
  ```json
  {
    "version": "0.1.0",
    "session": "0.1.0",
    "build_hash": "dev",
    "service": "toadstool"
  }
  ```
- Build hash populated via `option_env!("GIT_HASH")` — CI/CD sets `GIT_HASH` env var at compile time; local builds show `"dev"`.
- Classified as public (no auth gate) — matches other health probes.
- Semantic aliases: `ember.health.version`, `sovereign.health.version`.
- Wire L3 cost: `negligible` (pure in-memory, no I/O).

### 4. `health.drain` RPC method — RESOLVED

**What hotSpring asked**: Stop accepting new dispatches, wait for in-flight work, signal ready for shutdown.

**What toadStool shipped**:

- **`health.drain`** direct JSON-RPC method that:
  - Sets `draining` `AtomicBool` flag on the handler
  - Clears the `ready` flag (so `health.liveness` returns `"starting"`)
  - Returns `{ "status": "draining", "accepting_new_work": false, "message": "..." }`
- After drain, the caller can send SIGTERM for clean shutdown.
- Classified as public — upgrade scripts need to call this without auth.
- Semantic aliases: `ember.health.drain`, `sovereign.health.drain`.

---

## Validation Points for hotSpring

1. **`health.version`**: Call `health.version` after upgrade → verify `build_hash` matches deployed binary.
2. **`health.drain`**: Call `health.drain` → verify `accepting_new_work: false` → SIGTERM → verify clean shutdown.
3. **K80 dispatch**: Once local ecoBin is updated, `device.vfio.open` with K80 BDF should detect Kepler, create GK104 channel, and return `status: "ready"`.
4. **FECS context init**: Call `init_gr_context()` programmatically with method entries from experiments 184-190 before first compute dispatch on warm-caught Titan V.

---

## Metrics

- **81 JSON-RPC methods** (direct) — +2 from S259 (`health.version`, `health.drain`)
- **8,841 lib tests** — +4 from S259
- 0 clippy warnings, deny clean
- Kepler dispatch path wired with GK104 doorbell and 2-level page tables

---

## Cross-trio notes

### For coralReef team
- `deformed_wavefunction_f64.wgsl` type error and `health.version` noted — these are coralReef items, not toadStool.

### For barraCuda team
- `stats.entropy` (GAP-HS-041) noted — this is barraCuda's item.
- No remaining thermal niche blockers from toadStool's side.
