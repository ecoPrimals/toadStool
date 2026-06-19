// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel module build, cache, and guarded load/unload operations.

use std::process::Command;
use std::time::{Duration, Instant};

use super::{GuardedSysfsError, reap_or_orphan};

mod build;
mod load;

pub use build::{
    KmodBuilder, disengage_irq_clutch, engage_irq_clutch, restore_bus_reset, suppress_all_resets,
    suppress_bus_reset, unsuppress_bus_reset_for,
};
pub use load::{insmod_guarded, insmod_guarded_with_params, rmmod_guarded};

/// Run an arbitrary command with timeout (legacy fallback for non-kmod uses).
///
/// Kept for `KmodBuilder` which still needs `make` via `Command::new`.
pub fn kmod_guarded(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, GuardedSysfsError> {
    let args_str = args.join(" ");
    tracing::info!(
        cmd,
        args = args_str.as_str(),
        timeout_ms = timeout.as_millis() as u64,
        "guarded kmod operation"
    );

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
                let output = child
                    .wait_with_output()
                    .unwrap_or_else(|_| std::process::Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                if status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    tracing::info!(
                        cmd,
                        args = args_str.as_str(),
                        elapsed_ms = start.elapsed().as_millis() as u64,
                        "kmod operation completed"
                    );
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
                    tracing::warn!(
                        cmd,
                        args = args_str.as_str(),
                        timeout_ms = timeout.as_millis() as u64,
                        "kmod operation timed out — killing child"
                    );
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
