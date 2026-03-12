// SPDX-License-Identifier: AGPL-3.0-only
//! Apply recipe steps via nouveau DRM ioctls.
//!
//! This module uses the nouveau new UAPI (kernel 6.6+) to apply
//! init steps. It can be extended to support other DRM drivers
//! (amdgpu, i915) through the same pattern.
//!
//! Evolved from raw `extern "C"` FFI to `rustix` (S149, Mar 12, 2026).
//! File open/close uses safe `rustix::fs` with `OwnedFd` (auto-close).
//! The ioctl syscall remains a thin FFI wrapper — DRM ioctls with
//! runtime-determined opcodes have no safe abstraction.

use super::StepResult;
use rustix::fd::{AsFd, OwnedFd};
use std::ffi::CString;

/// Execute an ioctl step against a DRM device.
pub fn execute_ioctl(step_index: usize, card_path: &str, ioctl_nr: u64, args: &[u8]) -> StepResult {
    let fd = match open_drm_device(card_path) {
        Ok(fd) => fd,
        Err(e) => {
            return StepResult {
                step_index,
                success: false,
                detail: format!("failed to open {card_path}: {e}"),
            };
        }
    };

    let result = attempt_ioctl(&fd, ioctl_nr, args);

    match result {
        Ok(()) => StepResult {
            step_index,
            success: true,
            detail: format!("ioctl 0x{ioctl_nr:x} succeeded"),
        },
        Err(errno) => StepResult {
            step_index,
            success: false,
            detail: format!("ioctl 0x{ioctl_nr:x} failed: errno {errno}"),
        },
    }
}

fn open_drm_device(path: &str) -> Result<OwnedFd, String> {
    let cpath = CString::new(path).map_err(|e| e.to_string())?;
    rustix::fs::open(
        cpath.as_c_str(),
        rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|e| format!("open failed: {e}"))
}

/// Perform a DRM ioctl with a mutable buffer argument.
///
/// DRM ioctls use runtime-determined opcodes from init recipes, so they
/// cannot be expressed with rustix's compile-time `Ioctl` trait. This
/// thin wrapper is the only remaining FFI in hw-learn.
fn attempt_ioctl(fd: &OwnedFd, ioctl_nr: u64, args: &[u8]) -> Result<(), i32> {
    let mut buf = if args.is_empty() {
        vec![0u8; 256]
    } else {
        args.to_vec()
    };

    // SAFETY: `fd` is a valid, open DRM device file descriptor obtained
    // from `open_drm_device` above. `buf` is a mutable byte buffer that
    // outlives the syscall. `ioctl_nr` encodes the DRM command number
    // including direction and size per the DRM ioctl convention. The
    // kernel validates the buffer contents against the encoded size
    // before reading or writing. Worst case: the ioctl returns EINVAL
    // if the opcode or buffer layout is wrong.
    let ret = unsafe { raw_ioctl(fd.as_fd().as_raw_fd(), ioctl_nr, buf.as_mut_ptr()) };

    if ret < 0 {
        Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
    } else {
        Ok(())
    }
}

use std::os::fd::AsRawFd;

extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

/// Minimal FFI for the `ioctl` syscall.
///
/// # Safety
/// Caller must ensure `fd` is a valid file descriptor and `arg`
/// points to a buffer with the layout expected by `request`.
unsafe fn raw_ioctl(fd: i32, request: u64, arg: *mut u8) -> i32 {
    // SAFETY: forwarded from caller's contract; see `attempt_ioctl`.
    unsafe { ioctl(fd, request, arg) }
}
