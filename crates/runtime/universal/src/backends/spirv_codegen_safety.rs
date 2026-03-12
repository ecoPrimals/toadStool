// SPDX-License-Identifier: AGPL-3.0-only
//! SPIR-V codegen safety — transcendental poisoning defense.
//!
//! Root cause: naga SPIR-V codegen (not NVVM) produces incorrect transcendental
//! operations on proprietary NVIDIA drivers. Renamed from `nvvm_safety` per
//! hotSpring v0.6.30 root-cause clarification.
//!
//! Absorbed from hotSpring v0.6.25 handoff: proprietary NVIDIA drivers may
//! poison the device when running shaders that use DF64/F64Precise
//! transcendentals (exp, log) through naga-generated SPIR-V. NVK/Mesa and
//! AMD radv handle all tiers correctly.
//!
//! This module provides driver-aware calibration so callers can avoid
//! transcendental compilation on risky drivers without probing (probing risks
//! poisoning).

use super::wgpu_backend::GpuAdapterInfo;

/// NVVM poisoning risk classification inferred from driver identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvvmPoisoningRisk {
    /// Safe: NVK/Mesa driver handles all tiers correctly.
    None,
    /// Risk: proprietary NVIDIA driver; DF64/F64Precise transcendentals may poison device.
    TranscendentalOnly,
    /// Unknown driver — treat as risky.
    Unknown,
}

/// Precision tier (from hotSpring).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecisionTier {
    F32,
    F64,
    F64Precise,
    Df64,
}

/// Capability of a single precision tier.
#[derive(Debug, Clone)]
pub struct TierCapability {
    pub tier: PrecisionTier,
    /// Whether the tier compiles without poisoning the device.
    pub compiles: bool,
    /// Whether the tier dispatches and produces correct results.
    pub dispatches: bool,
    /// Whether f64 transcendentals (exp/log) are safe at this tier.
    /// Inferred from driver identity, not probed (probing risks poisoning).
    pub transcendentals_safe: bool,
    /// Dispatch latency ratio vs F32 baseline (1.0 = same speed).
    /// Used by `PrecisionBrain` for F64 throttle detection.
    /// Default 1.0 when unknown; set from runtime calibration probes.
    pub dispatch_latency_ratio: f64,
}

/// Hardware calibration for NVVM safety.
///
/// Built from adapter info; used to select safe precision tiers and avoid
/// transcendental compilation on drivers that may poison the device.
#[derive(Debug, Clone)]
pub struct HardwareCalibration {
    pub adapter_name: String,
    pub driver: String,
    pub tiers: Vec<TierCapability>,
    /// Whether any f64 tier is available.
    pub has_any_f64: bool,
    /// Whether DF64 arithmetic (no transcendentals) is safe.
    pub df64_arithmetic_safe: bool,
    /// Whether NVVM transcendental compilation risks device poisoning.
    pub nvvm_transcendental_risk: bool,
    /// Poisoning risk classification.
    pub poisoning_risk: NvvmPoisoningRisk,
}

impl HardwareCalibration {
    /// Build calibration from GPU adapter info.
    ///
    /// Driver classification:
    /// - NVK/Mesa → all tiers safe, transcendentals safe, `NvvmPoisoningRisk::None`
    /// - AMD (radv) → all tiers safe where f64 supported, `NvvmPoisoningRisk::None`
    /// - NVIDIA proprietary → F32 safe, F64 safe, F64Precise/DF64 arithmetic only,
    ///   transcendentals unsafe at F64Precise and DF64, `NvvmPoisoningRisk::TranscendentalOnly`
    /// - Unknown → `NvvmPoisoningRisk::Unknown`
    #[must_use]
    pub fn from_adapter_info(info: &GpuAdapterInfo) -> Self {
        let adapter_name = info.name.clone();
        let driver = info.driver.clone();

        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_radv = driver.contains("radv");
        let is_nvidia_proprietary = driver.contains("nvidia") && !driver.contains("nvk");
        let is_known = is_nvk || is_radv || is_nvidia_proprietary;

        let supports_f64 = info.supports_shader_f64 && !info.f64_compute_unreliable;

        let poisoning_risk = if is_nvk || is_radv {
            NvvmPoisoningRisk::None
        } else if is_nvidia_proprietary {
            NvvmPoisoningRisk::TranscendentalOnly
        } else {
            NvvmPoisoningRisk::Unknown
        };

        let mut tiers = vec![TierCapability {
            tier: PrecisionTier::F32,
            compiles: true,
            dispatches: true,
            transcendentals_safe: true,
            dispatch_latency_ratio: 1.0,
        }];

        if supports_f64 {
            tiers.push(TierCapability {
                tier: PrecisionTier::F64,
                compiles: true,
                dispatches: true,
                transcendentals_safe: is_known && (is_nvk || is_radv || is_nvidia_proprietary),
                dispatch_latency_ratio: 1.0,
            });

            let f64_precise_transcendentals_safe = is_nvk || is_radv;
            tiers.push(TierCapability {
                tier: PrecisionTier::F64Precise,
                compiles: true,
                dispatches: true,
                transcendentals_safe: f64_precise_transcendentals_safe,
                dispatch_latency_ratio: 1.0,
            });

            let df64_transcendentals_safe = is_nvk || is_radv;
            let df64_arithmetic_safe = is_known;
            tiers.push(TierCapability {
                tier: PrecisionTier::Df64,
                compiles: true,
                dispatches: true,
                transcendentals_safe: df64_transcendentals_safe,
                dispatch_latency_ratio: 1.0,
            });

            Self {
                adapter_name,
                driver,
                tiers,
                has_any_f64: true,
                df64_arithmetic_safe,
                nvvm_transcendental_risk: !df64_transcendentals_safe && supports_f64,
                poisoning_risk,
            }
        } else {
            Self {
                adapter_name,
                driver,
                tiers,
                has_any_f64: false,
                df64_arithmetic_safe: false,
                nvvm_transcendental_risk: false,
                poisoning_risk,
            }
        }
    }

    /// Whether the given tier is safe for the workload.
    #[must_use]
    pub fn is_tier_safe(&self, tier: PrecisionTier, uses_transcendentals: bool) -> bool {
        let Some(tc) = self.tiers.iter().find(|t| t.tier == tier) else {
            return false;
        };
        if !tc.compiles || !tc.dispatches {
            return false;
        }
        if uses_transcendentals && !tc.transcendentals_safe {
            return false;
        }
        true
    }

    /// Returns the best precision tier for a workload.
    ///
    /// - `precision_critical`: prefer F64Precise > F64 > DF64 > F32
    /// - `throughput_bound`: prefer F64 > DF64 > F32 (skip F64Precise overhead)
    #[must_use]
    pub fn best_tier(&self, precision_critical: bool, uses_transcendentals: bool) -> PrecisionTier {
        let order = if precision_critical {
            [
                PrecisionTier::F64Precise,
                PrecisionTier::F64,
                PrecisionTier::Df64,
                PrecisionTier::F32,
            ]
        } else {
            [
                PrecisionTier::F64,
                PrecisionTier::Df64,
                PrecisionTier::F32,
                PrecisionTier::F64Precise,
            ]
        };

        for t in order {
            if self.is_tier_safe(t, uses_transcendentals) {
                return t;
            }
        }

        PrecisionTier::F32
    }
}

/// Precision requirement hint from science domains.
///
/// Domain-agnostic — callers (springs, barraCuda) classify their workload
/// into one of these categories. `PrecisionBrain` maps the hint to the
/// best available `PrecisionTier` on the current hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecisionHint {
    /// Requires maximum precision (dielectric response, eigensolve, NLME FOCE).
    /// Routing: F64Precise → F64 → DF64 → F32.
    Critical,
    /// Needs f64 but not maximum precision (gradient flow, nuclear EOS, PK/PD).
    /// Routing: F64 → DF64 → F32.
    Moderate,
    /// Throughput-limited; f64 acceptable but not at performance cost.
    /// Routing: F64 (unless throttled) → DF64 → F32.
    ThroughputBound,
    /// f32 is sufficient (visualization, preprocessing, approximate).
    /// Routing: F32 only.
    LowPrecision,
}

/// Domain-aware precision routing brain (absorbed from hotSpring v0.6.25).
///
/// Builds a cached routing table from `HardwareCalibration` so callers get
/// O(1) tier lookups. Encodes the F64-throttle heuristic: when F64 dispatch
/// latency exceeds 8× F32 latency, throughput-bound workloads prefer DF64.
#[derive(Debug, Clone)]
pub struct PrecisionBrain {
    calibration: HardwareCalibration,
    /// Pre-computed tier for each `PrecisionHint` variant.
    /// Index: Critical=0, Moderate=1, ThroughputBound=2, LowPrecision=3.
    route_table: [PrecisionTier; 4],
}

impl PrecisionBrain {
    /// Build a routing brain from hardware calibration.
    ///
    /// `f64_throttle_ratio` is the F64/F32 dispatch latency ratio above which
    /// throughput-bound workloads prefer DF64 over F64. Pass `None` for the
    /// default threshold of 8.0 (from hotSpring empirical measurement).
    #[must_use]
    pub fn new(calibration: HardwareCalibration, f64_throttle_ratio: Option<f64>) -> Self {
        let threshold = f64_throttle_ratio.unwrap_or(8.0);
        let f64_throttled = Self::detect_f64_throttle(&calibration, threshold);
        let route_table = Self::build_route_table(&calibration, f64_throttled);

        Self {
            calibration,
            route_table,
        }
    }

    /// O(1) tier lookup for a precision hint.
    #[must_use]
    pub fn route(&self, hint: PrecisionHint) -> PrecisionTier {
        self.route_table[hint as usize]
    }

    /// Whether the routed tier for this hint uses transcendentals safely.
    #[must_use]
    pub fn transcendentals_safe(&self, hint: PrecisionHint) -> bool {
        self.calibration
            .is_tier_safe(self.route(hint), true)
    }

    /// Access the underlying calibration.
    #[must_use]
    pub fn calibration(&self) -> &HardwareCalibration {
        &self.calibration
    }

    /// Adapter name from calibration.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.calibration.adapter_name
    }

    fn detect_f64_throttle(cal: &HardwareCalibration, threshold: f64) -> bool {
        let f32_cap = cal.tiers.iter().find(|t| t.tier == PrecisionTier::F32);
        let f64_cap = cal.tiers.iter().find(|t| t.tier == PrecisionTier::F64);
        match (f32_cap, f64_cap) {
            (Some(f32_t), Some(f64_t)) if f32_t.dispatches && f64_t.dispatches => {
                f64_t.dispatch_latency_ratio > threshold
            }
            _ => false,
        }
    }

    fn build_route_table(cal: &HardwareCalibration, f64_throttled: bool) -> [PrecisionTier; 4] {
        [
            // Critical: F64Precise → F64 → DF64 → F32
            Self::first_safe(
                cal,
                &[
                    PrecisionTier::F64Precise,
                    PrecisionTier::F64,
                    PrecisionTier::Df64,
                    PrecisionTier::F32,
                ],
            ),
            // Moderate: F64 → DF64 → F32
            Self::first_safe(
                cal,
                &[PrecisionTier::F64, PrecisionTier::Df64, PrecisionTier::F32],
            ),
            // ThroughputBound: F64 (unless throttled) → DF64 → F32
            if f64_throttled {
                Self::first_safe(
                    cal,
                    &[PrecisionTier::Df64, PrecisionTier::F64, PrecisionTier::F32],
                )
            } else {
                Self::first_safe(
                    cal,
                    &[PrecisionTier::F64, PrecisionTier::Df64, PrecisionTier::F32],
                )
            },
            // LowPrecision: F32
            PrecisionTier::F32,
        ]
    }

    fn first_safe(cal: &HardwareCalibration, order: &[PrecisionTier]) -> PrecisionTier {
        for &tier in order {
            if cal.is_tier_safe(tier, false) {
                return tier;
            }
        }
        PrecisionTier::F32
    }
}

/// Fleet-level precision analysis.
///
/// Given a collection of `PrecisionBrain` instances (one per GPU in the fleet),
/// identify learning opportunities where working GPUs can help blocked ones.
#[cfg(feature = "hardware-learning")]
pub mod fleet {
    use super::*;
    use hw_learn::brain_ext::learning_advisor::{FleetGpu, LearningAdvisor, LearningOpportunity};
    use hw_learn::distiller::{GpuArch, Vendor};
    use toadstool_sysmon::{GpuDevice, FirmwareInventory};

    /// A GPU in the fleet with both precision calibration and firmware status.
    pub struct FleetMember {
        pub device: GpuDevice,
        pub brain: PrecisionBrain,
        pub firmware: FirmwareInventory,
    }

    /// Identify learning opportunities across a fleet of GPUs.
    ///
    /// Bridges PrecisionBrain's per-GPU calibration knowledge with
    /// hw-learn's fleet-level learning advisor.
    pub fn learning_opportunities(fleet: &[FleetMember]) -> Vec<LearningOpportunity> {
        let fleet_gpus: Vec<FleetGpu> = fleet
            .iter()
            .map(|m| {
                let vendor = match m.device.vendor {
                    toadstool_sysmon::GpuVendor::Amd => Vendor::Amd,
                    toadstool_sysmon::GpuVendor::Intel => Vendor::Intel,
                    toadstool_sysmon::GpuVendor::Nvidia => Vendor::Nvidia,
                    toadstool_sysmon::GpuVendor::Unknown => Vendor::Nvidia, // conservative
                };

                let compute_works = m.firmware.compute_viable && m.brain.calibration().has_any_f64;

                FleetGpu {
                    id: format!("card{}", m.device.card_index),
                    arch: GpuArch {
                        vendor,
                        generation: infer_generation(&m.brain.calibration().adapter_name),
                        chip: format!("dev{:04x}", m.device.device_id),
                        compute_class: infer_compute_class(&m.brain.calibration().adapter_name),
                    },
                    firmware: m.firmware.clone(),
                    compute_works,
                    driver: m.device.driver.clone(),
                }
            })
            .collect();

        let advisor = LearningAdvisor::new(fleet_gpus);
        advisor.opportunities()
    }

    fn infer_generation(adapter_name: &str) -> String {
        let name = adapter_name.to_uppercase();
        if name.contains("RTX 40") || name.contains("AD1") {
            "Ada".into()
        } else if name.contains("RTX 30") || name.contains("GA1") {
            "Ampere".into()
        } else if name.contains("RTX 20") || name.contains("TU1") {
            "Turing".into()
        } else if name.contains("TITAN V") || name.contains("GV1") {
            "Volta".into()
        } else if name.contains("RX 6") || name.contains("NAVI") {
            "RDNA2".into()
        } else if name.contains("RX 7") {
            "RDNA3".into()
        } else if name.contains("ARC") || name.contains("DG2") {
            "Alchemist".into()
        } else {
            "Unknown".into()
        }
    }

    fn infer_compute_class(adapter_name: &str) -> String {
        let name = adapter_name.to_uppercase();
        if name.contains("RTX 40") { "sm89".into() }
        else if name.contains("RTX 30") { "sm86".into() }
        else if name.contains("RTX 20") { "sm75".into() }
        else if name.contains("TITAN V") { "sm70".into() }
        else if name.contains("RX 6") { "gfx1030".into() }
        else if name.contains("RX 7") { "gfx1100".into() }
        else if name.contains("ARC") { "gen12".into() }
        else { "unknown".into() }
    }
}

/// NVK zero-output guard (absorbed from airSpring v0.7.5).
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
pub fn check_device_health(
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
    use crate::backends::wgpu_backend::{
        GpuDeviceType, HardwareFingerprint, SubstrateCapabilityKind,
    };

    fn make_test_adapter(
        name: &str,
        driver: &str,
        supports_f64: bool,
        f64_unreliable: bool,
    ) -> GpuAdapterInfo {
        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_ada =
            name.contains("RTX 40") || name.contains("RTX 4070") || name.contains("RTX 4090");
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

    // ── PrecisionBrain tests ──────────────────────────────────

    #[test]
    fn brain_nvk_critical_routes_f64_precise() {
        let adapter = make_test_adapter("NVIDIA RTX 3080", "nvk", true, false);
        let cal = HardwareCalibration::from_adapter_info(&adapter);
        let brain = PrecisionBrain::new(cal, None);

        assert_eq!(brain.route(PrecisionHint::Critical), PrecisionTier::F64Precise);
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

        assert_eq!(brain.route(PrecisionHint::ThroughputBound), PrecisionTier::F64);
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
        assert_eq!(brain.route(PrecisionHint::ThroughputBound), PrecisionTier::F32);
        assert_eq!(brain.route(PrecisionHint::LowPrecision), PrecisionTier::F32);
    }

    #[test]
    fn brain_nvidia_proprietary_critical_skips_f64_precise() {
        let adapter = make_test_adapter("NVIDIA RTX 3090", "nvidia", true, false);
        let cal = HardwareCalibration::from_adapter_info(&adapter);
        let brain = PrecisionBrain::new(cal, None);

        assert_eq!(brain.route(PrecisionHint::Critical), PrecisionTier::F64Precise);
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

    // ── NvkZeroGuard tests ────────────────────────────────────

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
        assert_eq!(nvk_zero_guard_check(&output), ZeroGuardVerdict::NanContaminated);
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
        assert_eq!(nvk_zero_guard_check_f32(&output), ZeroGuardVerdict::AllZeros);
    }

    #[test]
    fn zero_guard_f32_valid() {
        let output: [f32; 3] = [1.0, 0.0, 0.5];
        assert_eq!(nvk_zero_guard_check_f32(&output), ZeroGuardVerdict::Valid);
    }

    #[test]
    fn zero_guard_f32_nan() {
        let output: [f32; 2] = [f32::NAN, 1.0];
        assert_eq!(nvk_zero_guard_check_f32(&output), ZeroGuardVerdict::NanContaminated);
    }
}
