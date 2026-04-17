// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Embedded Systems Adapters
//!
//! Support for legacy embedded systems including:
//! - 8-bit microcontrollers (6502, Z80, 8080, 8051)
//! - 16-bit systems (8086, 68000)
//! - Embedded development tools
//! - Cross-compilation support
//! - Programming interfaces (ISP, ICSP, JTAG)
//! - ROM/EPROM programming
//! - Hardware debugging
//! - Memory layout management

// Module declarations
pub mod adapters;
/// Chip metadata (signatures, voltage, timing) for programmer validation.
pub mod chip_database;
pub mod cpu6502;
pub mod cpuz80;
pub mod dos;
/// Placeholder trait impls for emulators. When real hardware backends land
/// behind `embedded-hw`, add a parallel `#[cfg(feature = "embedded-hw")]`
/// module with genuine implementations and restrict this one to
/// `not(feature = "embedded-hw")`.
#[cfg(feature = "embedded-placeholder-impls")]
pub mod emulator_impls;
pub mod emulators;
pub mod errors;
pub mod managers;
/// Placeholder trait impls for programmers — same gating as `emulator_impls`.
#[cfg(feature = "embedded-placeholder-impls")]
pub mod programmer_impls;
pub mod programmers;
/// ISP / ICSP protocol logic without hardware transports (testable).
pub mod protocol;
/// Byte-level protocol sequences (AVR ISP, PIC ICSP, parallel EPROM) without I/O.
pub mod protocol_engine;
pub mod toolchains;
pub mod types;

// Re-exports
pub use adapters::{Microcontroller8BitAdapter, System16BitAdapter};
pub use dos::{DOSFileSystem, DOSInterface, DirectoryEntry, FileAllocationTable};
pub use emulators::{Emulator6502, EmulatorZ80};
pub use managers::{MemoryLayoutManager, PeripheralManager};
pub use programmers::{EPROMProgrammer, GenericProgrammer};
pub use toolchains::{
    Toolchain6502, Toolchain8051, Toolchain8080, Toolchain8086, Toolchain68000, ToolchainZ80,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        JobStatus, LegacyAdapter, LegacyArchitecture, LegacySystemType, MemoryLayout,
        ProgrammingInterface, ProgrammingInterfaceType,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_8bit_microcontroller_adapter_creation() {
        let adapter = Microcontroller8BitAdapter::new();
        assert_eq!(adapter.name(), "8-bit Microcontroller Adapter");
        assert!(
            adapter
                .supported_systems()
                .contains(&LegacySystemType::MOS6502)
        );
    }

    #[tokio::test]
    async fn test_16bit_system_adapter_creation() {
        let adapter = System16BitAdapter::new();
        assert_eq!(adapter.name(), "16-bit System Adapter");
        assert!(
            adapter
                .supported_systems()
                .contains(&LegacySystemType::Intel8086)
        );
    }

    #[tokio::test]
    async fn test_embedded_job_creation() {
        let job = EmbeddedJob {
            job_id: Uuid::new_v4(),
            target_architecture: LegacyArchitecture::MOS6502,
            job_type: EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::Assembly,
                optimization: OptimizationLevel::Size,
                debug_info: false,
            },
            source_files: vec![],
            memory_layout: MemoryLayout {
                rom_regions: vec![],
                ram_regions: vec![],
                io_regions: vec![],
            },
            programming_interface: ProgrammingInterface {
                interface_type: ProgrammingInterfaceType::ISP,
                connection_params: HashMap::new(),
            },
            status: JobStatus::Queued,
            output_files: vec![],
            compilation_log: String::new(),
            programming_log: String::new(),
            start_time: None,
            end_time: None,
        };

        assert_eq!(job.target_architecture, LegacyArchitecture::MOS6502);
        assert_eq!(job.status, JobStatus::Queued);
    }
}
