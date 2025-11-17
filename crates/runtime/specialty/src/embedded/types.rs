//! Type definitions for embedded systems support
//!
//! This module contains all type definitions for embedded system adapters,
//! including job types, languages, debugging interfaces, and file representations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    JobStatus, LegacyArchitecture, MemoryLayout, MemoryRegionType, PeripheralConfig,
    PeripheralType, ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult,
    MemoryPermissions,
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
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
}

/// Types of embedded jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddedJobType {
    /// Compile source code
    Compilation {
        language: EmbeddedLanguage,
        optimization: OptimizationLevel,
        debug_info: bool,
    },
    /// Program ROM/Flash
    Programming {
        target_memory: MemoryRegionType,
        verify: bool,
        erase_first: bool,
    },
    /// Debug session
    Debugging {
        debug_interface: DebugInterface,
        breakpoints: Vec<Breakpoint>,
    },
    /// Emulation
    Emulation {
        emulator_type: EmulatorType,
        rom_image: Vec<u8>,
    },
    /// Memory dump
    MemoryDump { start_address: u32, length: u32 },
    /// Peripheral test
    PeripheralTest {
        peripheral: PeripheralType,
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
pub trait EmbeddedToolchain: Send + Sync {
    /// Get toolchain name
    fn name(&self) -> &str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize toolchain
    async fn initialize(&mut self, config: &crate::EmbeddedConfig) -> ToadStoolResult<()>;

    /// Compile source code
    async fn compile(
        &self,
        sources: &[SourceFile],
        output_path: &PathBuf,
    ) -> ToadStoolResult<CompilationResult>;

    /// Link object files
    async fn link(
        &self,
        objects: &[PathBuf],
        output_path: &PathBuf,
        memory_layout: &MemoryLayout,
    ) -> ToadStoolResult<LinkResult>;

    /// Generate ROM image
    async fn generate_rom_image(
        &self,
        executable: &PathBuf,
        format: OutputFileType,
    ) -> ToadStoolResult<Vec<u8>>;

    /// Disassemble binary
    async fn disassemble(&self, binary: &[u8], start_address: u32) -> ToadStoolResult<String>;

    /// Create memory map
    async fn create_memory_map(&self, executable: &PathBuf) -> ToadStoolResult<MemoryMap>;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Custom section
    Custom { name: String },
}

/// Programmer interface trait
#[async_trait::async_trait]
pub trait ProgrammerInterface: Send + Sync {
    /// Get programmer name
    fn name(&self) -> &str;

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
    async fn verify_memory(
        &mut self,
        address: u32,
        expected_data: &[u8],
    ) -> ToadStoolResult<bool>;

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
pub trait EmbeddedEmulator: Send + Sync {
    /// Get emulator name
    fn name(&self) -> &str;

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
    Breakpoint { address: u32 },
    /// Emulation error
    Error { message: String },
}

/// Peripheral interface trait
#[async_trait::async_trait]
pub trait PeripheralInterface: Send + Sync {
    /// Get peripheral name
    fn name(&self) -> &str;

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
