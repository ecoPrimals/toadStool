// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    unsafe_code,
    reason = "DRM kernel ioctls require unsafe FFI via rustix — containment zone"
)]

//! Safe DRM ioctl wrappers.
//!
//! Provides a safe API for executing DRM ioctls with runtime-determined
//! opcodes. Used by `hw-learn` for nouveau init recipe steps.
//!
//! [`DrmIoctlResult`] is available on all platforms. Ioctl functions are
//! Linux-only (gated internally).

/// Result of a single DRM ioctl execution.
#[derive(Debug)]
pub struct DrmIoctlResult {
    /// Whether the ioctl succeeded.
    pub success: bool,
    /// Diagnostic detail.
    pub detail: String,
}

#[cfg(target_os = "linux")]
use rustix::fd::{AsFd, OwnedFd};
#[cfg(target_os = "linux")]
use rustix::ioctl::{Ioctl, IoctlOutput, Opcode};
#[cfg(target_os = "linux")]
use std::ffi::CString;

/// Open a DRM device by path (e.g. `/dev/dri/card0`).
#[cfg(target_os = "linux")]
pub fn open_drm_device(path: &str) -> Result<OwnedFd, String> {
    let cpath = CString::new(path).map_err(|e| e.to_string())?;
    rustix::fs::open(
        cpath.as_c_str(),
        rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|e| format!("open failed: {e}"))
}

/// Execute a DRM ioctl with the given opcode and argument buffer.
///
/// The `args` are copied into a mutable buffer (DRM ioctls may mutate the arg).
/// Returns `Ok(())` on success, `Err(errno)` on failure.
#[cfg(target_os = "linux")]
pub fn execute_drm_ioctl(fd: &OwnedFd, ioctl_nr: u64, args: &[u8]) -> Result<(), i32> {
    let mut buf = if args.is_empty() {
        vec![0u8; 256]
    } else {
        args.to_vec()
    };

    let ioctl = DrmIoctlCmd {
        opcode: ioctl_nr_to_opcode(ioctl_nr),
        arg: buf.as_mut_ptr(),
    };

    // SAFETY: fd is a valid DRM device; ioctl cmd is constructed from a DRM
    // ioctl number with correct buffer layout. The kernel validates buffer
    // layout against the encoded size. Worst case: EINVAL.
    unsafe { rustix::ioctl::ioctl(fd.as_fd(), ioctl) }.map_err(rustix::io::Errno::raw_os_error)
}

#[cfg(target_os = "linux")]
struct DrmIoctlCmd {
    opcode: Opcode,
    arg: *mut u8,
}

#[cfg(target_os = "linux")]
// SAFETY: DRM ioctl with runtime opcode; caller verifies fd and arg validity.
unsafe impl Ioctl for DrmIoctlCmd {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        self.opcode
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.arg.cast()
    }
    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "DRM ioctls encode into Opcode-width values"
)]
const fn ioctl_nr_to_opcode(nr: u64) -> Opcode {
    nr as Opcode
}
