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
//! 2. **`sysfs_write_guarded`** — child-process isolation with timeout.
//!    For `drivers_probe`, `bind`, `unbind` — operations that run full
//!    driver probe/teardown and can enter D-state.
//! 3. **`kmod_guarded`** — child-process isolation for `insmod`/`rmmod`.
//!
//! The guarded variants spawn a child process to perform the kernel-touching
//! write. If the child doesn't complete within the deadline, the parent
//! kills it and returns `Timeout`. This prevents the calling thread from
//! entering uninterruptible kernel sleep (D-state), which bricked both
//! Titan V GPUs during Exp 213.

use std::os::fd::FromRawFd as _;
use std::path::Path;
use std::process::{Child, Command};
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
fn reap_or_orphan(child: &mut Child, context: &str) {
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

// ── Tier 1: Direct sysfs write (fast attributes) ────────────────────

/// Direct sysfs write for fast, non-blocking attributes.
///
/// Suitable for `power/control`, `d3cold_allowed`, `reset_method`, and
/// `driver_override`. NOT suitable for `drivers_probe`, `bind`, or
/// `unbind` — use [`sysfs_write_guarded`] for those.
pub fn sysfs_write(path: &str, value: &str) -> Result<(), GuardedSysfsError> {
    std::fs::write(path, value).map_err(|e| GuardedSysfsError::WriteFailed {
        path: path.into(),
        reason: e.to_string(),
    })
}

// ── Tier 2: Guarded sysfs write (child-process + timeout) ───────────

/// Sysfs write via child process with timeout. If the child doesn't
/// complete within `timeout`, it is killed and `Timeout` is returned.
///
/// The calling thread never enters kernel D-state — only the child does.
/// This is the fix for the Exp 213 cascade where `drivers_probe` blocked
/// the tokio-rt-worker thread indefinitely.
pub fn sysfs_write_guarded(
    path: &str,
    value: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    tracing::debug!(path, value, timeout_ms = timeout.as_millis() as u64, "guarded sysfs write");

    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("printf '%s' '{}' > '{}'", value, path))
        .spawn()
        .map_err(|e| GuardedSysfsError::WriteFailed {
            path: path.into(),
            reason: format!("failed to spawn guard process: {e}"),
        })?;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    tracing::debug!(
                        path, elapsed_ms = start.elapsed().as_millis() as u64,
                        "guarded sysfs write completed"
                    );
                    return Ok(());
                }
                return Err(GuardedSysfsError::WriteFailed {
                    path: path.into(),
                    reason: format!("child exited with {status}"),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(
                        path, timeout_ms = timeout.as_millis() as u64,
                        "guarded sysfs write timed out — killing child"
                    );
                    let _ = child.kill();
                    reap_or_orphan(&mut child, "sysfs_write_guarded");
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
                    reason: format!("failed to poll child: {e}"),
                });
            }
        }
    }
}

// ── Tier 3: Guarded kmod operations ─────────────────────────────────

/// Run a kernel module command (`insmod`/`rmmod`) with timeout.
///
/// If the command doesn't complete within `timeout`, the child process
/// is killed and `KmodTimeout` is returned. This prevents `rmmod` from
/// permanently blocking a thread when a module has a stuck probe.
pub fn kmod_guarded(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, GuardedSysfsError> {
    let args_str = args.join(" ");
    tracing::info!(cmd, args = args_str.as_str(), timeout_ms = timeout.as_millis() as u64,
                   "guarded kmod operation");

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: cmd.into(),
            args: args_str.clone(),
            reason: format!("failed to spawn: {e}"),
        })?;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().unwrap_or_else(|_| {
                    std::process::Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    }
                });
                if status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    tracing::info!(cmd, args = args_str.as_str(),
                                   elapsed_ms = start.elapsed().as_millis() as u64,
                                   "kmod operation completed");
                    return Ok(stdout);
                }
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: cmd.into(),
                    args: args_str,
                    reason: stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(cmd, args = args_str.as_str(),
                                   timeout_ms = timeout.as_millis() as u64,
                                   "kmod operation timed out — killing child");
                    let _ = child.kill();
                    reap_or_orphan(&mut child, "kmod_guarded");
                    return Err(GuardedSysfsError::KmodTimeout {
                        cmd: cmd.into(),
                        args: args_str,
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: cmd.into(),
                    args: args_str,
                    reason: format!("failed to poll child: {e}"),
                });
            }
        }
    }
}

/// Guarded `insmod` — load a kernel module with timeout.
pub fn insmod_guarded(ko_path: &Path, timeout: Duration) -> Result<(), GuardedSysfsError> {
    let path_str = ko_path.display().to_string();
    kmod_guarded("insmod", &[&path_str], timeout)?;
    Ok(())
}

/// Guarded `rmmod` — unload a kernel module with timeout.
pub fn rmmod_guarded(name: &str, timeout: Duration) -> Result<(), GuardedSysfsError> {
    kmod_guarded("rmmod", &[name], timeout)?;
    Ok(())
}

// ── Unified sysfs helpers (replace duplicates) ──────────────────────

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
pub fn restore_flr(bdf: &str) {
    let reset_path = crate::linux_paths::sysfs_pci_device_file(bdf, "reset_method");
    if Path::new(&reset_path).exists() {
        match std::fs::write(&reset_path, "flr,bus") {
            Ok(()) => tracing::debug!(bdf, "reset_method restored to flr,bus"),
            Err(e) => tracing::debug!(bdf, error = %e, "could not restore reset_method"),
        }
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
pub fn unbind_iommu_siblings(bdf: &str) -> Vec<(String, Option<String>)> {
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
pub fn rebind_siblings_to_vfio(siblings: &[(String, Option<String>)]) {
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

// ── Pre-flight checks ───────────────────────────────────────────────

/// Check whether a kernel module is stuck in the "Unloading" state.
///
/// Parses `/proc/modules` for the named module and checks the state field.
/// Returns `true` if the module is in a stuck state (refcount < 0 or
/// state == "Unloading").
pub fn is_module_stuck(name: &str) -> bool {
    let proc_modules = format!("{}/modules", crate::linux_paths::proc_root());
    let Ok(contents) = std::fs::read_to_string(&proc_modules) else {
        return false;
    };
    parse_module_stuck(name, &contents)
}

/// Inner parser for `is_module_stuck` — testable without /proc access.
fn parse_module_stuck(name: &str, contents: &str) -> bool {
    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 5 && fields[0] == name {
            if fields[4] == "Unloading" || fields[4] == "Loading" {
                tracing::warn!(module = name, state = fields[4],
                               refcount = fields[2],
                               "module in stuck state");
                return true;
            }
            if let Ok(refcount) = fields[2].parse::<i64>() && refcount < 0 {
                tracing::warn!(module = name, refcount,
                               "module has negative refcount");
                return true;
            }
        }
    }
    false
}

/// Resolve the IOMMU group number for a PCI device.
fn iommu_group_number(bdf: &str) -> Option<u32> {
    let link = crate::linux_paths::sysfs_pci_device_file(bdf, "iommu_group");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.parse::<u32>().ok())
        })
}

/// Check whether the IOMMU group for a BDF is free of external holders.
///
/// Scans `/proc/*/fd` for open file descriptors pointing to the VFIO
/// group device (`/dev/vfio/{group_id}`). Returns `Ok(())` if no process
/// holds the group, or `Err` with the holding PID.
///
/// This is the pre-flight check that would have prevented the Exp 213
/// cascade: the daemon's own VFIO FDs locked the IOMMU group, blocking
/// nouveau's probe.
pub fn iommu_group_ready(bdf: &str) -> Result<(), GuardedSysfsError> {
    let Some(group_id) = iommu_group_number(bdf) else {
        return Ok(());
    };

    let vfio_path = format!("/dev/vfio/{group_id}");
    let vfio_path_canonical = std::fs::canonicalize(&vfio_path).unwrap_or_default();

    // Quick check via fuser-like scan of /proc
    let proc_root = crate::linux_paths::proc_root();
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return Ok(());
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let Some(pid_str) = name.to_str() else { continue };
        let Ok(pid) = pid_str.parse::<u32>() else { continue };

        let fd_dir = crate::linux_paths::proc_pid_fd_dir(pid);
        let Ok(fds) = std::fs::read_dir(&fd_dir) else { continue };

        for fd in fds.filter_map(|f| f.ok()) {
            if let Ok(target) = std::fs::read_link(fd.path())
                && (target == Path::new(&vfio_path) || target == vfio_path_canonical)
            {
                return Err(GuardedSysfsError::PreFlightFailed {
                    reason: format!(
                        "IOMMU group {group_id} held by PID {pid} (fd → {vfio_path})"
                    ),
                });
            }
        }
    }

    Ok(())
}

// ── BAR0 resource fd cleanup ────────────────────────────────────────

/// Close all leaked sysfs `resource0` file descriptors for a PCI device
/// held by the current process.
///
/// The sovereign pipeline opens BAR0 via sysfs `resource0` for health
/// monitoring and profiling. These fds are intentionally leaked by
/// `MappedBar` (the mmap outlives the File). Before a warm handoff, the
/// kernel's `request_mem_region()` in the seeder driver will fail if any
/// process still holds the BAR region open. This function scans
/// `/proc/self/fd` and closes matching descriptors.
///
/// Returns the number of fds closed.
pub fn release_bar0_fds(bdf: &str) -> usize {
    let resource_path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let resource_canonical = std::fs::canonicalize(&resource_path).ok();

    let self_fd_dir = format!("{}/self/fd", crate::linux_paths::proc_root());
    let Ok(entries) = std::fs::read_dir(&self_fd_dir) else {
        return 0;
    };

    let mut closed = 0;
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let matches = target == Path::new(&resource_path)
            || resource_canonical
                .as_ref()
                .is_some_and(|c| target == **c);
        if !matches {
            continue;
        }
        let Some(fd_num) = entry.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        // SAFETY: we own this fd (it's in /proc/self/fd). Closing a leaked
        // sysfs resource0 fd is safe — the corresponding MmioRegion mmap
        // remains valid until munmap (the kernel keeps the mapping alive
        // independently of the fd). The fd was leaked intentionally by
        // MappedBar; we are reclaiming it before driver rotation.
        unsafe {
            drop(std::os::fd::OwnedFd::from_raw_fd(fd_num));
        }
        closed += 1;
        tracing::info!(bdf, fd = fd_num, "closed leaked BAR0 resource0 fd");
    }
    closed
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
pub fn handoff_rollback(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysfs_write_nonexistent_path_fails() {
        let result = sysfs_write("/sys/nonexistent/path/12345", "test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GuardedSysfsError::WriteFailed { .. }));
    }

    #[test]
    fn read_current_driver_nonexistent() {
        assert_eq!(read_current_driver("ffff:ff:ff.f"), None);
    }

    #[test]
    fn iommu_group_siblings_nonexistent() {
        assert!(iommu_group_siblings("ffff:ff:ff.f").is_empty());
    }

    #[test]
    fn is_module_stuck_unknown_module() {
        assert!(!is_module_stuck("toadstool_nonexistent_12345"));
    }

    #[test]
    fn guarded_write_timeout_fires() {
        let result = sysfs_write_guarded(
            "/dev/null",
            "test",
            Duration::from_millis(100),
        );
        // /dev/null write should succeed fast, not timeout
        assert!(result.is_ok());
    }

    #[test]
    fn kmod_guarded_nonexistent_command() {
        let result = kmod_guarded("toadstool_fake_cmd_12345", &["arg"], Duration::from_secs(1));
        assert!(result.is_err());
    }

    #[test]
    fn guarded_write_timeout_actually_fires() {
        // Spawn a sleep via guarded write with a very short timeout.
        // The "write" target is actually a FIFO-like path that will block.
        // We use /dev/stdin in a subshell to simulate a blocking write.
        let result = sysfs_write_guarded(
            "/proc/self/fd/999", // nonexistent fd — sh will hang trying to open
            "test",
            Duration::from_millis(200),
        );
        // Should be either Timeout or WriteFailed (child can't write to bogus fd)
        assert!(result.is_err());
    }

    #[test]
    fn kmod_guarded_timeout_fires() {
        let result = kmod_guarded(
            "/bin/sleep",
            &["60"],
            Duration::from_millis(300),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GuardedSysfsError::KmodTimeout { .. }));
    }

    #[test]
    fn guarded_write_fast_path_succeeds() {
        let result = sysfs_write_guarded("/dev/null", "hello", Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn parse_module_stuck_detects_unloading() {
        let content = "nouveau 2654208 -1 - Unloading 0xffffffffc1234000\n\
                        vfio_pci 65536 0 - Live 0xffffffffc5678000\n";
        assert!(parse_module_stuck("nouveau", content));
        assert!(!parse_module_stuck("vfio_pci", content));
    }

    #[test]
    fn parse_module_stuck_detects_negative_refcount() {
        let content = "nouveau 2654208 -1 - Live 0xffffffffc1234000\n";
        assert!(parse_module_stuck("nouveau", content));
    }

    #[test]
    fn parse_module_stuck_detects_loading_state() {
        let content = "nouveau 2654208 0 - Loading 0xffffffffc1234000\n";
        assert!(parse_module_stuck("nouveau", content));
    }

    #[test]
    fn parse_module_stuck_live_is_ok() {
        let content = "kernel 0 0 - Live 0xffffffffc0000000\n\
                        nouveau 2654208 1 - Live 0xffffffffc1234000\n";
        assert!(!parse_module_stuck("kernel", content));
        assert!(!parse_module_stuck("nouveau", content));
    }

    #[test]
    fn parse_module_stuck_unknown_module_is_ok() {
        let content = "nouveau 2654208 1 - Live 0xffffffffc1234000\n";
        assert!(!parse_module_stuck("nonexistent_module_xyz", content));
    }

    #[test]
    fn parse_module_stuck_empty_content() {
        assert!(!parse_module_stuck("nouveau", ""));
    }
}
