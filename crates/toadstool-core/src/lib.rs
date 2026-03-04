// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! ToadStool Core - Hardware Infrastructure Layer
//!
//! Deep Debt: ToadStool directly interfaces with hardware in Rust
//! - No scripts, no sudo needed on fresh systems
//! - Self-evolves and adapts to hardware changes
//! - `BarraCuda` runs the math on all hardware via ToadStool

pub mod hardware;
pub mod npu_controller;
pub mod npu_dispatch;

pub use hardware::{HardwareDevice, HardwareError, HardwareManager, HardwareType};
pub use npu_controller::{
    ControllerError, NpuParameterController, ParameterSuggestion, SafetyClamp, SuggestionSource,
};
pub use npu_dispatch::{
    AkidaNpuDispatch, DispatchResult, NpuCapability, NpuDispatch, NpuDispatchError, NpuInfo,
    NpuModelHandle,
};
