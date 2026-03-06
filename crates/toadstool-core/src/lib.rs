// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! ToadStool Core - Hardware Infrastructure Layer
//!
//! Deep Debt: ToadStool directly interfaces with hardware in Rust
//! - No scripts, no sudo needed on fresh systems
//! - Self-evolves and adapts to hardware changes
//! - `BarraCuda` runs the math on all hardware via ToadStool

pub mod hardware;
pub mod hardware_transport;
pub mod npu_controller;
pub mod npu_dispatch;
pub mod transport_router;

pub use hardware::{HardwareDevice, HardwareError, HardwareManager, HardwareType};
pub use hardware_transport::{
    decode_frame, encode_frame, HardwareTransport, TransportDirection, TransportError,
    TransportInfo, TransportMedium, FRAME_HEADER_SIZE,
};
pub use npu_controller::{
    ControllerError, NpuParameterController, ParameterSuggestion, SafetyClamp, SuggestionSource,
};
pub use npu_dispatch::{
    AkidaNpuDispatch, DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError, NpuInfo,
    NpuModelHandle,
};
pub use transport_router::{TransportFilter, TransportRouter};
