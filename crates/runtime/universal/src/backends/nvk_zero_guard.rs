// SPDX-License-Identifier: AGPL-3.0-only
//! NVK zero-output guard and device health checks.
//!
//! NVK on certain architectures (Volta SM70) produces all-zeros for f64
//! compute shaders that appear to compile and dispatch successfully.
//! This module validates shader output buffers and signals fallback.

/// NVK zero-output guard verdict (absorbed from airSpring v0.7.5).
///
/// NVK on certain architectures (Volta SM70) produces all-zeros for f64
/// compute shaders that appear to compile and dispatch successfully.
/// This guard validates shader output buffers and signals fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroGuardVerdict {
    /// Output contains non-zero values — computation is valid.
    Valid,
    /// Output is all zeros — likely NVK zero-output bug.
    AllZeros,
    /// Output is NaN-contaminated — precision failure.
    NanContaminated,
}

/// Check a buffer of f64 results for NVK zero-output patterns.
///
/// Returns `ZeroGuardVerdict::AllZeros` when every element is exactly 0.0,
/// which indicates the NVK zero-output bug on affected architectures.
/// Returns `NanContaminated` if any element is NaN.
#[must_use]
pub fn nvk_zero_guard_check(output: &[f64]) -> ZeroGuardVerdict {
    if output.is_empty() {
        return ZeroGuardVerdict::Valid;
    }

    let mut all_zero = true;
    for &v in output {
        if v.is_nan() {
            return ZeroGuardVerdict::NanContaminated;
        }
        if v != 0.0 {
            all_zero = false;
        }
    }

    if all_zero {
        ZeroGuardVerdict::AllZeros
    } else {
        ZeroGuardVerdict::Valid
    }
}

/// f32 variant of the zero-guard check.
#[must_use]
pub fn nvk_zero_guard_check_f32(output: &[f32]) -> ZeroGuardVerdict {
    if output.is_empty() {
        return ZeroGuardVerdict::Valid;
    }

    let mut all_zero = true;
    for &v in output {
        if v.is_nan() {
            return ZeroGuardVerdict::NanContaminated;
        }
        if v != 0.0 {
            all_zero = false;
        }
    }

    if all_zero {
        ZeroGuardVerdict::AllZeros
    } else {
        ZeroGuardVerdict::Valid
    }
}

/// Device health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceHealthStatus {
    Healthy,
    /// Device may be poisoned — NVVM compilation failure detected.
    PoisonSuspected,
    /// Device confirmed poisoned — all operations will fail.
    Poisoned,
}

/// Check device health status based on observed NVVM compilation behavior.
///
/// Call when NVVM compilation fails — probing risks poisoning.
#[must_use]
pub const fn check_device_health(
    nvvm_compilation_failed: bool,
    all_operations_failing: bool,
) -> DeviceHealthStatus {
    if all_operations_failing {
        DeviceHealthStatus::Poisoned
    } else if nvvm_compilation_failed {
        DeviceHealthStatus::PoisonSuspected
    } else {
        DeviceHealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_device_health() {
        assert_eq!(
            check_device_health(false, false),
            DeviceHealthStatus::Healthy
        );
        assert_eq!(
            check_device_health(true, false),
            DeviceHealthStatus::PoisonSuspected
        );
        assert_eq!(
            check_device_health(true, true),
            DeviceHealthStatus::Poisoned
        );
        assert_eq!(
            check_device_health(false, true),
            DeviceHealthStatus::Poisoned
        );
    }

    #[test]
    fn zero_guard_valid_output() {
        let output = [1.0, 2.0, 3.0, 0.5];
        assert_eq!(nvk_zero_guard_check(&output), ZeroGuardVerdict::Valid);
    }

    #[test]
    fn zero_guard_all_zeros() {
        let output = [0.0, 0.0, 0.0, 0.0];
        assert_eq!(nvk_zero_guard_check(&output), ZeroGuardVerdict::AllZeros);
    }

    #[test]
    fn zero_guard_nan_contaminated() {
        let output = [1.0, f64::NAN, 3.0];
        assert_eq!(
            nvk_zero_guard_check(&output),
            ZeroGuardVerdict::NanContaminated
        );
    }

    #[test]
    fn zero_guard_empty_is_valid() {
        let output: [f64; 0] = [];
        assert_eq!(nvk_zero_guard_check(&output), ZeroGuardVerdict::Valid);
    }

    #[test]
    fn zero_guard_single_nonzero() {
        let output = [0.0, 0.0, 1e-300, 0.0];
        assert_eq!(nvk_zero_guard_check(&output), ZeroGuardVerdict::Valid);
    }

    #[test]
    fn zero_guard_f32_all_zeros() {
        let output: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
        assert_eq!(
            nvk_zero_guard_check_f32(&output),
            ZeroGuardVerdict::AllZeros
        );
    }

    #[test]
    fn zero_guard_f32_valid() {
        let output: [f32; 3] = [1.0, 0.0, 0.5];
        assert_eq!(nvk_zero_guard_check_f32(&output), ZeroGuardVerdict::Valid);
    }

    #[test]
    fn zero_guard_f32_nan() {
        let output: [f32; 2] = [f32::NAN, 1.0];
        assert_eq!(
            nvk_zero_guard_check_f32(&output),
            ZeroGuardVerdict::NanContaminated
        );
    }
}
