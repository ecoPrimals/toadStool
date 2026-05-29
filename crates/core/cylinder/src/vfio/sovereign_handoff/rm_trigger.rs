// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::nv::registers::pmc::InterruptProfile;

use super::errors::HandoffError;
use super::types::RmChannelEvidence;

/// Result of triggering RM initialization (and optionally creating a channel).
pub(crate) struct RmTriggerResult {
    pub summary: String,
    /// Present when rm_trigger ran with --channel and produced channel evidence.
    pub channel_evidence: Option<RmChannelEvidence>,
}

/// Trigger nvidia RM's GPU initialization by opening its dynamically-assigned chardev.
///
/// After the catalyst module loads with `__register_chrdev(0, ...)` (dynamic major),
/// we read `/proc/devices` to find the assigned major, create a temporary device node,
/// and open+close it. This triggers `nv_open()` → `nv_start_device()` → full RM init
/// (SEC2 → ACR → FECS → GPCCS → TPC PRI station creation).
///
/// When `create_channel` is true, passes `--channel` to rm_trigger to create a
/// full RM compute channel (Exp 229: Catalyst Channel).
pub(crate) fn trigger_rm_init(
    module_name: &str,
    create_channel: bool,
    bdf: &str,
    interrupt_profile: &InterruptProfile,
) -> Result<RmTriggerResult, HandoffError> {
    let devices = std::fs::read_to_string("/proc/devices")?;
    let mut majors: Vec<u32> = Vec::new();
    for line in devices.lines() {
        let line = line.trim();
        if (line.ends_with("nvidia-frontend") || line.ends_with(module_name))
            && let Some(num_str) = line.split_whitespace().next()
            && let Ok(n) = num_str.parse::<u32>()
        {
            majors.push(n);
        }
    }
    let major = majors
        .iter()
        .copied()
        .max()
        .ok_or_else(|| HandoffError::ChardevNotFound {
            module_name: module_name.to_string(),
        })?;

    tracing::info!(module_name, major, "found catalyst chardev major");

    let rm_trigger_bin = "/usr/local/bin/rm_trigger";
    if std::path::Path::new(rm_trigger_bin).exists() {
        tracing::info!(major, create_channel, bdf, "spawning rm_trigger helper");
        let mut cmd = std::process::Command::new(rm_trigger_bin);
        cmd.arg(major.to_string());
        if create_channel {
            cmd.arg("--channel");
        }
        cmd.args(["--bdf", bdf]);
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::info!(
                    exit_code = output.status.code(),
                    stdout = %stdout,
                    stderr = %stderr,
                    "rm_trigger helper completed"
                );

                let channel_evidence = if create_channel {
                    match serde_json::from_str::<serde_json::Value>(&stdout) {
                        Ok(json) => {
                            let ev = RmChannelEvidence::from_json(&json);
                            if let Some(ref e) = ev {
                                tracing::info!(
                                    channel_id = ?e.channel_id,
                                    work_submit_token = ?e.work_submit_token,
                                    steps_completed = e.steps_completed,
                                    all_ok = e.all_ok,
                                    "RM channel evidence captured"
                                );
                            }
                            ev
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to parse rm_trigger JSON output");
                            None
                        }
                    }
                } else {
                    None
                };

                // Exp 229 lockup #6: nvidia_close RE-ENABLES INTR_EN after
                // rm_trigger's pre-exit quench. Quench again from the pipeline
                // now that nvidia_close has fully completed.
                crate::nv::registers::pmc::quench_interrupts(
                    bdf, interrupt_profile, "post-exit (after nvidia_close)",
                );
                crate::nv::registers::pmc::intx_disable(bdf, "post-exit");

                std::thread::sleep(Duration::from_millis(3000));
                return Ok(RmTriggerResult {
                    summary: format!(
                        "RM triggered via rm_trigger helper (major={major}, channel={}), exit={}",
                        create_channel,
                        output.status.code().unwrap_or(-1)
                    ),
                    channel_evidence,
                });
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
        Err(e) => {
            return Err(HandoffError::DeviceNodeCreateFailed {
                path: dev_path.clone(),
                detail: e.to_string(),
            });
        }
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
            Ok(RmTriggerResult {
                summary: format!("RM triggered via {dev_path} (major={major})"),
                channel_evidence: None,
            })
        }
        Err(e) => {
            let _ = std::fs::remove_file(&dev_path);
            Err(HandoffError::ChardevOpenFailed {
                path: dev_path,
                source: e,
            })
        }
    }
}
