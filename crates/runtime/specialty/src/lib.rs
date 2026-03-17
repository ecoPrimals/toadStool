// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]

//! # ToadStool Specialty Hardware Runtime Engine
//!
//! Specialty hardware support for ToadStool Universal Compute Platform.
//!
//! This runtime engine provides execution support for:
//! - Mainframe systems (IBM System/360, VAX/VMS, AS/400, z/OS)
//! - Embedded systems (8-bit microcontrollers, 16-bit systems, Arduino, ESP32)
//! - Industrial control systems (PLCs, SCADA, real-time systems)
//! - Exotic Unix systems (PDP-11, early UNIX variants)
//! - Real-time operating systems (VxWorks, QNX, RT-11)
//!
//! ## Architecture
//!
//! ```text
//! Specialty Hardware Runtime Engine
//! ├── Mainframe Adapters (IBM, VAX, AS/400)
//! ├── Embedded Adapters (8-bit, 16-bit MCUs, Arduino, ESP32)
//! ├── Industrial Adapters (PLCs, SCADA)
//! ├── Real-time Adapters (VxWorks, QNX)
//! └── Cross-compilation Support
//! ```

pub mod config;
pub mod cross_compilation;
pub mod embedded;
pub mod emulation;
pub mod engine;
pub mod error;
pub mod industrial;
pub mod legacy_networking;
pub mod mainframe;
pub mod realtime;
pub mod runtime_bridge;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export core types
pub use toadstool::execution;
pub use toadstool::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, ResourceRequirements,
    RuntimeCapabilities, RuntimeEngine, RuntimeMetrics, RuntimeType, ToadStoolError,
    ToadStoolResult, WorkloadType,
};

// Re-export types for backward compatibility
pub use types::*;

// Disambiguate conflicting names
pub use types::configs::CompilationToolchainConfig as ToolchainConfig;
pub use types::configs::emulation::EmulationConfig as ConfigEmulationConfig;
pub use types::requirements::OptimizationLevel;

// Re-export main public API
pub use config::SpecialtyRuntimeConfig;
pub use engine::SpecialtyRuntimeEngine;
pub use error::SpecialtyRuntimeError;
