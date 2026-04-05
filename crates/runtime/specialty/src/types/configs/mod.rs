// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration type definitions for legacy systems
//!
//! This module organizes legacy system configuration types by domain:
//! - **compilation**: Target formats, toolchains, optimization levels
//! - **communication**: Connection types, authentication, protocols
//! - **terminal**: Terminal types, session configs, encodings
//! - **storage**: Paper tape, ROM, disk image formats
//! - **management**: Job priorities, monitoring, administration
//! - **mainframe**: IBM mainframe-specific configuration
//! - **embedded**: Embedded systems configuration
//! - **industrial**: Industrial control systems (PLC, SCADA, etc.)
//! - **realtime**: Real-time operating system configuration
//! - **emulation**: Emulator configuration

pub mod communication;
pub mod compilation;
pub mod embedded;
pub mod emulation;
pub mod industrial;
pub mod mainframe;
pub mod management;
pub mod realtime;
pub mod storage;
pub mod terminal;

// Re-export all public types for backward compatibility.
// Explicitly exclude names that conflict with jobs, requirements, cross_compilation, emulation:
// - compilation::TargetFormat, OptimizationLevel, ToolchainConfig (conflict with jobs, requirements, cross_compilation)
// - management::TransferType, MonitoringType, AdministrationType (conflict with jobs)
// - storage::PaperTapeFormat, ROMFormat (conflict with jobs)
// - embedded::PeripheralConfig (conflict with emulation)
// - emulation::EmulationConfig (conflict with types::emulation)
pub use communication::*;
pub use compilation::{
    OptimizationLevel as CompilationOptimizationLevel, TargetFormat as CompilationTargetFormat,
    ToolchainConfig as CompilationToolchainConfig,
};
pub use embedded::{
    EmbeddedConfig, MemoryLayout, MemoryPermissions, MemoryRegion, MemoryRegionType,
    PeripheralConfig as EmbeddedPeripheralConfig, PeripheralType,
};
pub use emulation::{EmulationConfig as ConfigEmulationConfig, EmulatorType};
pub use industrial::*;
pub use mainframe::*;
pub use management::{
    AdministrationType as ManagementAdministrationType, JobPriority as ManagementJobPriority,
    MonitoringType as ManagementMonitoringType, TransferType as ManagementTransferType,
};
pub use realtime::*;
pub use storage::{
    DiskImage, DiskImageType, PaperTapeFormat as StoragePaperTapeFormat, ROMFile,
    ROMFormat as StorageROMFormat,
};
pub use terminal::{
    CharacterEncoding, FlowControl, LineEnding, SessionConfig, TerminalType as ConfigTerminalType,
};
