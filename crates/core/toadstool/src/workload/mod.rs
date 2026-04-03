// SPDX-License-Identifier: AGPL-3.0-only
//! Workload types and specifications

mod types;
mod validators;

mod workload_type;

pub mod ai_ml;
pub mod analyzer;
pub mod cuda;
#[cfg(test)]
pub mod integration_tests;
pub mod selector;

mod spec;

#[cfg(test)]
mod spec_tests;

pub use ai_ml::{AiFramework, AiMlWorkload, AiOperation, ModelSize, Precision};
pub use analyzer::{
    ComputeIntensity, GpuAdvantage, MemoryRequirement, ParallelismLevel, WorkloadAnalyzer,
    WorkloadCharacteristics,
};
pub use cuda::{CudaBackend, CudaLaunchConfig, CudaSource, CudaWorkload};
pub use selector::{BackendDecision, BackendSelector, GpuDevice, GpuVendor, HardwareCapabilities};
pub use spec::WorkloadSpec;
pub use types::*;
pub use workload_type::WorkloadType;
