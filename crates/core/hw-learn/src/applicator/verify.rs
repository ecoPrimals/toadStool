// SPDX-License-Identifier: AGPL-3.0-or-later
//! Post-apply verification — confirm init recipe succeeded.
//!
//! Verification uses typed [`VerificationResult`] values. When [`RegisterAccess`]
//! or [`GpuReadbackAccess`] is unavailable, checks return
//! [`VerificationResult::Unavailable`] with a specific reason instead of opaque
//! debt strings.

use super::StepResult;
use crate::distiller::VerifyCheck;

/// Typed outcome for hardware verification steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Masked value matches expected.
    Success {
        /// Value read from hardware (after optional masking for display).
        value: u64,
        /// Expected value (same basis as `value`).
        expected: u64,
    },
    /// Read succeeded but masked value differs.
    Mismatch {
        /// Observed value (typically masked with the verify mask).
        actual: u64,
        /// Expected masked value.
        expected: u64,
        /// BAR-relative register offset when applicable.
        register: Option<u64>,
    },
    /// No path to perform this check on the current host / build.
    Unavailable {
        /// Why verification could not run.
        reason: UnavailableReason,
    },
    /// Transport or I/O failed while attempting verification.
    Error {
        /// Error description (e.g. MMIO fault string).
        source: String,
    },
}

/// Why a verification step could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableReason {
    /// No BAR / MMIO [`RegisterAccess`] was provided.
    NoRegisterAccess,
    /// No GPU readback implementation was wired (VRAM / compute scratch path).
    NoGpuReadbackPath,
    /// Memory aperture needs VFIO / visualization service; not BAR-mapped MMIO.
    ApertureNotProbedViaRegisterAccess {
        /// Distiller aperture label (e.g. `VRAM`, `SysMem`).
        aperture: String,
    },
}

impl std::fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnavailableReason::NoRegisterAccess => {
                write!(f, "no register access (attach RegisterAccess / Bar0Access)")
            }
            UnavailableReason::NoGpuReadbackPath => {
                write!(
                    f,
                    "no GPU readback path (wire GpuReadbackAccess for compute scratch)"
                )
            }
            UnavailableReason::ApertureNotProbedViaRegisterAccess { aperture } => {
                write!(
                    f,
                    "aperture {aperture:?} requires VFIO / service probe; not BAR MMIO"
                )
            }
        }
    }
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationResult::Success { value, expected } => {
                write!(f, "ok: value=0x{value:x} expected=0x{expected:x}")
            }
            VerificationResult::Mismatch {
                actual,
                expected,
                register,
            } => {
                if let Some(reg) = register {
                    write!(
                        f,
                        "mismatch at register 0x{reg:x}: actual=0x{actual:x} expected=0x{expected:x}"
                    )
                } else {
                    write!(f, "mismatch: actual=0x{actual:x} expected=0x{expected:x}")
                }
            }
            VerificationResult::Unavailable { reason } => write!(f, "unavailable: {reason}"),
            VerificationResult::Error { source } => write!(f, "error: {source}"),
        }
    }
}

/// Optional hook for GPU memory / compute scratch readback (no external deps).
///
/// Real backends (future) may map VRAM or read a post-dispatch scratch buffer.
pub trait GpuReadbackAccess {
    /// Read `word_count` 32-bit words from a GPU-accessible region at `byte_offset`.
    fn read_gpu_words(&self, byte_offset: u64, word_count: usize) -> Result<Vec<u32>, String>;
}

/// Run a verification check after recipe application.
#[must_use]
pub fn run_verification(
    step_index: usize,
    card_path: &str,
    check: &VerifyCheck,
    register_access: Option<&mut dyn super::RegisterAccess>,
    gpu_readback: Option<&dyn GpuReadbackAccess>,
) -> StepResult {
    match check {
        VerifyCheck::RegisterMatch {
            offset,
            expected,
            mask,
        } => {
            let ro = register_access
                .as_ref()
                .map(|r| &**r as &dyn super::RegisterAccess);
            step_from_verification(step_index, &verify_register(*offset, *expected, *mask, ro))
        }
        VerifyCheck::IoctlSucceeds { ioctl_nr } => verify_ioctl(step_index, card_path, *ioctl_nr),
        VerifyCheck::ComputeReadback => {
            step_from_verification(step_index, &verify_compute_readback(gpu_readback))
        }
        VerifyCheck::MemoryAccessible {
            aperture,
            offset,
            sentinel,
        } => step_from_verification(
            step_index,
            &verify_memory_accessible(aperture, *offset, *sentinel, register_access),
        ),
    }
}

fn step_from_verification(step_index: usize, vr: &VerificationResult) -> StepResult {
    let success = matches!(vr, VerificationResult::Success { .. });
    StepResult {
        step_index,
        success,
        detail: vr.to_string(),
    }
}

fn verify_register(
    offset: u64,
    expected: u64,
    mask: u64,
    register_access: Option<&dyn super::RegisterAccess>,
) -> VerificationResult {
    let Some(access) = register_access else {
        return VerificationResult::Unavailable {
            reason: UnavailableReason::NoRegisterAccess,
        };
    };
    match access.read_u32(offset) {
        Ok(val) => {
            let masked = u64::from(val) & mask;
            let expected_masked = expected & mask;
            if masked == expected_masked {
                VerificationResult::Success {
                    value: masked,
                    expected: expected_masked,
                }
            } else {
                VerificationResult::Mismatch {
                    actual: masked,
                    expected: expected_masked,
                    register: Some(offset),
                }
            }
        }
        Err(e) => VerificationResult::Error { source: e },
    }
}

fn verify_compute_readback(gpu_readback: Option<&dyn GpuReadbackAccess>) -> VerificationResult {
    let Some(rb) = gpu_readback else {
        return VerificationResult::Unavailable {
            reason: UnavailableReason::NoGpuReadbackPath,
        };
    };
    match rb.read_gpu_words(0, 4) {
        Ok(words) if words.is_empty() => VerificationResult::Error {
            source: "GPU readback returned zero words".into(),
        },
        Ok(words) => {
            let first = u64::from(words[0]);
            VerificationResult::Success {
                value: first,
                expected: first,
            }
        }
        Err(e) => VerificationResult::Error { source: e },
    }
}

fn verify_memory_accessible(
    aperture: &str,
    offset: u64,
    sentinel: u64,
    register_access: Option<&mut dyn super::RegisterAccess>,
) -> VerificationResult {
    let Some(access) = register_access else {
        return VerificationResult::Unavailable {
            reason: UnavailableReason::NoRegisterAccess,
        };
    };

    if !aperture_is_bar_mappable(aperture) {
        return VerificationResult::Unavailable {
            reason: UnavailableReason::ApertureNotProbedViaRegisterAccess {
                aperture: aperture.to_string(),
            },
        };
    }

    let word = sentinel as u32;
    if let Err(e) = access.write_u32(offset, word) {
        return VerificationResult::Error { source: e };
    }
    match access.read_u32(offset) {
        Ok(r) => {
            let actual = u64::from(r);
            let expected = u64::from(word);
            if actual == expected {
                VerificationResult::Success {
                    value: actual,
                    expected,
                }
            } else {
                VerificationResult::Mismatch {
                    actual,
                    expected,
                    register: Some(offset),
                }
            }
        }
        Err(e) => VerificationResult::Error { source: e },
    }
}

fn aperture_is_bar_mappable(aperture: &str) -> bool {
    matches!(
        aperture.to_ascii_uppercase().as_str(),
        "BAR" | "BAR0" | "MMIO" | "PRAMIN" | "REGS"
    )
}

#[cfg(target_os = "linux")]
fn verify_ioctl(step_index: usize, card_path: &str, ioctl_nr: u64) -> StepResult {
    let result = super::nouveau_drm::execute_ioctl(step_index, card_path, ioctl_nr, &[]);
    StepResult {
        step_index,
        success: result.success,
        detail: format!("verify ioctl 0x{ioctl_nr:x}: {}", result.detail),
    }
}

#[cfg(not(target_os = "linux"))]
fn verify_ioctl(step_index: usize, _card_path: &str, ioctl_nr: u64) -> StepResult {
    StepResult {
        step_index,
        success: false,
        detail: format!("verify ioctl 0x{ioctl_nr:x}: DRM ioctl path unavailable on this platform"),
    }
}

/// Verify a register using a `RegisterAccess` implementation (e.g. `Bar0Access`).
///
/// Preferred when BAR0 MMIO is available — delegates to the same logic as
/// [`run_verification`] for [`VerifyCheck::RegisterMatch`].
pub fn verify_register_via_access<R: super::RegisterAccess>(
    step_index: usize,
    access: &R,
    offset: u64,
    expected: u64,
    mask: u64,
) -> StepResult {
    step_from_verification(
        step_index,
        &verify_register(offset, expected, mask, Some(access)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

        fn write_u32(&mut self, offset: u64, value: u32) -> Result<(), String> {
            self.values.insert(offset, value);
            Ok(())
        }
    }

    struct MockReadback {
        words: Vec<u32>,
    }

    impl GpuReadbackAccess for MockReadback {
        fn read_gpu_words(&self, _byte_offset: u64, word_count: usize) -> Result<Vec<u32>, String> {
            Ok(self.words.iter().take(word_count).copied().collect())
        }
    }

    #[test]
    fn compute_readback_unavailable_without_hook() {
        let vr = verify_compute_readback(None);
        assert!(matches!(
            vr,
            VerificationResult::Unavailable {
                reason: UnavailableReason::NoGpuReadbackPath
            }
        ));
        let step = step_from_verification(0, &vr);
        assert!(!step.success);
        assert!(step.detail.contains("unavailable"));
    }

    #[test]
    fn compute_readback_success_with_hook() {
        let rb = MockReadback {
            words: vec![0xAABB_CCDD],
        };
        let vr = verify_compute_readback(Some(&rb));
        assert!(matches!(vr, VerificationResult::Success { .. }));
    }

    #[test]
    fn memory_accessible_bar_roundtrip() {
        let mut m = MockRegAccess {
            values: std::collections::HashMap::new(),
        };
        let vr = verify_memory_accessible("BAR", 0x1000, 0x42, Some(&mut m));
        assert!(matches!(vr, VerificationResult::Success { .. }));
    }

    #[test]
    fn memory_accessible_vram_requires_service() {
        let mut m = MockRegAccess {
            values: std::collections::HashMap::new(),
        };
        let vr = verify_memory_accessible("VRAM", 0, 0, Some(&mut m));
        assert!(matches!(
            vr,
            VerificationResult::Unavailable {
                reason: UnavailableReason::ApertureNotProbedViaRegisterAccess { .. }
            }
        ));
    }

    #[test]
    fn verify_register_no_access() {
        let vr = verify_register(0, 0, 0xFFFF_FFFF, None);
        assert!(matches!(
            vr,
            VerificationResult::Unavailable {
                reason: UnavailableReason::NoRegisterAccess
            }
        ));
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
        assert!(result.detail.contains("error"));
    }
}
