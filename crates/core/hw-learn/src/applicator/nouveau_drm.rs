// SPDX-License-Identifier: AGPL-3.0-only
//! Apply recipe steps via nouveau DRM ioctls.
//!
//! This module uses the nouveau new UAPI (kernel 6.6+) to apply
//! init steps. It can be extended to support other DRM drivers
//! (amdgpu, i915) through the same pattern.

use super::StepResult;
use std::os::unix::io::RawFd;

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

    let result = unsafe { attempt_ioctl(fd, ioctl_nr, args) };

    unsafe { libc_close(fd) };

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

fn open_drm_device(path: &str) -> Result<RawFd, String> {
    use std::ffi::CString;
    let cpath = CString::new(path).map_err(|e| e.to_string())?;

    // O_RDWR | O_CLOEXEC
    let fd = unsafe { libc_open(cpath.as_ptr(), 0o2 | 0o2000000) };
    if fd < 0 {
        Err(format!(
            "open failed: errno {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(fd)
    }
}

/// Attempt a raw DRM ioctl. Unsafe because it performs a syscall.
unsafe fn attempt_ioctl(fd: RawFd, ioctl_nr: u64, args: &[u8]) -> Result<(), i32> {
    if args.is_empty() {
        // For ioctls without arguments, pass a zeroed buffer
        let mut buf = [0u8; 256];
        let ret = libc_ioctl(fd, ioctl_nr, buf.as_mut_ptr());
        if ret < 0 {
            Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
        } else {
            Ok(())
        }
    } else {
        let mut buf = args.to_vec();
        let ret = libc_ioctl(fd, ioctl_nr, buf.as_mut_ptr());
        if ret < 0 {
            Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
        } else {
            Ok(())
        }
    }
}

// Thin wrappers around libc functions to keep `unsafe` blocks minimal.
// Using raw syscall numbers instead of linking libc, consistent with
// toadStool's zero-C philosophy (though these are pragmatic FFI for DRM).

unsafe fn libc_open(path: *const i8, flags: i32) -> RawFd {
    extern "C" {
        fn open(path: *const i8, flags: i32) -> i32;
    }
    open(path, flags)
}

unsafe fn libc_ioctl(fd: RawFd, request: u64, arg: *mut u8) -> i32 {
    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    ioctl(fd, request, arg)
}

unsafe fn libc_close(fd: RawFd) {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    close(fd);
}
