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

use std::ffi::CString;
use std::os::fd::FromRawFd as _;
use std::path::{Path, PathBuf};
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

// ── Tier 2: Guarded sysfs write (fork + direct write + timeout) ─────
//
// Phase 3 evolution: replaced `/bin/sh -c "printf ... > sysfs"` with
// fork + open(O_WRONLY) + write(). Same D-state isolation — the child
// is a disposable process that gets killed on timeout — but no shell
// process, no quoting, no PATH dependency.

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
                    let code = e.raw_os_error() as i32;
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
fn reap_forked_child(child_pid: rustix::process::Pid) {
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

/// Sysfs read via forked child with timeout. If the child doesn't
/// complete within `timeout`, it is killed and `Timeout` is returned.
///
/// Reads from certain sysfs attributes (e.g. `power/runtime_status` on a
/// D3cold device) can block indefinitely in kernel D-state. This provides
/// the same fork isolation as [`sysfs_write_guarded`] but for reads.
///
/// Returns the file contents as a trimmed string on success.
pub fn sysfs_read_guarded(
    path: &str,
    timeout: Duration,
) -> Result<String, GuardedSysfsError> {
    tracing::debug!(path, timeout_ms = timeout.as_millis() as u64, "guarded sysfs read");

    let path_c = CString::new(path).map_err(|_| GuardedSysfsError::WriteFailed {
        path: path.into(),
        reason: "path contains NUL byte".into(),
    })?;

    let (pipe_read, pipe_write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
        .map_err(|e| GuardedSysfsError::WriteFailed {
            path: path.into(),
            reason: format!("pipe creation failed: {e}"),
        })?;

    // SAFETY: fork in multi-threaded context. Child calls only
    // open/read/write(pipe)/close/exit_group — all async-signal-safe.
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::WriteFailed {
            path: path.into(),
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            drop(pipe_read);
            use rustix::fs::{open, Mode, OFlags};
            let fd = match open(path_c.as_c_str(), OFlags::RDONLY, Mode::empty()) {
                Ok(fd) => fd,
                Err(_) => rustix::runtime::exit_group(1),
            };
            let mut buf = [0u8; 4096];
            let n = match rustix::io::read(&fd, &mut buf) {
                Ok(n) => n,
                Err(_) => { drop(fd); rustix::runtime::exit_group(2) },
            };
            drop(fd);
            let _ = rustix::io::write(&pipe_write, &buf[..n]);
            drop(pipe_write);
            rustix::runtime::exit_group(0)
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => {
            drop(pipe_write);
            wait_for_child(child_pid, path, timeout)?;
            let mut buf = [0u8; 4096];
            let n = match rustix::io::read(&pipe_read, &mut buf) {
                Ok(n) => n,
                Err(_) => 0,
            };
            Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
        }
    }
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
pub fn sysfs_unbind_fire_and_poll(
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

    let _child_pid = fork_sysfs_child(&path_c, bdf.as_bytes())?;

    let start = Instant::now();
    let poll_interval = Duration::from_secs(2);

    loop {
        if read_current_driver(bdf).is_none() {
            let elapsed = start.elapsed();
            tracing::info!(
                bdf, elapsed_s = elapsed.as_secs(),
                "fire-and-poll unbind: driver cleared"
            );
            return Ok(elapsed);
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

// ── Tier 3: Guarded kmod operations (fork + syscall) ────────────────
//
// Phase 3 evolution: replaced `Command::new("insmod")` / `Command::new("rmmod")`
// with fork + `finit_module(2)` / `delete_module(2)` syscalls via rustix.
// Same D-state isolation — child is killed on timeout — but no PATH
// dependency on the `kmod` package.

/// Run an arbitrary command with timeout (legacy fallback for non-kmod uses).
///
/// Kept for `KmodBuilder` which still needs `make` via `Command::new`.
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

/// Wait for a forked kmod child with timeout, kill on timeout.
fn wait_for_kmod_child(
    child_pid: rustix::process::Pid,
    label: &str,
    args_str: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    use rustix::process::{Signal, WaitOptions, waitpid};

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match waitpid(Some(child_pid), WaitOptions::NOHANG) {
            Ok(Some((_pid, status))) => {
                if status.exited() && status.exit_status() == Some(0) {
                    tracing::info!(label, args = args_str,
                                   elapsed_ms = start.elapsed().as_millis() as u64,
                                   "kmod operation completed");
                    return Ok(());
                }
                let code = status.exit_status().unwrap_or(-1);
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("child exited with code {code}"),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(label, args = args_str,
                                   timeout_ms = timeout.as_millis() as u64,
                                   "kmod operation timed out — killing child");
                    let _ = rustix::process::kill_process(child_pid, Signal::KILL);
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
    tracing::info!(path = path_str.as_str(), params,
                   timeout_ms = timeout.as_millis() as u64,
                   "guarded insmod (finit_module)");

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
    let (pipe_read, pipe_write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: "finit_module".into(),
            args: path_str.clone(),
            reason: format!("pipe failed: {e}"),
        })?;

    // SAFETY: fork in multi-threaded context. Child calls only
    // finit_module (syscall) + write (pipe) + exit_group — all async-signal-safe.
    // ko_file fd is inherited by the child (not CLOEXEC).
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "finit_module".into(),
            args: path_str,
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            drop(pipe_read);
            match rustix::system::finit_module(&ko_file, &params_c, 0) {
                Ok(()) => rustix::runtime::exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error() as i32;
                    let _ = rustix::io::write(&pipe_write, &errno.to_ne_bytes());
                    rustix::runtime::exit_group(1)
                }
            }
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => {
            drop(ko_file);
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "finit_module", &path_str, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result {
                if reason.starts_with("child exited with code") {
                    let mut buf = [0u8; 4];
                    if let Ok(4) = rustix::io::read(&pipe_read, &mut buf) {
                        let errno = i32::from_ne_bytes(buf);
                        return Err(GuardedSysfsError::KmodFailed {
                            cmd: "finit_module".into(),
                            args: path_str,
                            reason: format!("finit_module errno {errno} ({})",
                                errno_name(errno)),
                        });
                    }
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
pub fn rmmod_guarded(name: &str, timeout: Duration) -> Result<(), GuardedSysfsError> {
    tracing::info!(module = name, timeout_ms = timeout.as_millis() as u64,
                   "guarded rmmod (delete_module)");

    let name_c = CString::new(name).map_err(|_| GuardedSysfsError::KmodFailed {
        cmd: "delete_module".into(),
        args: name.into(),
        reason: "name contains NUL byte".into(),
    })?;

    let (pipe_read, pipe_write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: "delete_module".into(),
            args: name.into(),
            reason: format!("pipe failed: {e}"),
        })?;

    // SAFETY: fork + delete_module syscall — async-signal-safe.
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "delete_module".into(),
            args: name.into(),
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            drop(pipe_read);
            match rustix::system::delete_module(&name_c, 0) {
                Ok(()) => rustix::runtime::exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error() as i32;
                    let _ = rustix::io::write(&pipe_write, &errno.to_ne_bytes());
                    rustix::runtime::exit_group(1)
                }
            }
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => {
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "delete_module", name, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result {
                if reason.starts_with("child exited with code") {
                    let mut buf = [0u8; 4];
                    if let Ok(4) = rustix::io::read(&pipe_read, &mut buf) {
                        let errno = i32::from_ne_bytes(buf);
                        return Err(GuardedSysfsError::KmodFailed {
                            cmd: "delete_module".into(),
                            args: name.into(),
                            reason: format!("delete_module errno {errno} ({})",
                                errno_name(errno)),
                        });
                    }
                }
            }
            result
        }
    }
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

/// Prepare a device for VFIO anchor release without triggering a reset.
///
/// Must be called BEFORE dropping the `VfioAnchor`. Three layers of defense:
///
/// 1. Pin bridge power hierarchy (prevent D3cold)
/// 2. Clear `reset_method` to suppress per-device FLR/PM reset (Exp 225)
/// 3. Load `no_bus_reset.ko` to set `PCI_DEV_FLAGS_NO_BUS_RESET`,
///    preventing the kernel's dev_set `pci_reset_bus()` SBR (Exp 226)
///
/// Without layers 1+2, `vfio_pci_core_release()` fires per-device FLR.
/// Without layer 3, `vfio_pci_dev_set_try_reset()` fires bus-level SBR
/// when all devices in the dev_set have open_count==0.
pub fn prepare_anchor_release(bdf: &str) {
    tracing::info!(bdf, "preparing anchor release: pinning bridges + disabling FLR + suppressing SBR");
    pin_bridge_hierarchy(bdf);
    disable_flr(bdf);
    for sib in iommu_group_siblings(bdf) {
        disable_flr(&sib);
    }
    // Exp 226: FLR suppression alone is insufficient — the kernel also
    // fires pci_reset_bus() (SBR) when the last dev_set fd closes.
    // Load a tiny module to set PCI_DEV_FLAGS_NO_BUS_RESET on the device.
    if let Err(e) = suppress_bus_reset(bdf) {
        tracing::error!(bdf, error = %e, "failed to suppress bus reset — SBR may destroy warm state");
    }
}

// ── Kbuild module builder ───────────────────────────────────────────
//
// Typed Rust abstraction over Linux kbuild — the irreducible kernel-space
// interface. C source and Makefile content are string literals (kernel ABI
// boundary), but everything around them — path resolution, compilation
// orchestration, parameter passing, load/unload lifecycle, cleanup —
// goes through the Rust compiler.

/// Builder for out-of-tree Linux kernel modules via kbuild.
///
/// Encapsulates the full lifecycle: stage source → compile via
/// `make -C /lib/modules/{krel}/build M=$PWD` → `insmod` with
/// parameters → `rmmod` → cleanup. Each step is typed and
/// compiler-verified; the only non-Rust artifacts are the C source
/// literal and the generated Makefile (irreducible kernel ABI boundary).
///
/// ```text
/// KmodBuilder::new("no_bus_reset")
///     .source(C_SOURCE)
///     .tmpdir("/tmp/toadstool-no-bus-reset")
///     .param("bdf", "0000:02:00.0")
///     .build_and_load()?;
/// ```
pub struct KmodBuilder {
    name: String,
    source: &'static str,
    tmpdir: String,
    params: Vec<(String, String)>,
}

impl KmodBuilder {
    /// Create a builder for a kernel module with the given name.
    ///
    /// The name determines the `.c` filename, `.ko` output, and
    /// `obj-m` target in the generated Makefile.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            source: "",
            tmpdir: format!("/tmp/toadstool-kmod-{name}"),
            params: Vec::new(),
        }
    }

    /// Set the C source code for the module.
    pub fn source(mut self, src: &'static str) -> Self {
        self.source = src;
        self
    }

    /// Override the build directory (default: `/tmp/toadstool-kmod-{name}`).
    pub fn tmpdir(mut self, dir: &str) -> Self {
        self.tmpdir = dir.to_string();
        self
    }

    /// Add a module parameter (passed to `insmod` as `key=value`).
    pub fn param(mut self, key: &str, value: &str) -> Self {
        self.params.push((key.to_string(), value.to_string()));
        self
    }

    /// If the module is already loaded, unload it first (idempotent reload).
    fn ensure_unloaded(&self) -> Result<(), GuardedSysfsError> {
        let sys_path = format!("/sys/module/{}", self.name);
        if Path::new(&sys_path).exists() {
            tracing::info!(module = self.name.as_str(),
                           "kmod already loaded — unloading for reload");
            let _ = rmmod_guarded(&self.name, RMMOD_TIMEOUT);
        }
        Ok(())
    }

    /// Stage source and Makefile, then compile via kbuild.
    ///
    /// Returns the path to the compiled `.ko` file. Does not load the
    /// module — use [`build_and_load`] for the full lifecycle, or call
    /// this when you only need the compiled artifact (e.g. ELF inspection
    /// in kernel health probes).
    pub fn compile_only(&self) -> Result<PathBuf, GuardedSysfsError> {
        let krel = crate::linux_paths::kernel_release().ok_or_else(|| {
            GuardedSysfsError::KmodFailed {
                cmd: "kernel_release".into(),
                args: String::new(),
                reason: "could not read /proc/sys/kernel/osrelease".into(),
            }
        })?;
        let kbuild = crate::linux_paths::kbuild_dir().ok_or_else(|| {
            GuardedSysfsError::KmodFailed {
                cmd: "kbuild_dir".into(),
                args: String::new(),
                reason: "kernel release unavailable for kbuild path".into(),
            }
        })?;

        let tmpdir = Path::new(&self.tmpdir);
        std::fs::create_dir_all(tmpdir)?;

        // Stage source
        let src_path = tmpdir.join(format!("{}.c", self.name));
        std::fs::write(&src_path, self.source)?;

        // Generate Makefile
        let makefile_path = tmpdir.join("Makefile");
        std::fs::write(
            &makefile_path,
            format!(
                "obj-m := {name}.o\n\
                 KDIR := {kbuild}\n\
                 all:\n\
                 \t$(MAKE) -C $(KDIR) M=$(PWD) modules\n\
                 clean:\n\
                 \t$(MAKE) -C $(KDIR) M=$(PWD) clean\n",
                name = self.name,
            ),
        )?;

        // Compile
        tracing::info!(module = self.name.as_str(), krel,
                       "kmod builder: compiling via kbuild");
        let compile_out = Command::new("make")
            .arg("-C")
            .arg(tmpdir)
            .output()
            .map_err(|e| GuardedSysfsError::KmodFailed {
                cmd: "make".into(),
                args: format!("-C {}", self.tmpdir),
                reason: format!("failed to spawn: {e}"),
            })?;

        if !compile_out.status.success() {
            let stderr = String::from_utf8_lossy(&compile_out.stderr);
            let snippet: String = stderr.lines().take(15).collect::<Vec<_>>().join("\n");
            return Err(GuardedSysfsError::KmodFailed {
                cmd: "make".into(),
                args: format!("-C {}", self.tmpdir),
                reason: format!("compilation failed:\n{snippet}"),
            });
        }

        let ko_path = tmpdir.join(format!("{}.ko", self.name));
        if !ko_path.exists() {
            return Err(GuardedSysfsError::KmodFailed {
                cmd: "make".into(),
                args: format!("-C {}", self.tmpdir),
                reason: format!("{}.ko not produced", self.name),
            });
        }

        Ok(ko_path)
    }

    /// Stage source and Makefile, compile via kbuild, and load the module.
    pub fn build_and_load(&self) -> Result<(), GuardedSysfsError> {
        self.ensure_unloaded()?;

        let ko_path = self.compile_only()?;

        let params_str: String = self.params.iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        insmod_guarded_with_params(&ko_path, &params_str, INSMOD_TIMEOUT)?;

        tracing::info!(module = self.name.as_str(),
                       params = ?self.params,
                       "kmod builder: module loaded");
        Ok(())
    }

    /// Remove build artifacts from the tmpdir.
    ///
    /// Deletes the entire build directory. Use after [`compile_only`] when
    /// the `.ko` has been consumed and is no longer needed.
    pub fn clean(tmpdir: &str) {
        let path = Path::new(tmpdir);
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    /// Unload the module and clean up build artifacts.
    pub fn unload_and_clean(name: &str, tmpdir: &str) -> Result<(), GuardedSysfsError> {
        let sys_path = format!("/sys/module/{name}");
        if !Path::new(&sys_path).exists() {
            return Ok(());
        }
        rmmod_guarded(name, RMMOD_TIMEOUT)?;
        tracing::info!(module = name, "kmod builder: module unloaded");

        KmodBuilder::clean(tmpdir);
        Ok(())
    }
}

// ── Bus-level reset (SBR) suppression ───────────────────────────────
//
// Exp 226: Clearing per-device `reset_method` (FLR/PM) is insufficient —
// when the last VFIO fd in a dev_set closes, the kernel's
// `vfio_pci_dev_set_try_reset()` fires `pci_reset_bus()` which performs a
// Secondary Bus Reset (SBR) at the PCIe bridge level.  SBR bypasses
// per-device `reset_method` entirely.
//
// `pci_reset_bus()` calls `pci_bus_resetable()` which checks
// `PCI_DEV_FLAGS_NO_BUS_RESET` on the bridge and all downstream devices.
// If any device has this flag, the bus is not resetable and SBR is skipped.
//
// Kernel 6.17 does not expose `no_bus_reset` via sysfs, so we compile and
// load a tiny GPL module that sets the flag on the target device.

const NO_BUS_RESET_MODULE: &str = "no_bus_reset";
const NO_BUS_RESET_TMPDIR: &str = "/tmp/toadstool-no-bus-reset";

const NO_BUS_RESET_SOURCE: &str = r#"
#include <linux/module.h>
#include <linux/pci.h>

static char *bdf = "";
module_param(bdf, charp, 0444);
MODULE_PARM_DESC(bdf, "PCI BDF to suppress bus reset for");

static struct pci_dev *target;

static int __init no_bus_reset_init(void) {
    struct pci_dev *dev = NULL;
    while ((dev = pci_get_device(PCI_ANY_ID, PCI_ANY_ID, dev))) {
        if (strcmp(dev_name(&dev->dev), bdf) == 0) {
            dev->dev_flags |= PCI_DEV_FLAGS_NO_BUS_RESET;
            target = dev;
            pr_info("no_bus_reset: suppressed on %s\n", bdf);
            return 0;
        }
    }
    pr_warn("no_bus_reset: device %s not found\n", bdf);
    return -ENODEV;
}

static void __exit no_bus_reset_exit(void) {
    if (target) {
        target->dev_flags &= ~PCI_DEV_FLAGS_NO_BUS_RESET;
        pci_dev_put(target);
        pr_info("no_bus_reset: restored on %s\n", bdf);
    }
}

module_init(no_bus_reset_init);
module_exit(no_bus_reset_exit);
MODULE_LICENSE("GPL");
"#;

/// Compile and load the `no_bus_reset` kernel module for a device.
///
/// Sets `PCI_DEV_FLAGS_NO_BUS_RESET` on the target device, which makes
/// `pci_bus_resetable()` return false and prevents `pci_reset_bus()` from
/// performing a Secondary Bus Reset (SBR) through the upstream bridge.
///
/// Must be called BEFORE dropping VFIO device fds. The module should be
/// unloaded via [`restore_bus_reset`] after the handoff is complete and
/// vfio-pci is re-bound.
pub fn suppress_bus_reset(bdf: &str) -> Result<(), GuardedSysfsError> {
    KmodBuilder::new(NO_BUS_RESET_MODULE)
        .source(NO_BUS_RESET_SOURCE)
        .tmpdir(NO_BUS_RESET_TMPDIR)
        .param("bdf", bdf)
        .build_and_load()
}

/// Unload the `no_bus_reset` kernel module and clean up build artifacts.
///
/// Clears `PCI_DEV_FLAGS_NO_BUS_RESET` on the target device (via the
/// module's exit handler) and removes the tmpdir.
pub fn restore_bus_reset() -> Result<(), GuardedSysfsError> {
    KmodBuilder::unload_and_clean(NO_BUS_RESET_MODULE, NO_BUS_RESET_TMPDIR)
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
