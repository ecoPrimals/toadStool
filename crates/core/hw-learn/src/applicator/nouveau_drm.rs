// SPDX-License-Identifier: AGPL-3.0-or-later
//! Apply recipe steps via nouveau DRM ioctls.
//!
//! Uses `hw-safe::drm_ioctl` for the unsafe DRM FFI.
//! Evolved from local unsafe to containment-zone delegation (S346).

use super::StepResult;

/// Execute an ioctl step against a DRM device.
#[must_use]
pub fn execute_ioctl(step_index: usize, card_path: &str, ioctl_nr: u64, args: &[u8]) -> StepResult {
    let fd = match toadstool_hw_safe::drm_ioctl::open_drm_device(card_path) {
        Ok(fd) => fd,
        Err(e) => {
            return StepResult {
                step_index,
                success: false,
                detail: format!("failed to open {card_path}: {e}"),
            };
        }
    };

    match toadstool_hw_safe::drm_ioctl::execute_drm_ioctl(&fd, ioctl_nr, args) {
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
