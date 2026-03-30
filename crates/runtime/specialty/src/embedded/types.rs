// SPDX-License-Identifier: AGPL-3.0-only
//! Type definitions for embedded systems support
//!
//! This module contains all type definitions for embedded system adapters,
//! including job types, languages, debugging interfaces, and file representations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

use crate::types::configs::embedded::{PeripheralConfig, PeripheralType};
use crate::{
    JobStatus, LegacyArchitecture, MemoryLayout, MemoryPermissions, MemoryRegionType,
    ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult,
};

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

/// Embedded toolchain trait
#[async_trait::async_trait]
pub trait EmbeddedToolchain: Send + Sync + std::fmt::Debug {
    /// Get toolchain name
    fn name(&self) -> &'static str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize toolchain
    async fn initialize(&mut self, config: &crate::EmbeddedConfig) -> ToadStoolResult<()>;

    /// Compile source code
    async fn compile(
        &self,
        sources: &[SourceFile],
        output_path: &Path,
    ) -> ToadStoolResult<CompilationResult>;

    /// Link object files
    async fn link(
        &self,
        objects: &[PathBuf],
        output_path: &Path,
        memory_layout: &MemoryLayout,
    ) -> ToadStoolResult<LinkResult>;

    /// Generate ROM image
    async fn generate_rom_image(
        &self,
        executable: &Path,
        format: OutputFileType,
    ) -> ToadStoolResult<Vec<u8>>;

    /// Disassemble binary
    async fn disassemble(&self, binary: &[u8], start_address: u32) -> ToadStoolResult<String>;

    /// Create memory map
    async fn create_memory_map(&self, executable: &Path) -> ToadStoolResult<MemoryMap>;
}

/// Compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    /// Success flag
    pub success: bool,
    /// Output files
    pub output_files: Vec<OutputFile>,
    /// Compiler messages
    pub messages: Vec<CompilerMessage>,
    /// Compilation time
    pub compilation_time: Duration,
    /// Memory usage
    pub memory_usage: MemoryUsage,
}

/// Link result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    /// Success flag
    pub success: bool,
    /// Output executable
    pub executable: Option<PathBuf>,
    /// Memory map
    pub memory_map: Option<MemoryMap>,
    /// Linker messages
    pub messages: Vec<LinkerMessage>,
    /// Link time
    pub link_time: Duration,
}

/// Compiler message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerMessage {
    /// Message type
    pub message_type: MessageType,
    /// Source file
    pub source_file: Option<PathBuf>,
    /// Line number
    pub line_number: Option<u32>,
    /// Column number
    pub column_number: Option<u32>,
    /// Message text
    pub message: String,
}

/// Linker message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkerMessage {
    /// Message type
    pub message_type: MessageType,
    /// Section name
    pub section: Option<String>,
    /// Symbol name
    pub symbol: Option<String>,
    /// Message text
    pub message: String,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Error message
    Error,
    /// Warning message
    Warning,
    /// Information message
    Info,
    /// Debug message
    Debug,
}

/// Memory usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// ROM/Flash usage
    pub rom_usage: RegionUsage,
    /// RAM usage
    pub ram_usage: RegionUsage,
    /// EEPROM usage
    pub eeprom_usage: Option<RegionUsage>,
}

/// Memory region usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionUsage {
    /// Used bytes
    pub used: u32,
    /// Total bytes
    pub total: u32,
    /// Usage percentage
    pub percentage: f32,
}

impl Default for RegionUsage {
    fn default() -> Self {
        Self {
            used: 0,
            total: 0,
            percentage: 0.0,
        }
    }
}

/// Memory map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMap {
    /// Memory regions
    pub regions: Vec<MemoryMapRegion>,
    /// Symbols
    pub symbols: Vec<Symbol>,
    /// Sections
    pub sections: Vec<Section>,
}

/// Memory map region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMapRegion {
    /// Region name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// End address
    pub end_address: u32,
    /// Size
    pub size: u32,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Permissions
    pub permissions: MemoryPermissions,
}

/// Symbol definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol address
    pub address: u32,
    /// Symbol size
    pub size: u32,
    /// Symbol type
    pub symbol_type: SymbolType,
    /// Symbol section
    pub section: Option<String>,
}

/// Symbol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    /// Function symbol
    Function,
    /// Variable symbol
    Variable,
    /// Constant symbol
    Constant,
    /// Label symbol
    Label,
    /// Section symbol
    Section,
}

/// Section definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Section name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// Size
    pub size: u32,
    /// Section type
    pub section_type: SectionType,
    /// Alignment
    pub alignment: u32,
}

/// Section types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionType {
    /// Code section
    Code,
    /// Data section
    Data,
    /// BSS section
    BSS,
    /// Read-only data section
    ReadOnlyData,
    /// Stack section
    Stack,
    /// Heap section
    Heap,
    /// Custom section with user-defined name.
    Custom {
        /// Section name.
        name: String,
    },
}

/// Programmer interface trait
#[async_trait::async_trait]
pub trait ProgrammerInterface: Send + Sync + std::fmt::Debug {
    /// Get programmer name
    fn name(&self) -> &'static str;

    /// Get supported interfaces
    fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType>;

    /// Initialize programmer
    async fn initialize(&mut self, config: &ProgrammingInterface) -> ToadStoolResult<()>;

    /// Connect to target
    async fn connect(&mut self) -> ToadStoolResult<()>;

    /// Disconnect from target
    async fn disconnect(&mut self) -> ToadStoolResult<()>;

    /// Read memory
    async fn read_memory(&mut self, address: u32, length: u32) -> ToadStoolResult<Vec<u8>>;

    /// Write memory
    async fn write_memory(&mut self, address: u32, data: &[u8]) -> ToadStoolResult<()>;

    /// Erase memory
    async fn erase_memory(&mut self, address: u32, length: u32) -> ToadStoolResult<()>;

    /// Verify memory
    async fn verify_memory(&mut self, address: u32, expected_data: &[u8]) -> ToadStoolResult<bool>;

    /// Get target information
    async fn get_target_info(&self) -> ToadStoolResult<TargetInfo>;
}

/// Target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    /// Target name
    pub name: String,
    /// Target architecture
    pub architecture: LegacyArchitecture,
    /// Flash size
    pub flash_size: u32,
    /// RAM size
    pub ram_size: u32,
    /// EEPROM size
    pub eeprom_size: Option<u32>,
    /// CPU speed
    pub cpu_speed: u32,
    /// Supported features
    pub features: Vec<String>,
}

/// Embedded emulator trait
#[async_trait::async_trait]
pub trait EmbeddedEmulator: Send + Sync + std::fmt::Debug {
    /// Get emulator name
    fn name(&self) -> &'static str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize emulator
    async fn initialize(&mut self, config: &crate::EmbeddedConfig) -> ToadStoolResult<()>;

    /// Load ROM image
    async fn load_rom(&mut self, rom_data: &[u8], load_address: u32) -> ToadStoolResult<()>;

    /// Start emulation
    async fn start(&mut self) -> ToadStoolResult<()>;

    /// Stop emulation
    async fn stop(&mut self) -> ToadStoolResult<()>;

    /// Step instruction
    async fn step(&mut self) -> ToadStoolResult<()>;

    /// Set breakpoint
    async fn set_breakpoint(&mut self, address: u32) -> ToadStoolResult<()>;

    /// Clear breakpoint
    async fn clear_breakpoint(&mut self, address: u32) -> ToadStoolResult<()>;

    /// Read CPU registers
    async fn read_registers(&self) -> ToadStoolResult<CpuRegisters>;

    /// Write CPU registers
    async fn write_registers(&mut self, registers: &CpuRegisters) -> ToadStoolResult<()>;

    /// Read memory
    async fn read_memory(&self, address: u32, length: u32) -> ToadStoolResult<Vec<u8>>;

    /// Write memory
    async fn write_memory(&mut self, address: u32, data: &[u8]) -> ToadStoolResult<()>;

    /// Get emulation status
    async fn get_status(&self) -> ToadStoolResult<EmulationStatus>;
}

/// CPU registers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRegisters {
    /// General purpose registers
    pub general_purpose: HashMap<String, u32>,
    /// Program counter
    pub program_counter: u32,
    /// Stack pointer
    pub stack_pointer: u32,
    /// Status register
    pub status_register: u32,
    /// Special registers
    pub special: HashMap<String, u32>,
}

/// Emulation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulationStatus {
    /// Emulation is running
    Running,
    /// Emulation is stopped
    Stopped,
    /// Emulation is paused at breakpoint
    Breakpoint {
        /// Address where execution stopped.
        address: u32,
    },
    /// Emulation error
    Error {
        /// Error description.
        message: String,
    },
}

/// Peripheral interface trait
#[async_trait::async_trait]
pub trait PeripheralInterface: Send + Sync + std::fmt::Debug {
    /// Get peripheral name
    fn name(&self) -> &'static str;

    /// Get peripheral type
    fn peripheral_type(&self) -> PeripheralType;

    /// Initialize peripheral
    async fn initialize(&mut self, config: &PeripheralConfig) -> ToadStoolResult<()>;

    /// Read from peripheral
    async fn read(&self, address: u32) -> ToadStoolResult<u32>;

    /// Write to peripheral
    async fn write(&mut self, address: u32, value: u32) -> ToadStoolResult<()>;

    /// Reset peripheral
    async fn reset(&mut self) -> ToadStoolResult<()>;

    /// Get peripheral status
    async fn get_status(&self) -> ToadStoolResult<PeripheralStatus>;
}

/// Peripheral status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralStatus {
    /// Peripheral name
    pub name: String,
    /// Peripheral type
    pub peripheral_type: PeripheralType,
    /// Status
    pub status: String,
    /// Register values
    pub registers: HashMap<String, u32>,
    /// Interrupt status
    pub interrupt_status: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        JobStatus, LegacyArchitecture, MemoryLayout, MemoryPermissions, MemoryRegionType,
        ProgrammingInterface, ProgrammingInterfaceType,
    };
    use std::collections::HashMap as StdHashMap;

    /// Asserts JSON serialization is stable across serialize → deserialize → serialize.
    fn assert_serde_json_stable<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serde_json serialize");
        let back: T = serde_json::from_str(&json).expect("serde_json deserialize");
        let json_again = serde_json::to_string(&back).expect("serde_json re-serialize");
        assert_eq!(
            json, json_again,
            "serde round-trip must preserve JSON representation"
        );
    }

    fn sample_memory_layout() -> MemoryLayout {
        MemoryLayout {
            rom_regions: vec![],
            ram_regions: vec![],
            io_regions: vec![],
        }
    }

    fn sample_programming_interface() -> ProgrammingInterface {
        ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: StdHashMap::new(),
        }
    }

    #[test]
    fn memory_usage_default_matches_region_usage_defaults() {
        let u = MemoryUsage::default();
        let z = RegionUsage::default();
        assert_eq!(u.rom_usage.used, z.used);
        assert_eq!(u.rom_usage.total, z.total);
        assert!((u.rom_usage.percentage - z.percentage).abs() < f32::EPSILON);
        assert_eq!(u.ram_usage.used, z.used);
        assert!(u.eeprom_usage.is_none());
    }

    #[test]
    fn region_usage_default_is_zeroed() {
        let r = RegionUsage::default();
        assert_eq!(r.used, 0);
        assert_eq!(r.total, 0);
        assert!((r.percentage - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn embedded_job_serde_roundtrip() {
        let job = EmbeddedJob {
            job_id: Uuid::nil(),
            target_architecture: LegacyArchitecture::MOS6502,
            job_type: EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::Assembly,
                optimization: OptimizationLevel::Size,
                debug_info: true,
            },
            source_files: vec![SourceFile {
                path: PathBuf::from("src/main.asm"),
                file_type: SourceFileType::Assembly,
                content: "nop".to_string(),
                include_paths: vec![PathBuf::from("inc")],
                defines: HashMap::from([("BOARD".to_string(), "1".to_string())]),
            }],
            memory_layout: sample_memory_layout(),
            programming_interface: sample_programming_interface(),
            status: JobStatus::Queued,
            output_files: vec![OutputFile {
                path: PathBuf::from("out.bin"),
                file_type: OutputFileType::Binary,
                size: 4,
                load_address: Some(0x8000),
                execution_address: Some(0x8000),
            }],
            compilation_log: String::new(),
            programming_log: String::new(),
            start_time: None,
            end_time: None,
        };
        assert_serde_json_stable(&job);
    }

    #[test]
    fn embedded_job_type_variants_roundtrip() {
        let cases = vec![
            EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::C,
                optimization: OptimizationLevel::Speed,
                debug_info: false,
            },
            EmbeddedJobType::Programming {
                target_memory: MemoryRegionType::Flash,
                verify: true,
                erase_first: false,
            },
            EmbeddedJobType::Debugging {
                debug_interface: DebugInterface::JTAG,
                breakpoints: vec![Breakpoint {
                    address: 0x1000,
                    breakpoint_type: BreakpointType::Code,
                    condition: None,
                    hit_count: 0,
                    enabled: true,
                }],
            },
            EmbeddedJobType::Emulation {
                emulator_type: EmulatorType::Software,
                rom_image: vec![0xea, 0x4c],
            },
            EmbeddedJobType::MemoryDump {
                start_address: 0,
                length: 256,
            },
            EmbeddedJobType::PeripheralTest {
                peripheral: PeripheralType::UART,
                test_type: PeripheralTestType::Functional,
            },
        ];
        for job_type in cases {
            assert_serde_json_stable(&job_type);
        }
    }

    #[test]
    fn embedded_language_variants_roundtrip() {
        for lang in [
            EmbeddedLanguage::Assembly,
            EmbeddedLanguage::C,
            EmbeddedLanguage::CPlusPlus,
            EmbeddedLanguage::BASIC,
            EmbeddedLanguage::Pascal,
            EmbeddedLanguage::Forth,
            EmbeddedLanguage::MachineCode,
        ] {
            assert_serde_json_stable(&lang);
        }
    }

    #[test]
    fn optimization_level_variants_roundtrip() {
        for level in [
            OptimizationLevel::None,
            OptimizationLevel::Size,
            OptimizationLevel::Speed,
            OptimizationLevel::Debug,
        ] {
            assert_serde_json_stable(&level);
        }
    }

    #[test]
    fn debug_interface_variants_roundtrip() {
        for iface in [
            DebugInterface::ICE,
            DebugInterface::JTAG,
            DebugInterface::SWD,
            DebugInterface::BDM,
            DebugInterface::Serial,
            DebugInterface::Software,
        ] {
            assert_serde_json_stable(&iface);
        }
    }

    #[test]
    fn breakpoint_type_variants_roundtrip() {
        for t in [
            BreakpointType::Code,
            BreakpointType::DataRead,
            BreakpointType::DataWrite,
            BreakpointType::DataAccess,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn emulator_type_variants_roundtrip() {
        for t in [
            EmulatorType::Software,
            EmulatorType::Hardware,
            EmulatorType::InCircuit,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn peripheral_test_type_variants_roundtrip() {
        for t in [
            PeripheralTestType::Functional,
            PeripheralTestType::Performance,
            PeripheralTestType::Stress,
            PeripheralTestType::Compliance,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn source_file_type_variants_roundtrip() {
        for t in [
            SourceFileType::C,
            SourceFileType::CPlusPlus,
            SourceFileType::Assembly,
            SourceFileType::Header,
            SourceFileType::LinkerScript,
            SourceFileType::Configuration,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn output_file_type_variants_roundtrip() {
        for t in [
            OutputFileType::Binary,
            OutputFileType::IntelHex,
            OutputFileType::MotorolaS,
            OutputFileType::ELF,
            OutputFileType::Object,
            OutputFileType::Library,
            OutputFileType::Map,
            OutputFileType::Listing,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn message_type_variants_roundtrip() {
        for t in [
            MessageType::Error,
            MessageType::Warning,
            MessageType::Info,
            MessageType::Debug,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn symbol_type_variants_roundtrip() {
        for t in [
            SymbolType::Function,
            SymbolType::Variable,
            SymbolType::Constant,
            SymbolType::Label,
            SymbolType::Section,
        ] {
            assert_serde_json_stable(&t);
        }
    }

    #[test]
    fn section_type_variants_roundtrip() {
        let cases = vec![
            SectionType::Code,
            SectionType::Data,
            SectionType::BSS,
            SectionType::ReadOnlyData,
            SectionType::Stack,
            SectionType::Heap,
            SectionType::Custom {
                name: ".vectors".to_string(),
            },
        ];
        for s in cases {
            assert_serde_json_stable(&s);
        }
    }

    #[test]
    fn compilation_result_serde_roundtrip() {
        let result = CompilationResult {
            success: true,
            output_files: vec![],
            messages: vec![CompilerMessage {
                message_type: MessageType::Warning,
                source_file: Some(PathBuf::from("a.c")),
                line_number: Some(10),
                column_number: None,
                message: "unused".to_string(),
            }],
            compilation_time: Duration::from_millis(100),
            memory_usage: MemoryUsage::default(),
        };
        assert_serde_json_stable(&result);
    }

    #[test]
    fn link_result_serde_roundtrip() {
        let result = LinkResult {
            success: false,
            executable: Some(PathBuf::from("a.out")),
            memory_map: Some(MemoryMap {
                regions: vec![],
                symbols: vec![],
                sections: vec![],
            }),
            messages: vec![LinkerMessage {
                message_type: MessageType::Error,
                section: Some(".text".to_string()),
                symbol: Some("_start".to_string()),
                message: "undefined".to_string(),
            }],
            link_time: Duration::from_millis(50),
        };
        assert_serde_json_stable(&result);
    }

    #[test]
    fn memory_map_region_and_symbol_serde_roundtrip() {
        let region = MemoryMapRegion {
            name: "flash".to_string(),
            start_address: 0,
            end_address: 0xffff,
            size: 0x10000,
            region_type: MemoryRegionType::Flash,
            permissions: MemoryPermissions {
                read: true,
                write: false,
                execute: true,
            },
        };
        assert_serde_json_stable(&region);

        let sym = Symbol {
            name: "main".to_string(),
            address: 0x200,
            size: 4,
            symbol_type: SymbolType::Function,
            section: Some(".text".to_string()),
        };
        assert_serde_json_stable(&sym);
    }

    #[test]
    fn section_struct_serde_roundtrip() {
        let section = Section {
            name: ".data".to_string(),
            start_address: 0x1000,
            size: 0x100,
            section_type: SectionType::Data,
            alignment: 4,
        };
        assert_serde_json_stable(&section);
    }

    #[test]
    fn target_info_serde_roundtrip() {
        let info = TargetInfo {
            name: "demo".to_string(),
            architecture: LegacyArchitecture::ZilogZ80,
            flash_size: 32 * 1024,
            ram_size: 8 * 1024,
            eeprom_size: Some(1024),
            cpu_speed: 16_000_000,
            features: vec!["uart".to_string()],
        };
        assert_serde_json_stable(&info);
    }

    #[test]
    fn cpu_registers_serde_roundtrip() {
        let mut gp = StdHashMap::new();
        gp.insert("a".to_string(), 0x42);
        let regs = CpuRegisters {
            general_purpose: gp,
            program_counter: 0x100,
            stack_pointer: 0x200,
            status_register: 0,
            special: StdHashMap::new(),
        };
        assert_serde_json_stable(&regs);
    }

    #[test]
    fn emulation_status_variants_roundtrip() {
        let cases = vec![
            EmulationStatus::Running,
            EmulationStatus::Stopped,
            EmulationStatus::Breakpoint { address: 0x400 },
            EmulationStatus::Error {
                message: "halt".to_string(),
            },
        ];
        for s in cases {
            assert_serde_json_stable(&s);
        }
    }

    #[test]
    fn peripheral_status_serde_roundtrip() {
        let mut regs = StdHashMap::new();
        regs.insert("sr".to_string(), 1);
        let ps = PeripheralStatus {
            name: "uart0".to_string(),
            peripheral_type: PeripheralType::UART,
            status: "idle".to_string(),
            registers: regs,
            interrupt_status: false,
        };
        assert_serde_json_stable(&ps);
    }

    #[test]
    fn debug_formatting_is_non_empty() {
        let job = EmbeddedJob {
            job_id: Uuid::new_v4(),
            target_architecture: LegacyArchitecture::Intel8086,
            job_type: EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::Assembly,
                optimization: OptimizationLevel::None,
                debug_info: false,
            },
            source_files: vec![],
            memory_layout: sample_memory_layout(),
            programming_interface: sample_programming_interface(),
            status: JobStatus::Running,
            output_files: vec![],
            compilation_log: String::new(),
            programming_log: String::new(),
            start_time: None,
            end_time: None,
        };
        let dbg = format!("{job:?}");
        assert!(dbg.contains("EmbeddedJob"));
        assert!(dbg.len() > 20);
    }
}
