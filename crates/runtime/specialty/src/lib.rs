// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Async stubs for future I/O implementation
#![allow(
    clippy::must_use_candidate,
    clippy::uninlined_format_args,
    clippy::too_long_first_doc_paragraph
)] // pedantic noise vs API churn

//! # ToadStool Specialty Hardware Runtime Engine
//!
//! Specialty hardware support for ToadStool Universal Compute Platform.
//!
//! This runtime engine provides execution support for:
//! - Mainframe systems (IBM System/360, VAX/VMS, AS/400, z/OS)
//! - Embedded systems (8-bit microcontrollers, 16-bit systems, Arduino, ESP32)
//! - Industrial control systems (PLCs, SCADA, real-time systems)
//! - Exotic Unix systems (PDP-11, early UNIX variants)
//! - Real-time operating systems (`VxWorks`, `QNX`, RT-11)
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

/// Configuration for specialty runtime subsystems.
pub mod config;
/// Cross-compilation toolchain and target support.
pub mod cross_compilation;
/// Embedded systems adapters and toolchains.
pub mod embedded;
/// Emulation support for legacy architectures.
pub mod emulation;
/// Core specialty runtime engine.
pub mod engine;
/// Error types for specialty runtime operations.
pub mod error;
/// Industrial control system adapters.
pub mod industrial;
/// Mainframe system adapters (IBM, VAX, AS/400).
pub mod mainframe;
/// Real-time operating system support.
pub mod realtime;
/// Bridge between specialty runtime and core platform.
pub mod runtime_bridge;
/// Type definitions for specialty systems.
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
