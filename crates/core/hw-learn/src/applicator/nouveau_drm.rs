// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    unsafe_code,
    reason = "DRM kernel ioctls require unsafe FFI via rustix"
)]
//! Apply recipe steps via nouveau DRM ioctls.
//!
//! This module uses the nouveau new UAPI (kernel 6.6+) to apply
//! init steps. It can be extended to support other DRM drivers
//! (amdgpu, i915) through the same pattern.
//!
//! Evolved from raw `extern "C"` FFI to `rustix::ioctl` (S149, Mar 12, 2026).
//! File open/close uses safe `rustix::fs` with `OwnedFd` (auto-close).
//! DRM ioctls use runtime-determined opcodes from init recipes; we use
//! a `DrmIoctl` wrapper implementing rustix's `Ioctl` trait with a
//! raw opcode constructed from the recipe's ioctl number.

use super::StepResult;
use rustix::fd::{AsFd, OwnedFd};
use rustix::ioctl::{Ioctl, IoctlOutput, Opcode};
use std::ffi::CString;

/// Execute an ioctl step against a DRM device.
#[must_use]
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

/// DRM ioctl wrapper for runtime opcodes.
///
/// DRM ioctls encode direction+size+type+nr in a single u32. We construct
/// the rustix `Opcode` from the recipe's ioctl number.
struct DrmIoctl {
    opcode: Opcode,
    arg: *mut u8,
}

// SAFETY: DRM ioctl with runtime opcode; caller verifies fd and arg validity.
// The kernel validates buffer layout against the encoded size. Worst case:
// EINVAL if opcode or buffer is wrong.
unsafe impl Ioctl for DrmIoctl {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        self.opcode
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.arg.cast()
    }
    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        // SAFETY: No-op — body is intentionally empty (trait method stub for ioctl infrastructure).
        Ok(())
    }
}

/// Convert a DRM ioctl number (u64) to a rustix Opcode.
///
/// DRM ioctls encode direction+size+type+nr via _IOWR/_IOW/_IOR macros,
/// producing 32-bit values. rustix's `Opcode` is `c_uint` (u32).
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
const fn ioctl_nr_to_opcode(nr: u64) -> Opcode {
    nr as u32
}

/// Perform a DRM ioctl with a mutable buffer argument.
///
/// Uses `rustix::ioctl` with a `DrmIoctl` wrapper for runtime opcodes from
/// init recipes. Eliminates the last `extern "C"` FFI in hw-learn.
/// Single ioctl dispatch for DRM operations.
fn do_ioctl<I: rustix::ioctl::Ioctl>(fd: &OwnedFd, cmd: I) -> Result<I::Output, i32> {
    // SAFETY: fd is a valid DRM device from open_drm_device; cmd is
    // constructed from a DRM ioctl number with correct buffer layout.
    unsafe { rustix::ioctl::ioctl(fd.as_fd(), cmd) }.map_err(rustix::io::Errno::raw_os_error)
}

fn attempt_ioctl(fd: &OwnedFd, ioctl_nr: u64, args: &[u8]) -> Result<(), i32> {
    let mut buf = if args.is_empty() {
        vec![0u8; 256]
    } else {
        // Copy required: DRM ioctl mutates the argument buffer; caller only provides `&[u8]`.
        args.to_vec()
    };

    let ioctl = DrmIoctl {
        opcode: ioctl_nr_to_opcode(ioctl_nr),
        arg: buf.as_mut_ptr(),
    };

    do_ioctl(fd, ioctl)
}
