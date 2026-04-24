// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compute unit backend implementations

#[cfg(feature = "cpu")]
pub mod cpu;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

#[cfg(feature = "wgpu-backend")]
pub mod nvk_zero_guard;

#[cfg(feature = "wgpu-backend")]
pub mod spirv_codegen_safety;

#[cfg(feature = "cpu")]
pub use cpu::CpuComputeUnit;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::{
    GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, PrecisionRoutingAdvice,
    SubstrateCapabilityKind, WgpuComputeUnit,
};

#[cfg(feature = "wgpu-backend")]
pub use spirv_codegen_safety::{
    DeviceHealthStatus, HardwareCalibration, NvvmPoisoningRisk, PrecisionBrain, PrecisionHint,
    PrecisionTier, TierCapability, ZeroGuardVerdict, check_device_health, nvk_zero_guard_check,
    nvk_zero_guard_check_f32,
};

/// Type alias for forward compatibility after root-cause rename.
#[cfg(feature = "wgpu-backend")]
pub type SpirvCodegenRisk = NvvmPoisoningRisk;
