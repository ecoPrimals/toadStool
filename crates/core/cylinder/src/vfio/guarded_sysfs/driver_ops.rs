// SPDX-License-Identifier: AGPL-3.0-or-later
//! Guarded sysfs write operations — bind, unbind, driver_override, drivers_probe.

use std::ffi::CString;
use std::path::Path;
use std::time::{Duration, Instant};

use super::GuardedSysfsError;
use super::kmod_build::{rmmod_guarded, suppress_bus_reset, unsuppress_bus_reset_for};
use super::{REAP_POLL_CAP, RMMOD_TIMEOUT, UNBIND_TIMEOUT};

pub fn sysfs_write(path: &str, value: &str) -> Result<(), GuardedSysfsError> {
    std::fs::write(path, value).map_err(|e| GuardedSysfsError::WriteFailed {
        path: path.into(),
        reason: e.to_string(),
    })
}

/// Fork a child that opens `path` and writes `value` to it.
///
/// The child is async-signal-safe: CStrings are prepared before fork,
/// the child only calls open/write/close/exit_group. If the write
/// enters D-state (e.g. `drivers_probe` blocking on driver init),
/// the parent kills the child after `timeout`.
///
/// Returns the child PID (for fire-and-forget callers) or waits for
/// completion (for synchronous callers).
fn fork_sysfs_child(
    path_c: &CString,
    value: &[u8],
) -> Result<rustix::process::Pid, GuardedSysfsError> {
    let path_str = path_c.to_string_lossy();

    // SAFETY: fork in multi-threaded context. The child only calls
    // open/write/close/exit_group — all async-signal-safe.
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::WriteFailed {
            path: path_str.into_owned(),
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            use rustix::fs::{open, Mode, OFlags};
            let fd = match open(path_c.as_c_str(), OFlags::WRONLY, Mode::empty()) {
                Ok(fd) => fd,
                Err(e) => {
                    let code = e.raw_os_error();
                    rustix::runtime::exit_group(code.min(255) as u8 as i32)
                },
            };
            let _ = rustix::io::write(&fd, value);
            drop(fd);
            rustix::runtime::exit_group(0)
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => Ok(child_pid),
    }
}

/// Wait for a forked child with timeout, kill on timeout.
fn wait_for_child(
    child_pid: rustix::process::Pid,
    path: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    use rustix::process::{Signal, WaitOptions, waitpid};

    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    loop {
        match waitpid(Some(child_pid), WaitOptions::NOHANG) {
            Ok(Some((_pid, status))) => {
                if status.exited() && status.exit_status() == Some(0) {
                    tracing::debug!(
                        path, elapsed_ms = start.elapsed().as_millis() as u64,
                        "guarded sysfs write completed"
                    );
                    return Ok(());
                }
                let code = status.exit_status().unwrap_or(-1);
                return Err(GuardedSysfsError::WriteFailed {
                    path: path.into(),
                    reason: format!("child exited with code {code}"),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(
                        path, timeout_ms = timeout.as_millis() as u64,
                        "guarded sysfs write timed out — killing child"
                    );
                    let _ = rustix::process::kill_process(child_pid, Signal::KILL);
                    reap_forked_child(child_pid);
                    return Err(GuardedSysfsError::Timeout {
                        path: path.into(),
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(GuardedSysfsError::WriteFailed {
                    path: path.into(),
                    reason: format!("waitpid failed: {e}"),
                });
            }
        }
    }
}

/// Non-blocking reap of a killed forked child. If the child is in D-state,
/// SIGKILL won't take effect until the kernel code returns — a blocking
/// waitpid would deadlock us too. Poll briefly, then abandon the zombie.
pub(super) fn reap_forked_child(child_pid: rustix::process::Pid) {
    use rustix::process::{WaitOptions, waitpid};

    let deadline = Instant::now() + REAP_POLL_CAP;
    loop {
        match waitpid(Some(child_pid), WaitOptions::NOHANG) {
            Ok(Some(_)) => return,
            Ok(None) => {
                if Instant::now() >= deadline {
                    tracing::warn!(
                        pid = child_pid.as_raw_nonzero().get(),
                        "fork guard: child stuck in D-state after SIGKILL — abandoning zombie"
                    );
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return,
        }
    }
}

/// Sysfs write via forked child with timeout. If the child doesn't
/// complete within `timeout`, it is killed and `Timeout` is returned.
///
/// The calling thread never enters kernel D-state — only the child does.
/// This is the fix for the Exp 213 cascade where `drivers_probe` blocked
/// the tokio-rt-worker thread indefinitely.
///
/// Phase 3: pure Rust fork+write — no `/bin/sh`, no shell quoting.
pub fn sysfs_write_guarded(
    path: &str,
    value: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    tracing::debug!(path, value, timeout_ms = timeout.as_millis() as u64, "guarded sysfs write");

    let path_c = CString::new(path).map_err(|_| GuardedSysfsError::WriteFailed {
        path: path.into(),
        reason: "path contains NUL byte".into(),
    })?;

    let child_pid = fork_sysfs_child(&path_c, value.as_bytes())?;
    wait_for_child(child_pid, path, timeout)
}

/// Fire-and-forget unbind with driver-state polling.
///
/// For nvidia catalyst teardown, the kernel-side `remove` callback takes
/// 160-400s (HBM2 dealloc, falcon halt). `sysfs_write_guarded` would block
/// the calling thread for the entire duration. Instead:
///   1. Fork a child to write the unbind (returns immediately to parent)
///   2. Poll `read_current_driver` every 2s until driver clears
///   3. The child stays alive in kernel D-state — we don't wait for it
///
/// This keeps ember responsive during the entire teardown.
///
/// Phase 3: pure Rust fork+write — no `/bin/sh`.
pub(crate) fn sysfs_unbind_fire_and_poll(
    bdf: &str,
    driver: &str,
    deadline: Duration,
) -> Result<Duration, GuardedSysfsError> {
    let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(driver);
    tracing::info!(
        bdf, driver, deadline_s = deadline.as_secs(),
        "fire-and-poll unbind: initiating driver teardown"
    );

    let path_c = CString::new(unbind_path.as_str()).map_err(|_| GuardedSysfsError::WriteFailed {
        path: unbind_path.clone(),
        reason: "path contains NUL byte".into(),
    })?;

    let child_pid = fork_sysfs_child(&path_c, bdf.as_bytes())?;

    let start = Instant::now();
    let poll_interval = Duration::from_secs(2);

    loop {
        if read_current_driver(bdf).is_none() {
            let symlink_elapsed = start.elapsed();
            tracing::info!(
                bdf, elapsed_s = symlink_elapsed.as_secs(),
                "fire-and-poll unbind: driver symlink cleared"
            );

            // The driver symlink is removed by driver_sysfs_remove() BEFORE
            // the driver's .remove callback runs. The child's write() syscall
            // won't return until device_release_driver() completes (including
            // .remove and device_lock release). Wait for the child to exit so
            // the device_lock is guaranteed free for subsequent sysfs ops.
            let child_deadline = deadline.saturating_sub(symlink_elapsed);
            let child_start = Instant::now();
            loop {
                match rustix::process::waitpid(
                    Some(child_pid),
                    rustix::process::WaitOptions::NOHANG,
                ) {
                    Ok(Some(_status)) => {
                        let total = start.elapsed();
                        tracing::info!(
                            bdf, elapsed_s = total.as_secs(),
                            remove_ms = (total - symlink_elapsed).as_millis() as u64,
                            "fire-and-poll unbind: child exited (device_lock released)"
                        );
                        return Ok(total);
                    }
                    Ok(None) => {
                        if child_start.elapsed() >= child_deadline {
                            tracing::warn!(
                                bdf,
                                "fire-and-poll: child still in .remove after deadline — \
                                 proceeding (device_lock may still be held)"
                            );
                            return Ok(start.elapsed());
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                    Err(_) => {
                        return Ok(start.elapsed());
                    }
                }
            }
        }

        if start.elapsed() >= deadline {
            tracing::error!(
                bdf, deadline_s = deadline.as_secs(),
                "fire-and-poll unbind: deadline exceeded — device still bound"
            );
            return Err(GuardedSysfsError::Timeout {
                path: unbind_path,
                timeout_ms: deadline.as_millis() as u64,
            });
        }

        std::thread::sleep(poll_interval);
    }
}

/// Read the current driver name for a PCI device via its sysfs symlink.
///
/// Replaces 4 duplicate implementations across sovereign_handoff,
/// SysfsSwapExecutor, glowplug_client, and nvpmu::vfio_bind.
pub fn read_current_driver(bdf: &str) -> Option<String> {
    let link = crate::linux_paths::sysfs_pci_device_file(bdf, "driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Walk the sysfs device path upward, pinning `power/control=on` and
/// `d3cold_allowed=0` on every ancestor PCI bridge, plus the device itself.
///
/// Prevents PLX (and similar PCIe switch) bridges from entering D3cold
/// when the downstream endpoint is unbound — critical for the Tesla K80
/// whose PLX PEX 8747 fabric goes dark instantly on unbind.
///
/// Replaces 3 duplicate implementations across sovereign_handoff,
/// SysfsSwapExecutor, and ember::sysfs.
pub fn pin_bridge_hierarchy(bdf: &str) {
    let device_link = crate::linux_paths::sysfs_pci_device_path(bdf);
    let Ok(canonical) = std::fs::canonicalize(&device_link) else {
        return;
    };

    let mut current = canonical.as_path();
    while let Some(parent) = current.parent() {
        let power_control = parent.join("power/control");
        if power_control.exists() {
            let _ = std::fs::write(&power_control, "on");
        }
        let d3cold = parent.join("d3cold_allowed");
        if d3cold.exists() {
            let _ = std::fs::write(&d3cold, "0");
        }

        if !parent.join("vendor").exists() {
            break;
        }
        current = parent;
    }

    // Also pin the endpoint device itself (SysfsSwapExecutor does this,
    // sovereign_handoff previously did not).
    let control = crate::linux_paths::sysfs_pci_device_file(bdf, "power/control");
    let d3cold = crate::linux_paths::sysfs_pci_device_file(bdf, "power/d3cold_allowed");
    let _ = std::fs::write(&control, "on");
    let _ = std::fs::write(&d3cold, "0");
}

/// Disable Function Level Reset for warm-preserving swaps.
///
/// Clearing `reset_method` before a driver swap prevents the kernel from
/// triggering FLR, which destroys the warm state (PRI Ring, clock trees,
/// memory training) set up by the seeder driver.
pub fn disable_flr(bdf: &str) {
    let reset_path = crate::linux_paths::sysfs_pci_device_file(bdf, "reset_method");
    if Path::new(&reset_path).exists() {
        match std::fs::write(&reset_path, "") {
            Ok(()) => tracing::debug!(bdf, "FLR disabled (reset_method cleared)"),
            Err(e) => tracing::warn!(bdf, error = %e, "failed to clear reset_method"),
        }
    }
}

/// Re-enable default reset methods after a swap is complete.
pub(crate) fn restore_flr(bdf: &str) {
    let reset_path = crate::linux_paths::sysfs_pci_device_file(bdf, "reset_method");
    if Path::new(&reset_path).exists() {
        match std::fs::write(&reset_path, "flr,bus") {
            Ok(()) => tracing::debug!(bdf, "reset_method restored to flr,bus"),
            Err(e) => tracing::debug!(bdf, error = %e, "could not restore reset_method"),
        }
    }
}

/// Prepare a device for VFIO anchor release.
///
/// Must be called BEFORE dropping the `VfioAnchor`. Three layers of defense:
///
/// 1. Pin bridge power hierarchy (prevent D3cold)
/// 2. Clear `reset_method` to suppress per-device FLR/PM reset (Exp 225)
/// 3. (Conditional) Load `no_bus_reset.ko` to set `PCI_DEV_FLAGS_NO_BUS_RESET`,
///    preventing the kernel's dev_set `pci_reset_bus()` SBR (Exp 226)
///
/// Without layers 1+2, `vfio_pci_core_release()` fires per-device FLR.
/// Without layer 3, `vfio_pci_dev_set_try_reset()` fires bus-level SBR
/// when all devices in the dev_set have open_count==0.
///
/// When `suppress_sbr` is false (cold GPU + catalyst pipeline), SBR is
/// intentionally allowed so the GPU gets a clean PCIe reset. RM needs a
/// post-reset state to run DEVINIT on cold hardware. The pipeline then
/// calls [`suppress_bus_reset`] later, after RM init completes, to
/// protect the newly-warm state during the warm swap (Exp 229).
pub fn prepare_anchor_release(bdf: &str, suppress_sbr: bool) {
    tracing::info!(
        bdf, suppress_sbr,
        "preparing anchor release: pinning bridges + disabling FLR{}",
        if suppress_sbr { " + suppressing SBR" } else { " (SBR allowed for cold DEVINIT)" }
    );
    pin_bridge_hierarchy(bdf);
    disable_flr(bdf);
    for sib in iommu_group_siblings(bdf) {
        disable_flr(&sib);
    }
    if suppress_sbr {
        if let Err(e) = suppress_bus_reset(bdf) {
            tracing::error!(bdf, error = %e, "failed to suppress bus reset — SBR may destroy warm state");
        }
    } else if let Err(e) = unsuppress_bus_reset_for(bdf) {
        tracing::error!(bdf, error = %e, "failed to unsuppress bus reset for catalyst SBR");
    }
}

/// Discover IOMMU group siblings (other PCI functions sharing the group).
///
/// Returns BDFs of sibling devices (excludes the target BDF itself).
/// On NVIDIA GPUs, function 1 is typically the HD Audio controller.
pub fn iommu_group_siblings(bdf: &str) -> Vec<String> {
    let group_link = crate::linux_paths::sysfs_pci_device_file(bdf, "iommu_group/devices");
    let Ok(entries) = std::fs::read_dir(&group_link) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name != bdf { Some(name) } else { None }
        })
        .collect()
}

/// Unbind all IOMMU group siblings from their current drivers.
///
/// Returns the list of (sibling_bdf, previous_driver) pairs for rollback.
pub(crate) fn unbind_iommu_siblings(bdf: &str) -> Vec<(String, Option<String>)> {
    let siblings = iommu_group_siblings(bdf);
    let mut results = Vec::new();
    for sibling in &siblings {
        let prev = read_current_driver(sibling);
        if let Some(ref drv) = prev {
            let unbind = crate::linux_paths::sysfs_pci_driver_unbind(drv);
            match sysfs_write_guarded(&unbind, sibling, UNBIND_TIMEOUT) {
                Ok(()) => tracing::debug!(bdf = sibling.as_str(), driver = drv.as_str(),
                                          "IOMMU sibling unbound (guarded)"),
                Err(e) => tracing::warn!(bdf = sibling.as_str(), driver = drv.as_str(),
                                         error = %e, "IOMMU sibling unbind failed (guarded)"),
            }
        }
        results.push((sibling.clone(), prev));
    }
    results
}

/// Rebind IOMMU group siblings to vfio-pci after the handoff completes.
pub(crate) fn rebind_siblings_to_vfio(siblings: &[(String, Option<String>)]) {
    for (sibling, _) in siblings {
        let override_path = crate::linux_paths::sysfs_pci_device_file(sibling, "driver_override");
        let _ = sysfs_write(&override_path, "vfio-pci");
        let probe_path = crate::linux_paths::sysfs_pci_drivers_probe();
        match sysfs_write_guarded(&probe_path, sibling, Duration::from_secs(5)) {
            Ok(()) => tracing::debug!(bdf = sibling.as_str(), "IOMMU sibling rebound to vfio-pci"),
            Err(e) => tracing::warn!(bdf = sibling.as_str(), error = %e,
                                     "IOMMU sibling vfio-pci rebind failed"),
        }
    }
}

// ── Handoff rollback ────────────────────────────────────────────────

/// Attempt best-effort rollback after a failed handoff.
///
/// Tries to restore the system to a usable state:
/// 1. Guarded rmmod of the loaded module (with timeout)
/// 2. Clear driver_override on the target device
/// 3. Rebind target to vfio-pci
/// 4. Rebind IOMMU siblings to vfio-pci
///
/// When `device_poisoned` is true the device is assumed to be locked
/// by a D-state kernel thread (e.g. a stuck insmod probe). All sysfs
/// operations on the **target device** are skipped because they would
/// cascade the D-state to ember's own thread. Only sibling rebinding
/// is attempted. The device is effectively sacrificed until reboot.
pub(crate) fn handoff_rollback(
    bdf: &str,
    module_name: Option<&str>,
    siblings: &[(String, Option<String>)],
    device_poisoned: bool,
) {
    if device_poisoned {
        tracing::error!(bdf,
            "handoff rollback: device POISONED (D-state) — skipping all \
             sysfs ops on target to protect ember. Device is lost until reboot.");

        if let Some(name) = module_name {
            tracing::warn!(bdf, module = name,
                "rollback: skipping rmmod (device poisoned, module likely stuck)");
        }

        if !siblings.is_empty() {
            rebind_siblings_to_vfio(siblings);
            tracing::info!(bdf, count = siblings.len(),
                "rollback: siblings rebound (device itself abandoned)");
        }
        return;
    }

    tracing::warn!(bdf, "handoff rollback: attempting recovery");

    // 1. Try to unload the module if we loaded it
    if let Some(name) = module_name
        && crate::vfio::kmod::is_module_loaded(name)
    {
        tracing::info!(module = name, "rollback: attempting guarded rmmod");
        match rmmod_guarded(name, RMMOD_TIMEOUT) {
            Ok(()) => tracing::info!(module = name, "rollback: rmmod succeeded"),
            Err(e) => tracing::warn!(module = name, error = %e,
                                     "rollback: rmmod failed (module may be stuck)"),
        }
    }

    // 2–3. Clear driver_override and rebind — use guarded writes to
    //      avoid D-state cascade if the device is partially stuck.
    let override_path = crate::linux_paths::sysfs_pci_device_file(bdf, "driver_override");
    let _ = sysfs_write_guarded(&override_path, "", UNBIND_TIMEOUT);
    let _ = sysfs_write_guarded(&override_path, "vfio-pci", UNBIND_TIMEOUT);

    let probe_path = crate::linux_paths::sysfs_pci_drivers_probe();
    match sysfs_write_guarded(&probe_path, bdf, Duration::from_secs(5)) {
        Ok(()) => tracing::info!(bdf, "rollback: target rebound to vfio-pci"),
        Err(e) => tracing::warn!(bdf, error = %e, "rollback: target vfio-pci rebind failed"),
    }

    // 4. Rebind siblings
    if !siblings.is_empty() {
        rebind_siblings_to_vfio(siblings);
        tracing::info!(bdf, count = siblings.len(), "rollback: siblings rebound");
    }
}
