// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

/// Trigger nvidia RM's GPU initialization by opening its dynamically-assigned chardev.
///
/// After the catalyst module loads with `__register_chrdev(0, ...)` (dynamic major),
/// we read `/proc/devices` to find the assigned major, create a temporary device node,
/// and open+close it. This triggers `nv_open()` → `nv_start_device()` → full RM init
/// (SEC2 → ACR → FECS → GPCCS → TPC PRI station creation).
/// Trigger nvidia RM's GPU initialization by opening its dynamically-assigned chardev.
///
/// After the catalyst module loads with `__register_chrdev(0, ...)` (dynamic major),
/// we read `/proc/devices` to find the assigned major, create a temporary device node,
/// and open+close it. This triggers `nv_open()` → `nv_start_device()` → full RM init
/// (SEC2 → ACR → FECS → GPCCS → TPC PRI station creation).
pub(crate) fn trigger_rm_init(module_name: &str) -> Result<String, String> {
    let devices = std::fs::read_to_string("/proc/devices")
        .map_err(|e| format!("failed to read /proc/devices: {e}"))?;
    let mut majors: Vec<u32> = Vec::new();
    for line in devices.lines() {
        let line = line.trim();
        if line.ends_with("nvidia-frontend") || line.ends_with(module_name) {
            if let Some(num_str) = line.split_whitespace().next() {
                if let Ok(n) = num_str.parse::<u32>() {
                    majors.push(n);
                }
            }
        }
    }
    let major = majors.iter()
        .copied()
        .max()
        .ok_or_else(|| format!(
            "{module_name} chardev not found in /proc/devices — \
             __register_chrdev may have been NOPed"
        ))?;

    tracing::info!(module_name, major, "found catalyst chardev major");

    // Use the rm_trigger Rust binary for RM ioctl allocation.
    // It creates chardev nodes, opens them (triggering rm_init_adapter),
    // then issues NV_ESC_RM_ALLOC ioctls for root → device → subdevice →
    // GR control, which triggers full GR initialization (GPCCS + TPC).
    // Built from src/bin/rm_trigger.rs in the cylinder crate.
    let rm_trigger_bin = "/usr/local/bin/rm_trigger";
    if std::path::Path::new(rm_trigger_bin).exists() {
        tracing::info!(major, "spawning rm_trigger helper for RM ioctl sequence");
        match std::process::Command::new(rm_trigger_bin)
            .arg(major.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::info!(
                    exit_code = output.status.code(),
                    stdout = %stdout,
                    stderr = %stderr,
                    "rm_trigger helper completed"
                );
                // Give RM extra time for async GR init after helper exits
                std::thread::sleep(Duration::from_millis(3000));
                return Ok(format!(
                    "RM triggered via rm_trigger helper (major={major}), exit={}",
                    output.status.code().unwrap_or(-1)
                ));
            }
            Err(e) => {
                tracing::warn!(error = %e, "rm_trigger helper spawn failed — falling back to open-only");
            }
        }
    } else {
        tracing::warn!("rm_trigger binary not found at {rm_trigger_bin} — using open-only fallback");
    }

    // Fallback: just open the GPU device (minor 0) to trigger rm_init_adapter.
    let dev_path = format!("/dev/toadstool-{module_name}-ctl");
    let _ = std::fs::remove_file(&dev_path);

    let dev = rustix::fs::makedev(major, 0);
    match rustix::fs::mknodat(
        rustix::fs::CWD,
        &*dev_path,
        rustix::fs::FileType::CharacterDevice,
        rustix::fs::Mode::from_raw_mode(0o666),
        dev,
    ) {
        Ok(()) => {}
        Err(e) => return Err(format!("mknodat({dev_path}): {e}")),
    }

    tracing::info!(dev_path, major, "opening catalyst chardev to trigger RM init (fallback)");
    let fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&dev_path);
    match fd {
        Ok(f) => {
            std::thread::sleep(Duration::from_millis(5000));
            drop(f);
            let _ = std::fs::remove_file(&dev_path);
            Ok(format!("RM triggered via {dev_path} (major={major})"))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&dev_path);
            Err(format!("failed to open {dev_path}: {e}"))
        }
    }
}
