// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::CString;
use std::os::fd::AsFd;
use std::path::Path;
use std::time::{Duration, Instant};

use toadstool_hw_safe::{
    ForkResult, LinuxDeviceIo, Pid, WaitResult, delete_module, exit_group, finit_module, fork,
    kill_process, pipe_cloexec, waitpid_nohang,
};

use super::super::GuardedSysfsError;
use super::super::driver_ops::reap_forked_child;

/// Wait for a forked kmod child with timeout, kill on timeout.
fn wait_for_kmod_child(
    child_pid: Pid,
    label: &str,
    args_str: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match waitpid_nohang(child_pid) {
            Ok(Some(WaitResult::Exited(0))) => {
                tracing::info!(
                    label,
                    args = args_str,
                    elapsed_ms = start.elapsed().as_millis() as u64,
                    "kmod operation completed"
                );
                return Ok(());
            }
            Ok(Some(WaitResult::Exited(code))) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("child exited with code {code}"),
                });
            }
            Ok(Some(WaitResult::Signaled(sig))) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("child killed by signal {sig}"),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(
                        label,
                        args = args_str,
                        timeout_ms = timeout.as_millis() as u64,
                        "kmod operation timed out — killing child"
                    );
                    let _ = kill_process(child_pid);
                    reap_forked_child(child_pid);
                    return Err(GuardedSysfsError::KmodTimeout {
                        cmd: label.into(),
                        args: args_str.into(),
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("waitpid failed: {e}"),
                });
            }
        }
    }
}

/// Guarded `insmod` — load a kernel module via `finit_module(2)` in a
/// forked child. Pure Rust, no `insmod` binary dependency.
pub fn insmod_guarded(ko_path: &Path, timeout: Duration) -> Result<(), GuardedSysfsError> {
    insmod_guarded_with_params(ko_path, "", timeout)
}

/// Guarded `insmod` with module parameters.
pub fn insmod_guarded_with_params(
    ko_path: &Path,
    params: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    let path_str = ko_path.display().to_string();
    tracing::info!(
        path = path_str.as_str(),
        params,
        timeout_ms = timeout.as_millis() as u64,
        "guarded insmod (finit_module)"
    );

    let ko_file = std::fs::File::open(ko_path).map_err(|e| GuardedSysfsError::KmodFailed {
        cmd: "finit_module".into(),
        args: path_str.clone(),
        reason: format!("failed to open .ko: {e}"),
    })?;

    let params_c = CString::new(params).map_err(|_| GuardedSysfsError::KmodFailed {
        cmd: "finit_module".into(),
        args: path_str.clone(),
        reason: "params contain NUL byte".into(),
    })?;

    // Pipe for errno propagation: child writes raw errno (4 bytes) on
    // failure, nothing on success. Parent reads after waitpid.
    let (pipe_read, pipe_write) = pipe_cloexec().map_err(|e| GuardedSysfsError::KmodFailed {
        cmd: "finit_module".into(),
        args: path_str.clone(),
        reason: format!("pipe failed: {e}"),
    })?;

    // SAFETY: fork in multi-threaded context. Child calls only
    // finit_module (syscall) + write (pipe) + exit_group — all async-signal-safe.
    // ko_file fd is inherited by the child (not CLOEXEC).
    let fork_result = unsafe { fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "finit_module".into(),
            args: path_str,
            reason: format!("fork failed: {e}"),
        }),
        Ok(ForkResult::Child) => {
            drop(pipe_read);
            match finit_module(&ko_file, &params_c, 0) {
                Ok(()) => exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error().unwrap_or(1);
                    let _ = LinuxDeviceIo::write(pipe_write.as_fd(), &errno.to_ne_bytes());
                    exit_group(1)
                }
            }
        }
        Ok(ForkResult::Parent { child_pid }) => {
            drop(ko_file);
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "finit_module", &path_str, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result
                && reason.starts_with("child exited with code")
            {
                let mut buf = [0u8; 4];
                if matches!(LinuxDeviceIo::read(pipe_read.as_fd(), &mut buf), Ok(4)) {
                    let errno = i32::from_ne_bytes(buf);
                    return Err(GuardedSysfsError::KmodFailed {
                        cmd: "finit_module".into(),
                        args: path_str,
                        reason: format!("finit_module errno {errno} ({})", errno_name(errno)),
                    });
                }
            }
            result
        }
    }
}

/// Map common finit_module/delete_module errnos to human-readable names.
fn errno_name(errno: i32) -> &'static str {
    match errno {
        1 => "EPERM",
        2 => "ENOENT",
        12 => "ENOMEM",
        16 => "EBUSY",
        17 => "EEXIST",
        22 => "EINVAL",
        _ => "unknown",
    }
}

/// Guarded `rmmod` — unload a kernel module via `delete_module(2)` in a
/// forked child. Pure Rust, no `rmmod` binary dependency.
///
/// On failure, automatically retries with `O_NONBLOCK | O_TRUNC` (force
/// removal) as a zombie-killer fallback. This handles modules stuck in
/// cleanup due to NOP'd teardown paths.
pub fn rmmod_guarded(name: &str, timeout: Duration) -> Result<(), GuardedSysfsError> {
    match rmmod_with_flags(name, 0, timeout) {
        Ok(()) => Ok(()),
        Err(first_err) => {
            tracing::warn!(module = name, error = %first_err,
                           "normal rmmod failed — trying force rmmod (O_NONBLOCK|O_TRUNC)");
            match rmmod_with_flags(name, O_NONBLOCK | O_TRUNC, timeout) {
                Ok(()) => {
                    tracing::info!(module = name, "force rmmod succeeded (zombie buried)");
                    Ok(())
                }
                Err(force_err) => {
                    tracing::warn!(module = name, error = %force_err,
                                   "force rmmod also failed — module is a permanent zombie");
                    Err(first_err)
                }
            }
        }
    }
}

const O_NONBLOCK: i32 = 0x800;
const O_TRUNC: i32 = 0x200;

/// Inner `delete_module` with configurable flags.
fn rmmod_with_flags(name: &str, flags: i32, timeout: Duration) -> Result<(), GuardedSysfsError> {
    let flag_desc = if flags == 0 {
        "normal".to_string()
    } else {
        format!("flags=0x{flags:x}")
    };
    tracing::info!(
        module = name,
        timeout_ms = timeout.as_millis() as u64,
        mode = flag_desc.as_str(),
        "guarded rmmod (delete_module)"
    );

    let name_c = CString::new(name).map_err(|_| GuardedSysfsError::KmodFailed {
        cmd: "delete_module".into(),
        args: name.into(),
        reason: "name contains NUL byte".into(),
    })?;

    let (pipe_read, pipe_write) = pipe_cloexec().map_err(|e| GuardedSysfsError::KmodFailed {
        cmd: "delete_module".into(),
        args: name.into(),
        reason: format!("pipe failed: {e}"),
    })?;

    // SAFETY: fork + delete_module syscall — async-signal-safe.
    let fork_result = unsafe { fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "delete_module".into(),
            args: name.into(),
            reason: format!("fork failed: {e}"),
        }),
        Ok(ForkResult::Child) => {
            drop(pipe_read);
            match delete_module(&name_c, flags) {
                Ok(()) => exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error().unwrap_or(1);
                    let _ = LinuxDeviceIo::write(pipe_write.as_fd(), &errno.to_ne_bytes());
                    exit_group(1)
                }
            }
        }
        Ok(ForkResult::Parent { child_pid }) => {
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "delete_module", name, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result
                && reason.starts_with("child exited with code")
            {
                let mut buf = [0u8; 4];
                if matches!(LinuxDeviceIo::read(pipe_read.as_fd(), &mut buf), Ok(4)) {
                    let errno = i32::from_ne_bytes(buf);
                    return Err(GuardedSysfsError::KmodFailed {
                        cmd: "delete_module".into(),
                        args: name.into(),
                        reason: format!(
                            "delete_module errno {errno} ({}) [flags=0x{flags:x}]",
                            errno_name(errno)
                        ),
                    });
                }
            }
            result
        }
    }
}
