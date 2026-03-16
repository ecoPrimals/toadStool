// SPDX-License-Identifier: AGPL-3.0-only
//! Post-apply verification — confirm init recipe succeeded.
//!
//! `verify_register` uses the `RegisterAccess` trait so callers can
//! provide BAR0 MMIO (via nvpmu `Bar0Access`) or a test mock.

use super::StepResult;
use crate::distiller::VerifyCheck;

/// Run a verification check after recipe application.
#[must_use]
pub fn run_verification(step_index: usize, card_path: &str, check: &VerifyCheck) -> StepResult {
    match check {
        VerifyCheck::RegisterMatch {
            offset,
            expected,
            mask,
        } => verify_register(step_index, card_path, *offset, *expected, *mask),
        VerifyCheck::IoctlSucceeds { ioctl_nr } => verify_ioctl(step_index, card_path, *ioctl_nr),
        VerifyCheck::ComputeReadback => verify_compute_readback(step_index),
        VerifyCheck::MemoryAccessible {
            aperture,
            offset,
            sentinel,
        } => verify_memory_accessible(step_index, aperture, *offset, *sentinel),
    }
}

/// Verify a register matches an expected value via ioctl-based readback.
///
/// Direct BAR0 MMIO verification requires a `RegisterAccess` impl
/// (e.g. `nvpmu::Bar0Access`). This path uses a DRM ioctl approach
/// that works without root mmap permissions, reading the register
/// value via the debugfs or nouveau UAPI query interface.
fn verify_register(
    step_index: usize,
    card_path: &str,
    offset: u64,
    expected: u64,
    mask: u64,
) -> StepResult {
    let query_ioctl = build_register_query_ioctl(offset);

    let result = super::nouveau_drm::execute_ioctl(step_index, card_path, query_ioctl, &[]);

    if !result.success {
        return StepResult {
            step_index,
            success: false,
            detail: format!(
                "register verify 0x{offset:08x}: query ioctl failed — {}",
                result.detail
            ),
        };
    }

    StepResult {
        step_index,
        success: false,
        detail: format!(
            "register verify 0x{offset:08x} == 0x{expected:08x} (mask 0x{mask:08x}): \
             ioctl succeeded but value extraction requires nouveau UAPI query struct parsing \
             (tracked as D-REG-VERIFY)"
        ),
    }
}

/// Build a nouveau UAPI register query ioctl number.
///
/// The nouveau new UAPI (kernel 6.6+) uses `DRM_IOCTL_NOUVEAU_GETPARAM`
/// with NV-specific param types for register queries.
fn build_register_query_ioctl(offset: u64) -> u64 {
    const DRM_IOCTL_BASE: u64 = 0xC010_6400;
    DRM_IOCTL_BASE | (offset & 0xFFFF)
}

fn verify_ioctl(step_index: usize, card_path: &str, ioctl_nr: u64) -> StepResult {
    let result = super::nouveau_drm::execute_ioctl(step_index, card_path, ioctl_nr, &[]);
    StepResult {
        step_index,
        success: result.success,
        detail: format!("verify ioctl 0x{ioctl_nr:x}: {}", result.detail),
    }
}

fn verify_compute_readback(step_index: usize) -> StepResult {
    StepResult {
        step_index,
        success: false,
        detail: "compute readback verify: requires dispatch pipeline integration \
                 via barraCuda/coralReef (tracked as Gap 3 — FECS GR context init)"
            .to_string(),
    }
}

fn verify_memory_accessible(
    step_index: usize,
    aperture: &str,
    offset: u64,
    sentinel: u64,
) -> StepResult {
    StepResult {
        step_index,
        success: false,
        detail: format!(
            "memory accessible verify: {aperture} @ 0x{offset:x} sentinel 0x{sentinel:x} — \
             requires VFIO MemoryRegion probe integration via coralReef \
             (see coralReef::vfio::memory::PraminRegion for VRAM, DmaRegion for sysmem)"
        ),
    }
}

/// Verify a register using a `RegisterAccess` implementation (e.g. `Bar0Access`).
///
/// This is the preferred path when BAR0 MMIO is available — directly reads
/// the register and checks against the expected value with mask.
pub fn verify_register_via_access(
    step_index: usize,
    access: &dyn super::RegisterAccess,
    offset: u64,
    expected: u64,
    mask: u64,
) -> StepResult {
    match access.read_u32(offset) {
        Ok(val) => {
            let masked = u64::from(val) & mask;
            let expected_masked = expected & mask;
            if masked == expected_masked {
                StepResult {
                    step_index,
                    success: true,
                    detail: format!(
                        "register 0x{offset:08x} = 0x{val:08x} (expected 0x{expected:08x}, mask 0x{mask:08x})"
                    ),
                }
            } else {
                StepResult {
                    step_index,
                    success: false,
                    detail: format!(
                        "register 0x{offset:08x} = 0x{val:08x} (expected 0x{expected:08x}, mask 0x{mask:08x}): mismatch"
                    ),
                }
            }
        }
        Err(e) => StepResult {
            step_index,
            success: false,
            detail: format!("register 0x{offset:08x} read failed: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_readback_not_yet_implemented() {
        let result = verify_compute_readback(0);
        assert!(!result.success);
        assert!(result.detail.contains("compute readback"));
    }

    struct MockRegAccess {
        values: std::collections::HashMap<u64, u32>,
    }

    impl super::super::RegisterAccess for MockRegAccess {
        fn read_u32(&self, offset: u64) -> Result<u32, String> {
            self.values
                .get(&offset)
                .copied()
                .ok_or_else(|| format!("no value at 0x{offset:x}"))
        }

        fn write_u32(&mut self, _offset: u64, _value: u32) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn verify_register_via_access_match() {
        let mut values = std::collections::HashMap::new();
        values.insert(0x2000, 0xDEAD_BEEF_u32);
        let access = MockRegAccess { values };

        let result = verify_register_via_access(0, &access, 0x2000, 0xDEAD_BEEF, 0xFFFF_FFFF);
        assert!(result.success);
    }

    #[test]
    fn verify_register_via_access_mask_match() {
        let mut values = std::collections::HashMap::new();
        values.insert(0x2000, 0xDEAD_BEEF_u32);
        let access = MockRegAccess { values };

        let result = verify_register_via_access(0, &access, 0x2000, 0x0000_BEEF, 0x0000_FFFF);
        assert!(result.success);
    }

    #[test]
    fn verify_register_via_access_mismatch() {
        let mut values = std::collections::HashMap::new();
        values.insert(0x2000, 0xDEAD_BEEF_u32);
        let access = MockRegAccess { values };

        let result = verify_register_via_access(0, &access, 0x2000, 0xCAFE_BABE, 0xFFFF_FFFF);
        assert!(!result.success);
        assert!(result.detail.contains("mismatch"));
    }

    #[test]
    fn verify_register_via_access_read_error() {
        let access = MockRegAccess {
            values: std::collections::HashMap::new(),
        };

        let result = verify_register_via_access(0, &access, 0x9999, 0, 0xFFFF_FFFF);
        assert!(!result.success);
        assert!(result.detail.contains("read failed"));
    }
}
