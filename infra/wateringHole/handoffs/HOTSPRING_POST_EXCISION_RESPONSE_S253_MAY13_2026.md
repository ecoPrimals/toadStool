# Post-Excision Trio Alignment — toadStool Response (S253)

**Date**: May 13, 2026
**Session**: S253
**From**: hotSpring post-excision audit (coralReef Sprint 9 diesel engine deletion)
**Action**: All remaining Phase C gaps resolved

---

## Audit Cross-Reference

hotSpring reported 6 blocking items for Phase C. Resolution status:

| Blocking Item | Status | Session |
|--------------|--------|---------|
| `device.swap`, `device.warm_catch` — not in handler | **RESOLVED** | S252 (Batch 1) |
| `ember.mmio.*`, `ember.falcon.*` RPCs | **RESOLVED** | S252 (Batch 2: 6 methods) |
| `toadstool-cylinder` crate — doesn't exist | **RESOLVED** | S245 (153 .rs files) |
| VFIO fd holding — `Option<i32>` not `OwnedFd` | **RESOLVED** | S253 (this session) |
| SwapOrchestrator — quiesce/persist/restore stubs | **RESOLVED** | S253 (this session) |
| CLI warm-fecs/swap — no coralctl-equivalent | **RESOLVED** | S253 (this session) |
| coral-driver absorption — ~367 .rs files | **N/A** — remaining files (Intel skeleton, GSP firmware) are coralReef-domain (compiler, not hardware lifecycle) |

---

## What Shipped (S253)

### 1. VFIO fd holding: `Option<i32>` → `OwnedFd`

- `VfioResourceHandle.vfio_fd` evolved from `Option<i32>` to `Option<OwnedFd>`
- Accessor returns `Option<BorrowedFd<'_>>` (zero-cost borrow)
- Constructor accepts `OwnedFd` (RAII ownership — dropping closes the fd)
- `set_vfio_fd()` accepts `OwnedFd` (for SCM_RIGHTS receive path)
- All tests updated with real `/dev/null` fds
- File: `crates/core/ember/src/vfio_handle.rs`

### 2. SwapOrchestrator: stubs → real implementations

- **Quiesce**: Polls `gpu_busy_percent` via sysfs DRM card node; waits until idle or timeout. Reads `power_state` for pre-swap diagnostics.
- **Persist**: Writes device state JSON (personality, timestamp) to temp file. Used for crash recovery / audit trail.
- **Restore**: Verifies post-swap personality matches target. Cleans up temp state file.
- All three steps now report meaningful detail strings in the `BootStep` log.
- File: `crates/core/glowplug/src/swap.rs`

### 3. CLI device lifecycle commands (coralctl parity)

New `toadstool device` subcommand with 4 actions:

- `toadstool device swap <bdf> <target>` — full 7-step orchestrated swap via `SysfsSwapExecutor`
- `toadstool device list` — enumerate all GPU/NPU PCI devices with driver/class info
- `toadstool device status [--bdf <bdf>]` — detailed device status (vendor, device, driver, power state)
- `toadstool device warm <bdf>` — PMC_ENABLE warm detection via sysfs config space

Supports `--format json` for machine-readable output.

- Files: `crates/cli/src/commands/device.rs`, `crates/cli/src/commands/definitions.rs`, `crates/cli/src/commands/dispatch/mod.rs`, `crates/cli/src/commands/mod.rs`, `crates/cli/Cargo.toml`

---

## Quality Gates

- `cargo clippy --workspace --lib`: 0 warnings
- `cargo test --workspace --lib`: 8,827 passed, 0 failed
- All `VfioResourceHandle` tests pass with real `OwnedFd`
- All `SwapOrchestrator` 7-step tests pass with evolved quiesce/persist/restore

---

## What hotSpring Can Rewire Now

With S253 complete, **all Phase C blocking items are resolved**:

1. `fleet_client.rs` → toadStool socket paths (`TOADSTOOL_RUN_DIR`, S252)
2. `fleet_ember.rs` → toadStool MMIO/falcon RPCs (6 methods, S252)
3. `glowplug_client.rs` → toadStool client (device.swap/warm_catch, S252)
4. experiment bins → toadStool diesel discovery (device CLI, S253)
5. `capability_registry.toml` → toadStool 74 methods (S252)
6. `primal_bridge.rs` → `toadstool → [toadstool-server, toadstool-glowplug, compute]`

---

## Changed Files

- `crates/core/ember/src/vfio_handle.rs` — `OwnedFd` migration
- `crates/core/glowplug/src/swap.rs` — quiesce/persist/restore evolution
- `crates/cli/src/commands/device.rs` — new device CLI module
- `crates/cli/src/commands/definitions.rs` — `DeviceCommand` enum
- `crates/cli/src/commands/dispatch/mod.rs` — device dispatch routing
- `crates/cli/src/commands/mod.rs` — module registration
- `crates/cli/Cargo.toml` — glowplug + ember dependencies
