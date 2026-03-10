// SPDX-License-Identifier: AGPL-3.0-only
//! Compute unit backend implementations

#[cfg(feature = "cpu")]
pub mod cpu;

#[cfg(feature = "opencl")]
pub mod opencl;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

#[cfg(feature = "wgpu-backend")]
pub mod nvvm_safety;

#[cfg(feature = "cpu")]
pub use cpu::CpuComputeUnit;

#[cfg(feature = "opencl")]
pub use opencl::OpenClComputeUnit;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::{
    GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, PrecisionRoutingAdvice,
    SubstrateCapabilityKind, WgpuComputeUnit,
};

#[cfg(feature = "wgpu-backend")]
pub use nvvm_safety::{
    check_device_health, DeviceHealthStatus, HardwareCalibration, NvvmPoisoningRisk, PrecisionTier,
    TierCapability,
};
