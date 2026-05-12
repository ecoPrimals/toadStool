# ToadStool S239 — Wave 8 Phase B: Glowplug Absorption

**Date**: May 12, 2026
**Commit**: (pending)
**Previous**: S238 (deep debt — magic numbers, JH-2 audit)

---

## Summary

Phase B of the Compute Trio diesel engine absorption. `coral-glowplug`'s
sovereign boot lifecycle, swap orchestration, and personality management
absorbed into toadStool. The `EmberClient` cross-process Unix socket IPC
pattern is replaced by toadStool-internal `SwapOrchestrator<SysfsSwapExecutor>`.
`GpuPersonality` unified with `NvidiaOracle` and `Akida` variants from
`coral-glowplug`'s `Personality` enum.

---

## What Changed

### New: `toadstool-glowplug/src/boot.rs`
- `BootResult` — full sovereign boot outcome (device_id, initial/final personality,
  warm_cycle_performed, steps, success, summary)
- `BootStep` — per-step record (name, status, detail, duration_ms)
- `StepStatus` — `Ok | Skipped | Failed`
- Convenience constructors: `BootResult::failed()`, `BootStep::ok/skipped/failed()`
- Serde roundtrip tests, constructor tests

### New: `toadstool-glowplug/src/sysfs_executor.rs`
- `SysfsSwapExecutor` — first production `SwapExecutor` implementation
- Performs PCI driver unbind/rebind via `/sys/bus/pci/drivers/*/unbind`,
  `driver_override`, and `drivers_probe`
- `SysfsSwapError` enum: `NotPciBdf`, `SysfsWrite { path, reason }`,
  `BindFailed { bdf, driver }`
- Driver name mapping (e.g. "vfio" → "vfio-pci", "akida" → "akida-pcie")
- Replaces `coral-glowplug`'s `EmberClient` cross-process IPC pattern

### Modified: `toadstool-glowplug/src/swap.rs`
- `SwapOrchestrator::orchestrate_swap()` — 7-step lifecycle:
  1. Quiesce — wait for in-flight operations
  2. Persist — snapshot device state
  3. Drop — release current handle
  4. Delegate — execute bus-specific swap
  5. Reacquire — record new observation
  6. Restore — replay persisted state
  7. Health — verify device healthy
- `SwapOrchestrator::execute_boot()` — high-level entry combining driver
  detection with orchestrate_swap
- Returns `BootResult` with per-step timing and status
- 5 new async tests: success 7-step, failure stops at delegate, boot delegation

### Modified: `GpuPersonality` (runtime/gpu/src/glowplug/personality.rs)
- Added `NvidiaOracle { module_name: String }` variant — custom/experimental
  kernel modules (matches coral-glowplug's `Personality::NvidiaOracle`)
- Added `Akida` variant — BrainChip neuromorphic accelerator
  (capabilities: `["neuromorphic", "inference"]`)
- `GpuPersonalityRegistry` handles `nvidia_oracle_*` prefix matching and
  `akida`/`akida-pcie` aliases
- 5 new tests: oracle name+module, akida capabilities, oracle prefix, akida
  aliases, all variants have driver modules

### Modified: `GlowPlugClient` (server/src/glowplug_client.rs)
- Now wraps `SwapOrchestrator<SysfsSwapExecutor>` internally
- `swap_device_orchestrated()` — lifecycle-managed swap via orchestrator
- `orchestrator()` accessor for downstream use
- Legacy `swap_device()` retained for backward compatibility
- `read_current_driver()` helper for sysfs driver readlink

### Modified: `dispatch/capabilities.rs`
- `ember.phase` updated from `"A"` to `"B"`
- New `glowplug` section: `orchestrator`, `lifecycle_steps: 7`, `personalities[]`

### Modified: `dispatch/tests/trio_contract.rs`
- Phase assertion updated: `ember.phase == "B"`
- 2 new tests: `capabilities_includes_glowplug_info`,
  `capabilities_glowplug_has_orchestrator_type`

---

## Test Results

- **8,278** lib-only tests passing (0 failures)
- **62** `toadstool-glowplug` tests (boot: 7, sysfs_executor: 4, swap: 12+)
- **115** dispatch tests (including 2 new Phase B)
- `cargo clippy --workspace -- -D warnings` — zero warnings

---

## Dependency Changes

- `crates/server/Cargo.toml`: Added `toadstool-glowplug` dependency
- No new external dependencies

---

## Architecture After Phase B

```
ecosystem → JSON-RPC IPC → toadStool server
                               ├─ DispatchHandler
                               │    ├─ device_pool (VfioResourceHandle)  [Phase A]
                               │    └─ acquire_device_handle()
                               ├─ GlowPlugClient
                               │    ├─ SwapOrchestrator<SysfsSwapExecutor>  [Phase B]
                               │    ├─ swap_device_orchestrated() → 7-step lifecycle
                               │    ├─ list_devices() → sysfs PCI enumeration
                               │    └─ status() → held devices + uptime
                               └─ glowPlug crate
                                    ├─ BootResult / BootStep / StepStatus
                                    ├─ SwapOrchestrator<E: SwapExecutor>
                                    ├─ SysfsSwapExecutor (production)
                                    ├─ DeviceSlot<H, P>
                                    ├─ DevicePersonality + GpuPersonality
                                    │    └─ NvidiaOracle + Akida variants
                                    └─ ember subsystem
                                         ├─ VfioResourceHandle
                                         ├─ vendor lifecycle
                                         └─ sysfs abstraction
```

---

## What's Next

- **Phase C**: cylinder + coral-driver hardware — generalize per-device subprocess
- **Phase D**: local dispatch (Gate 4) — execute locally via absorbed driver layer
- **Coverage push**: 83.6% → 90% (hardware mocks needed)

---

## For coralReef

Phase B absorption is complete. `coral-glowplug`'s portable components (boot,
swap, personality, sysfs) are now in toadStool. You can begin **soft-deprecating
`coral-glowplug`** — don't remove until toadStool confirms Phase C coverage of
cylinder + coral-driver hardware-specific code.
