// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! ToadStool Core - Hardware Infrastructure Layer
//!
//! Deep Debt: ToadStool directly interfaces with hardware in Rust
//! - No scripts, no sudo needed on fresh systems
//! - Self-evolves and adapts to hardware changes
//! - Compute services run the math on all hardware via ToadStool

/// Hardware discovery and management (GPU, NPU, CPU, FPGA).
pub mod hardware;
pub mod hardware_transport;
pub mod npu_controller;
pub mod npu_dispatch;
/// GPU silicon unit discovery and performance surface types.
///
/// Every functional unit on the GPU die — shader cores, tensor cores,
/// RT cores, TMUs, ROPs, rasterizer, depth buffer, tessellator, video
/// encoder — modeled as first-class types for discovery and routing.
pub mod silicon;
/// Workload specification types — compute job definitions, executable sources,
/// AI/ML workload descriptors, CUDA kernels, validators, and analyzers.
pub mod workload;
/// Resource types — metrics, requirements, limits, system info.
pub mod resources;
/// Security types — isolation levels, capabilities, contexts, policies.
pub mod security;
/// Encryption types — security levels, payloads, config.
pub mod encryption;
/// Execution types — requests, responses, status, runtime config.
pub mod execution;
/// Canonical NUCLEUS composition manifest — `biome.yaml` schema.
pub mod manifest;

pub use hardware::{HardwareDevice, HardwareError, HardwareManager, HardwareType};
pub use hardware_transport::{
    FRAME_HEADER_SIZE, HardwareTransport, TransportDirection, TransportError, TransportInfo,
    TransportMedium, decode_frame, encode_frame,
};
pub use npu_controller::{
    AdaptiveSimulationController, ControllerError, NpuParameterController, ParameterSuggestion,
    ProxyFeature, ProxyFeatureSet, SafetyClamp, SuggestionSource,
};
pub use npu_dispatch::{
    DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError, NpuInferenceRequest, NpuInfo,
    NpuModelHandle,
};
pub use silicon::{
    MultiUnitRoutingPlan, PerformanceMeasurement, PerformanceSurfaceEntry, RoutedOperation,
    RtCoreGen, SiliconCapabilities, SiliconEnergyLedger, SiliconUnit, SiliconUnitEnergyEntry,
    SiliconUnitUtilization, TensorCoreGen,
};
pub use manifest::{
    BiomeManifest, BiomeMetadata, CompositionGraph, CompositionKind, ManifestPrimalConfig,
    ManifestServiceConfig,
};
pub use workload::{WorkloadSpec, WorkloadType};
