// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::backends::wgpu_backend::{
    GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, SubstrateCapabilityKind,
};

fn make_test_adapter(
    name: &str,
    driver: &str,
    supports_f64: bool,
    f64_unreliable: bool,
) -> GpuAdapterInfo {
    let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
    let is_ada = name.contains("RTX 40") || name.contains("RTX 4070") || name.contains("RTX 4090");
    let is_prop_nv = driver.contains("nvidia") && !driver.contains("nvk");
    let zeros_risk = (is_nvk && supports_f64) || (is_ada && is_prop_nv);

    let mut capabilities = vec![SubstrateCapabilityKind::NnInference];
    if supports_f64 && !f64_unreliable {
        capabilities.push(SubstrateCapabilityKind::F64Native);
    }
    capabilities.push(SubstrateCapabilityKind::Df64Emulation);

    let fingerprint = HardwareFingerprint {
        estimated_tflops_f32: 20.0,
        estimated_tflops_f64: if supports_f64 && !f64_unreliable {
            10.0
        } else {
            0.0
        },
        sovereign_capable: true,
        sovereign_binary_capable: false,
        capabilities,
    };

    GpuAdapterInfo {
        name: name.to_owned(),
        driver: driver.to_owned(),
        driver_info: String::new(),
        vendor_id: 0,
        device_id: 0,
        backend: "Vulkan".to_owned(),
        device_type: GpuDeviceType::Discrete,
        max_compute_workgroups_per_dimension: 65535,
        max_compute_workgroup_size_x: 256,
        max_compute_workgroup_size_y: 256,
        max_compute_workgroup_size_z: 64,
        max_buffer_size: 4_294_967_296,
        supports_shader_f64: supports_f64,
        f64_compute_unreliable: f64_unreliable,
        f64_shared_memory_reliable: false,
        f64_zeros_risk: zeros_risk,
        min_subgroup_size: 32,
        max_subgroup_size: 32,
        fingerprint,
        safe_allocation_limit: 4_294_967_296,
        silicon: None,
    }
}

#[test]
fn test_nvk_all_tiers_safe() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.poisoning_risk, NvvmPoisoningRisk::None);
    assert!(!cal.nvvm_transcendental_risk);
    assert!(cal.has_any_f64);
    assert!(cal.df64_arithmetic_safe);

    assert!(cal.is_tier_safe(PrecisionTier::F32, false));
    assert!(cal.is_tier_safe(PrecisionTier::F32, true));
    assert!(cal.is_tier_safe(PrecisionTier::F64, true));
    assert!(cal.is_tier_safe(PrecisionTier::F64Precise, true));
    assert!(cal.is_tier_safe(PrecisionTier::Df64, true));
}

#[test]
fn test_amd_radv_all_tiers_safe() {
    let adapter = make_test_adapter("AMD Radeon RX 6950 XT", "radv", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.poisoning_risk, NvvmPoisoningRisk::None);
    assert!(!cal.nvvm_transcendental_risk);
    assert!(cal.has_any_f64);
    assert!(cal.df64_arithmetic_safe);

    assert!(cal.is_tier_safe(PrecisionTier::Df64, true));
    assert!(cal.is_tier_safe(PrecisionTier::F64Precise, true));
}

#[test]
fn test_nvidia_proprietary_transcendental_risk() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3090", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.poisoning_risk, NvvmPoisoningRisk::TranscendentalOnly);
    assert!(cal.nvvm_transcendental_risk);
    assert!(cal.has_any_f64);
    assert!(cal.df64_arithmetic_safe);

    assert!(cal.is_tier_safe(PrecisionTier::F32, true));
    assert!(cal.is_tier_safe(PrecisionTier::F64, true));
    assert!(!cal.is_tier_safe(PrecisionTier::F64Precise, true));
    assert!(!cal.is_tier_safe(PrecisionTier::Df64, true));

    assert!(cal.is_tier_safe(PrecisionTier::F64Precise, false));
    assert!(cal.is_tier_safe(PrecisionTier::Df64, false));
}

#[test]
fn test_f32_only_adapter() {
    let adapter = make_test_adapter("Intel UHD Graphics 630", "anv", false, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert!(!cal.has_any_f64);
    assert!(!cal.df64_arithmetic_safe);
    assert_eq!(cal.tiers.len(), 1);
    assert_eq!(cal.tiers[0].tier, PrecisionTier::F32);

    assert!(cal.is_tier_safe(PrecisionTier::F32, false));
    assert!(cal.is_tier_safe(PrecisionTier::F32, true));
    assert!(!cal.is_tier_safe(PrecisionTier::F64, false));
    assert!(!cal.is_tier_safe(PrecisionTier::Df64, true));
}

#[test]
fn test_unknown_driver_risky() {
    let adapter = make_test_adapter("Mystery GPU", "unknown-driver", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.poisoning_risk, NvvmPoisoningRisk::Unknown);
    assert!(cal.nvvm_transcendental_risk);
}

#[test]
fn test_tier_safety_with_without_transcendentals() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3080", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert!(cal.is_tier_safe(PrecisionTier::Df64, false));
    assert!(!cal.is_tier_safe(PrecisionTier::Df64, true));
}

#[test]
fn test_best_tier_precision_critical_no_transcendentals() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3090", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    let tier = cal.best_tier(true, false);
    assert_eq!(tier, PrecisionTier::F64Precise);
}

#[test]
fn test_best_tier_precision_critical_with_transcendentals() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3090", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    let tier = cal.best_tier(true, true);
    assert_eq!(tier, PrecisionTier::F64);
}

#[test]
fn test_best_tier_throughput_bound() {
    let adapter = make_test_adapter("NVIDIA GeForce RTX 3090", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    let tier = cal.best_tier(false, false);
    assert_eq!(tier, PrecisionTier::F64);
}

#[test]
fn test_best_tier_f32_only_fallback() {
    let adapter = make_test_adapter("Intel UHD 630", "anv", false, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.best_tier(true, false), PrecisionTier::F32);
    assert_eq!(cal.best_tier(true, true), PrecisionTier::F32);
    assert_eq!(cal.best_tier(false, false), PrecisionTier::F32);
}

#[test]
fn test_best_tier_nvk_prefers_f64_precise() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);

    assert_eq!(cal.best_tier(true, true), PrecisionTier::F64Precise);
}

#[test]
fn test_precision_tier_equality() {
    assert_eq!(PrecisionTier::F32, PrecisionTier::F32);
    assert_ne!(PrecisionTier::F32, PrecisionTier::F64);
}

#[test]
fn test_nvvm_poisoning_risk_variants() {
    assert_eq!(NvvmPoisoningRisk::None, NvvmPoisoningRisk::None);
    assert_ne!(
        NvvmPoisoningRisk::None,
        NvvmPoisoningRisk::TranscendentalOnly
    );
}

#[test]
fn brain_nvk_critical_routes_f64_precise() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(
        brain.route(PrecisionHint::Critical),
        PrecisionTier::F64Precise
    );
}

#[test]
fn brain_nvk_moderate_routes_f64() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(brain.route(PrecisionHint::Moderate), PrecisionTier::F64);
}

#[test]
fn brain_nvk_throughput_routes_f64() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(
        brain.route(PrecisionHint::ThroughputBound),
        PrecisionTier::F64
    );
}

#[test]
fn brain_low_precision_always_f32() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(brain.route(PrecisionHint::LowPrecision), PrecisionTier::F32);
}

#[test]
fn brain_f32_only_gpu_all_routes_f32() {
    let adapter = make_test_adapter("Intel UHD 630", "anv", false, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(brain.route(PrecisionHint::Critical), PrecisionTier::F32);
    assert_eq!(brain.route(PrecisionHint::Moderate), PrecisionTier::F32);
    assert_eq!(
        brain.route(PrecisionHint::ThroughputBound),
        PrecisionTier::F32
    );
    assert_eq!(brain.route(PrecisionHint::LowPrecision), PrecisionTier::F32);
}

#[test]
fn brain_nvidia_proprietary_critical_skips_f64_precise() {
    let adapter = make_test_adapter("NVIDIA RTX 3090", "nvidia", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(
        brain.route(PrecisionHint::Critical),
        PrecisionTier::F64Precise
    );
    assert_eq!(brain.route(PrecisionHint::Moderate), PrecisionTier::F64);
}

#[test]
fn brain_accessor_adapter_name() {
    let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
    let cal = HardwareCalibration::from_adapter_info(&adapter);
    let brain = PrecisionBrain::new(cal, None);

    assert_eq!(brain.adapter_name(), "NVIDIA RTX 3080");
    assert!(brain.calibration().has_any_f64);
}
