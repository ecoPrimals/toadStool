// SPDX-License-Identifier: AGPL-3.0-only
//! Post-apply verification — confirm init recipe succeeded.

use super::StepResult;
use crate::distiller::VerifyCheck;

/// Run a verification check after recipe application.
pub fn run_verification(
    step_index: usize,
    card_path: &str,
    check: &VerifyCheck,
) -> StepResult {
    match check {
        VerifyCheck::RegisterMatch { offset, expected, mask } => {
            verify_register(step_index, card_path, *offset, *expected, *mask)
        }
        VerifyCheck::IoctlSucceeds { ioctl_nr } => {
            verify_ioctl(step_index, card_path, *ioctl_nr)
        }
        VerifyCheck::ComputeReadback => {
            verify_compute_readback(step_index, card_path)
        }
    }
}

fn verify_register(
    step_index: usize,
    _card_path: &str,
    offset: u64,
    expected: u64,
    _mask: u64,
) -> StepResult {
    // Register readback requires debugfs or mapped BAR access.
    // Stub for now — will be implemented when we have mmio access.
    StepResult {
        step_index,
        success: false,
        detail: format!(
            "register verify 0x{offset:08x} == 0x{expected:08x}: \
             not yet implemented (needs debugfs/BAR access)"
        ),
    }
}

fn verify_ioctl(step_index: usize, card_path: &str, ioctl_nr: u64) -> StepResult {
    let result = super::nouveau_drm::execute_ioctl(step_index, card_path, ioctl_nr, &[]);
    StepResult {
        step_index,
        success: result.success,
        detail: format!("verify ioctl 0x{ioctl_nr:x}: {}", result.detail),
    }
}

fn verify_compute_readback(step_index: usize, _card_path: &str) -> StepResult {
    // Full compute readback requires buffer alloc + shader dispatch + readback.
    // This is the ultimate test — delegates to barraCuda/coralReef dispatch.
    StepResult {
        step_index,
        success: false,
        detail: "compute readback verify: not yet implemented \
                 (requires dispatch pipeline integration)"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_readback_not_yet_implemented() {
        let result = verify_compute_readback(0, "/dev/dri/card0");
        assert!(!result.success);
        assert!(result.detail.contains("not yet implemented"));
    }
}
