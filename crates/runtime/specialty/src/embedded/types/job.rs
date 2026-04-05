// SPDX-License-Identifier: AGPL-3.0-or-later
//! Embedded jobs, languages, source/output files, and related enums.
//!
//! [`EmbeddedJob`] and [`EmbeddedJobType`] model work performed on a target;
//! [`SourceFile`] and [`OutputFile`] represent inputs and build artifacts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

use crate::types::configs::embedded::PeripheralType;
use crate::{JobStatus, LegacyArchitecture, MemoryLayout, MemoryRegionType, ProgrammingInterface};

/// Embedded job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedJob {
    /// Job ID
    pub job_id: Uuid,
    /// Target architecture
    pub target_architecture: LegacyArchitecture,
    /// Job type
    pub job_type: EmbeddedJobType,
    /// Source files
    pub source_files: Vec<SourceFile>,
    /// Memory layout
    pub memory_layout: MemoryLayout,
    /// Programming interface
    pub programming_interface: ProgrammingInterface,
    /// Job status
    pub status: JobStatus,
    /// Output files
    pub output_files: Vec<OutputFile>,
    /// Compilation log
    pub compilation_log: String,
    /// Programming log
    pub programming_log: String,
    /// Start time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub start_time: Option<SystemTime>,
    /// End time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub end_time: Option<SystemTime>,
}

/// Types of embedded jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddedJobType {
    /// Compile source code for target architecture.
    Compilation {
        /// Source language (C, assembly, etc.).
        language: EmbeddedLanguage,
        /// Optimization level for the compiler.
        optimization: OptimizationLevel,
        /// Whether to include debug symbols.
        debug_info: bool,
    },
    /// Program ROM/Flash memory on target device.
    Programming {
        /// Target memory region (flash, EEPROM, etc.).
        target_memory: MemoryRegionType,
        /// Whether to verify after programming.
        verify: bool,
        /// Whether to erase before programming.
        erase_first: bool,
    },
    /// Debug session with breakpoints.
    Debugging {
        /// Debug interface (JTAG, SWD, etc.).
        debug_interface: DebugInterface,
        /// Breakpoints to set.
        breakpoints: Vec<Breakpoint>,
    },
    /// Run code in emulator.
    Emulation {
        /// Type of emulator (software, hardware, in-circuit).
        emulator_type: EmulatorType,
        /// ROM image to load.
        rom_image: Vec<u8>,
    },
    /// Dump memory region from target.
    MemoryDump {
        /// Start address for dump.
        start_address: u32,
        /// Number of bytes to dump.
        length: u32,
    },
    /// Test a peripheral device.
    PeripheralTest {
        /// Peripheral type to test.
        peripheral: PeripheralType,
        /// Type of test (functional, performance, etc.).
        test_type: PeripheralTestType,
    },
}

/// Embedded programming languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddedLanguage {
    /// Assembly language
    Assembly,
    /// C (K&R or ANSI)
    C,
    /// C++
    CPlusPlus,
    /// BASIC
    BASIC,
    /// Pascal
    Pascal,
    /// Forth
    Forth,
    /// Machine code
    MachineCode,
}

/// Optimization levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization
    None,
    /// Size optimization
    Size,
    /// Speed optimization
    Speed,
    /// Debug optimization
    Debug,
}

/// Debug interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugInterface {
    /// In-Circuit Emulator (ICE)
    ICE,
    /// JTAG
    JTAG,
    /// SWD (Serial Wire Debug)
    SWD,
    /// BDM (Background Debug Mode)
    BDM,
    /// Serial debug
    Serial,
    /// Software breakpoints
    Software,
}

/// Breakpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Breakpoint address
    pub address: u32,
    /// Breakpoint type
    pub breakpoint_type: BreakpointType,
    /// Condition (optional)
    pub condition: Option<String>,
    /// Hit count
    pub hit_count: u32,
    /// Enabled flag
    pub enabled: bool,
}

/// Breakpoint types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakpointType {
    /// Code breakpoint
    Code,
    /// Data read breakpoint
    DataRead,
    /// Data write breakpoint
    DataWrite,
    /// Data access breakpoint
    DataAccess,
}

/// Emulator types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    /// Software emulator
    Software,
    /// Hardware emulator
    Hardware,
    /// In-circuit emulator
    InCircuit,
}

/// Peripheral test types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeripheralTestType {
    /// Functional test
    Functional,
    /// Performance test
    Performance,
    /// Stress test
    Stress,
    /// Compliance test
    Compliance,
}

/// Source file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// File path
    pub path: PathBuf,
    /// File type
    pub file_type: SourceFileType,
    /// File content
    pub content: String,
    /// Include paths
    pub include_paths: Vec<PathBuf>,
    /// Preprocessor defines
    pub defines: HashMap<String, String>,
}

/// Source file types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceFileType {
    /// C source file
    C,
    /// C++ source file
    CPlusPlus,
    /// Assembly source file
    Assembly,
    /// Header file
    Header,
    /// Linker script
    LinkerScript,
    /// Configuration file
    Configuration,
}

/// Output file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// File path
    pub path: PathBuf,
    /// File type
    pub file_type: OutputFileType,
    /// File size
    pub size: u64,
    /// Load address
    pub load_address: Option<u32>,
    /// Execution address
    pub execution_address: Option<u32>,
}

/// Output file types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFileType {
    /// Binary executable
    Binary,
    /// Intel HEX file
    IntelHex,
    /// Motorola S-record
    MotorolaS,
    /// ELF file
    ELF,
    /// Object file
    Object,
    /// Library file
    Library,
    /// Map file
    Map,
    /// Listing file
    Listing,
}
