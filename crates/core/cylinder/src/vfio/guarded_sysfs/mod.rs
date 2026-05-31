// SPDX-License-Identifier: AGPL-3.0-or-later
//! Guarded sysfs I/O layer — unified, timeout-safe PCI sysfs operations.
//!
//! Replaces the four duplicate `sysfs_write` / `read_current_driver` /
//! `pin_bridge_hierarchy` / `disable_flr` implementations scattered across
//! `sovereign_handoff`, `SysfsSwapExecutor`, `ember::sysfs`, and
//! `nvpmu::vfio_bind`.
//!
//! Three tiers of write safety:
//!
//! 1. **`sysfs_write`** — direct `std::fs::write`, for fast attributes
//!    (power/control, d3cold_allowed, reset_method).
//! 2. **`sysfs_write_guarded`** — fork + `open(O_WRONLY)` + `write()` with
//!    timeout. For `drivers_probe`, `bind`, `unbind` — operations that run
//!    full driver probe/teardown and can enter D-state.
//! 3. **`insmod_guarded`/`rmmod_guarded`** — fork + `finit_module(2)` /
//!    `delete_module(2)` syscalls with timeout.
//!
//! The guarded variants spawn a child process to perform the kernel-touching
//! write. If the child doesn't complete within the deadline, the parent
//! kills it and returns `Timeout`. This prevents the calling thread from
//! entering uninterruptible kernel sleep (D-state), which bricked both
//! Titan V GPUs during Exp 213.

use std::process::Child;
use std::time::{Duration, Instant};

/// How long to poll a killed child before orphaning it.
const REAP_POLL_CAP: Duration = Duration::from_secs(2);

/// Default timeout for `drivers_probe` / `bind` / `unbind` operations.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// Default timeout for driver unbind operations.
pub const UNBIND_TIMEOUT: Duration = Duration::from_secs(10);
/// Default timeout for `insmod` operations.
pub const INSMOD_TIMEOUT: Duration = Duration::from_secs(15);
/// Default timeout for `rmmod` operations.
pub const RMMOD_TIMEOUT: Duration = Duration::from_secs(10);
/// Extended timeout for nvidia RM teardown during catalyst unbind.
/// nvidia-470's RM on GV100 takes ~160s to fully teardown (HBM2 dealloc,
/// falcon shutdown, FECS/GPCCS halt). Must exceed this or the child gets
/// killed and the probe/rebind races with still-running kernel teardown.
pub const CATALYST_TEARDOWN_TIMEOUT: Duration = Duration::from_secs(200);
/// Default overall handoff deadline.
/// 400s for catalyst: 15s settle + 160s RM teardown + 30s BAR0 capture.
pub const HANDOFF_DEADLINE: Duration = Duration::from_secs(400);

/// Errors from guarded sysfs operations.
#[derive(Debug, thiserror::Error)]
pub enum GuardedSysfsError {
    #[error("sysfs write to {path}: {reason}")]
    WriteFailed { path: String, reason: String },

    #[error("sysfs write to {path} timed out after {timeout_ms}ms")]
    Timeout { path: String, timeout_ms: u64 },

    #[error("child process killed by signal for {path}")]
    ChildKilled { path: String },

    #[error("kmod {cmd} {args}: {reason}")]
    KmodFailed {
        cmd: String,
        args: String,
        reason: String,
    },

    #[error("kmod {cmd} {args} timed out after {timeout_ms}ms")]
    KmodTimeout {
        cmd: String,
        args: String,
        timeout_ms: u64,
    },

    #[error("pre-flight check failed: {reason}")]
    PreFlightFailed { reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Poll a killed child for up to [`REAP_POLL_CAP`], then orphan if still alive.
///
/// After `kill()`, the child *should* exit quickly. But if it's in kernel
/// D-state, `wait()` blocks the parent thread indefinitely — exactly the
/// scenario the guard is designed to prevent. Instead we poll with
/// `try_wait()` up to the cap, then log and detach (accept the zombie).
pub(super) fn reap_or_orphan(child: &mut Child, context: &str) {
    let start = Instant::now();
    let interval = Duration::from_millis(100);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if start.elapsed() >= REAP_POLL_CAP => {
                tracing::warn!(
                    context,
                    pid = child.id(),
                    "killed child still alive after {}ms — orphaning (zombie expected)",
                    REAP_POLL_CAP.as_millis(),
                );
                return;
            }
            Ok(None) => std::thread::sleep(interval),
            Err(_) => return,
        }
    }
}

mod driver_ops;
mod kmod_build;
mod proc_scan;

#[cfg(test)]
mod tests;

pub use driver_ops::{
    disable_flr, iommu_group_siblings, pin_bridge_hierarchy, prepare_anchor_release,
    read_current_driver, sysfs_write, sysfs_write_guarded,
};
pub(crate) use driver_ops::{
    handoff_rollback, rebind_siblings_to_vfio, restore_flr, sysfs_unbind_fire_and_poll,
    unbind_iommu_siblings,
};
pub use kmod_build::{
    KmodBuilder, disengage_irq_clutch, engage_irq_clutch, insmod_guarded,
    insmod_guarded_with_params, kmod_guarded, restore_bus_reset, rmmod_guarded,
    suppress_bus_reset, unsuppress_bus_reset_for,
};
pub use proc_scan::{ModuleSnapshot, iommu_group_ready, is_module_stuck, module_snapshot, release_bar0_fds};
