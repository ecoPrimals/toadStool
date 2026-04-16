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
    AkidaNpuDispatch, DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError,
    NpuInferenceRequest, NpuInfo, NpuModelHandle,
};
pub use silicon::{
    MultiUnitRoutingPlan, PerformanceMeasurement, PerformanceSurfaceEntry, RoutedOperation,
    RtCoreGen, SiliconCapabilities, SiliconUnit, TensorCoreGen,
};
