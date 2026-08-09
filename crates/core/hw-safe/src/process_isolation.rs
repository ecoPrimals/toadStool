// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "fork requires unsafe — containment zone")]

//! Process isolation primitives — fork, exit, kill, wait, pipe, signal.
//!
//! These are the lowest-level process management syscalls wrapped in
//! safe(r) Rust via `rustix`. Used exclusively by cylinder's VFIO isolation.

/// Result of a `fork()` system call.
#[cfg(target_os = "linux")]
pub enum ForkResult {
    /// This is the child process.
    Child,
    /// This is the parent; holds the child's PID.
    Parent {
        /// PID of the child process.
        child_pid: rustix::process::Pid,
    },
}

/// Fork the current process.
///
/// # Safety
///
/// `fork()` in a multi-threaded program is inherently unsafe. The caller must
/// ensure the child only performs async-signal-safe operations.
///
/// On architectures where `rustix::runtime` is unavailable (e.g. powerpc64,
/// s390x, loongarch64), this returns `Unsupported`.
#[cfg(target_os = "linux")]
pub unsafe fn fork() -> std::io::Result<ForkResult> {
    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "mips64",
    ))]
    {
        // SAFETY: caller guarantees async-signal-safe usage in child.
        match unsafe { rustix::runtime::kernel_fork() } {
            Ok(rustix::runtime::Fork::Child(_)) => Ok(ForkResult::Child),
            Ok(rustix::runtime::Fork::ParentOf(pid)) => Ok(ForkResult::Parent { child_pid: pid }),
            Err(e) => Err(std::io::Error::from(e)),
        }
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "mips64",
    )))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "fork not available via rustix on this architecture",
        ))
    }
}

/// Exit all threads in the current process (child exit after fork).
///
/// On architectures where `rustix::runtime` is unavailable, falls back to
/// `std::process::exit`.
#[cfg(target_os = "linux")]
pub fn exit_group(code: i32) -> ! {
    #[cfg(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "mips64",
    ))]
    {
        rustix::runtime::exit_group(code)
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "mips64",
    )))]
    {
        std::process::exit(code)
    }
}

/// Create a pipe with `O_CLOEXEC` flag.
///
/// Returns `(read_end, write_end)` as owned file descriptors.
#[cfg(target_os = "linux")]
pub fn pipe_cloexec() -> std::io::Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC).map_err(std::io::Error::from)
}

/// Send SIGKILL to a process.
#[cfg(target_os = "linux")]
pub fn kill_process(pid: rustix::process::Pid) -> std::io::Result<()> {
    rustix::process::kill_process(pid, rustix::process::Signal::KILL).map_err(std::io::Error::from)
}

/// Non-blocking wait for a child process.
///
/// Returns `Some(WaitResult)` if the child exited/was signaled,
/// `None` if still running.
#[cfg(target_os = "linux")]
pub fn waitpid_nohang(pid: rustix::process::Pid) -> std::io::Result<Option<WaitResult>> {
    match rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG) {
        Ok(Some((_pid, status))) => {
            if status.exited() {
                Ok(Some(WaitResult::Exited(status.exit_status().unwrap_or(-1))))
            } else if status.signaled() {
                Ok(Some(WaitResult::Signaled(
                    status.terminating_signal().unwrap_or(-1),
                )))
            } else {
                Ok(Some(WaitResult::Exited(-1)))
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(std::io::Error::from(e)),
    }
}

/// Result of waiting on a child process.
#[derive(Debug, Clone, Copy)]
pub enum WaitResult {
    /// Child exited with the given status code.
    Exited(i32),
    /// Child was killed by a signal.
    Signaled(i32),
}

/// Re-export of `rustix::process::Pid` for process management.
#[cfg(target_os = "linux")]
pub use rustix::process::Pid;

/// Get the current process ID.
#[cfg(target_os = "linux")]
pub fn getpid() -> Pid {
    rustix::process::getpid()
}

/// Send a signal to a process (more general than kill_process which sends SIGKILL).
#[cfg(target_os = "linux")]
pub fn send_signal(pid: Pid, sig: i32) -> std::io::Result<()> {
    let signal = match sig {
        2 => rustix::process::Signal::INT,
        9 => rustix::process::Signal::KILL,
        15 => rustix::process::Signal::TERM,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported signal: {sig}"),
            ));
        }
    };
    rustix::process::kill_process(pid, signal).map_err(std::io::Error::from)
}
