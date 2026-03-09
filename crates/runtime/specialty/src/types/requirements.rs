// SPDX-License-Identifier: AGPL-3.0-only
//! Resource requirement type definitions for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

// Import from other modules
use super::systems::LegacyArchitecture;

/// Compilation requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    /// Compiler type
    pub compiler: CompilerType,
    /// Compilation flags
    pub flags: Vec<String>,
    /// Include paths
    pub include_paths: Vec<PathBuf>,
    /// Library paths
    pub library_paths: Vec<PathBuf>,
    /// Linked libraries
    pub libraries: Vec<String>,
    /// Target memory model
    pub memory_model: MemoryModel,
    /// Optimization level
    pub optimization: OptimizationLevel,
    /// Debug information
    pub debug_info: bool,
}

/// Types of legacy compilers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilerType {
    /// IBM COBOL compiler
    IBM_COBOL,
    /// Micro Focus COBOL
    MicroFocus_COBOL,
    /// IBM FORTRAN compiler
    IBM_FORTRAN,
    /// VAX FORTRAN
    VAX_FORTRAN,
    /// Turbo Pascal
    Turbo_Pascal,
    /// Microsoft C 6.0
    Microsoft_C_60,
    /// Lattice C
    Lattice_C,
    /// PL/I compiler
    PL_I_Compiler,
    /// 6502 assembler
    ASM_6502,
    /// Z80 assembler
    ASM_Z80,
    /// 8080 assembler
    ASM_8080,
    /// 68000 assembler
    ASM_68000,
    /// PDP-11 assembler
    ASM_PDP11,
    /// System/360 assembler
    ASM_System360,
    /// Cross-compiler
    CrossCompiler {
        host_arch: String,
        target_arch: LegacyArchitecture,
    },
}

/// Runtime requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyRuntimeRequirements {
    /// Memory requirements
    pub memory: MemoryRequirements,
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// Communication requirements
    pub communication: CommunicationRequirements,
    /// Timing requirements
    pub timing: TimingRequirements,
    /// Special hardware requirements
    pub special_hardware: Vec<SpecialHardware>,
}

/// Memory requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes
    pub min_memory: u64,
    /// Maximum memory in bytes
    pub max_memory: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Memory model
    pub memory_model: MemoryModel,
}

/// CPU requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// CPU architecture
    pub architecture: LegacyArchitecture,
    /// Minimum CPU speed in Hz
    pub min_speed: u64,
    /// Required CPU features
    pub required_features: Vec<String>,
    /// Floating point unit required
    pub fpu_required: bool,
}

/// Storage requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes
    pub min_storage: u64,
    /// Storage type
    pub storage_type: StorageType,
    /// File system type
    pub file_system: FileSystemType,
}

/// Communication requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationRequirements {
    /// Communication protocols
    pub protocols: Vec<CommunicationProtocol>,
    /// Port requirements
    pub ports: Vec<PortRequirement>,
    /// Network requirements
    pub network: NetworkRequirements,
}

/// Timing requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingRequirements {
    /// Real-time requirements
    pub real_time: bool,
    /// Maximum response time
    pub max_response_time: Duration,
    /// Minimum cycle time
    pub min_cycle_time: Duration,
    /// Timing accuracy
    pub timing_accuracy: Duration,
}

/// Special hardware requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialHardware {
    /// Paper tape reader
    PaperTapeReader,
    /// Paper tape punch
    PaperTapePunch,
    /// Card reader
    CardReader,
    /// Card punch
    CardPunch,
    /// Line printer
    LinePrinter,
    /// Magnetic tape drive
    MagneticTapeDrive,
    /// Disk drive
    DiskDrive,
    /// Terminal
    Terminal,
    /// Modem
    Modem,
    /// Serial port
    SerialPort,
    /// Parallel port
    ParallelPort,
    /// IEEE-488 interface
    IEEE488,
    /// Custom hardware
    CustomHardware { description: String },
}

/// Memory types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// RAM
    RAM,
    /// ROM
    ROM,
    /// EPROM
    EPROM,
    /// EEPROM
    EEPROM,
    /// Flash
    Flash,
    /// Magnetic core
    MagneticCore,
    /// Bubble memory
    BubbleMemory,
}

/// Memory models for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryModel {
    /// Flat memory model
    Flat,
    /// Segmented memory model
    Segmented,
    /// Paged memory model
    Paged,
    /// Bank-switched memory
    BankSwitched,
    /// Harvard architecture
    Harvard,
    /// Von Neumann architecture
    VonNeumann,
}

/// Storage types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// Magnetic tape
    MagneticTape,
    /// Floppy disk
    FloppyDisk,
    /// Hard disk
    HardDisk,
    /// Paper tape
    PaperTape,
    /// Punch cards
    PunchCards,
    /// Cassette tape
    CassetteTape,
    /// Cartridge
    Cartridge,
    /// Drum storage
    DrumStorage,
    /// Flash memory
    Flash,
    /// ROM (Read-Only Memory)
    ROM,
    /// EEPROM
    EEPROM,
}

/// File system types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemType {
    /// No file system (raw storage)
    None,
    /// CP/M file system
    CPM,
    /// DOS file system
    DOS,
    /// VAX VMS file system
    VMS,
    /// IBM MVS dataset
    MVS_Dataset,
    /// Unix file system
    Unix,
    /// RT-11 file system
    RT11,
    /// Custom file system
    Custom { name: String },
}

/// Communication protocols for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationProtocol {
    /// Serial communication
    Serial { baud_rate: u32, data_bits: u8, stop_bits: u8, parity: Parity },
    /// Parallel communication
    Parallel,
    /// Telnet
    Telnet,
    /// SSH
    SSH,
    /// IBM 3270 terminal
    IBM3270,
    /// VAX terminal
    VAXTerminal,
    /// Modbus
    Modbus,
    /// Profibus
    Profibus,
    /// CAN bus
    CANBus,
    /// Ethernet
    Ethernet,
    /// Token ring
    TokenRing,
    /// DECnet
    DECnet,
    /// SNA (Systems Network Architecture)
    SNA,
    /// X.25
    X25,
    /// Custom protocol
    Custom { name: String, specification: String },
}

/// Port requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRequirement {
    /// Port type
    pub port_type: PortType,
    /// Port number or identifier
    pub port_id: String,
    /// Required or optional
    pub required: bool,
}

/// Port types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortType {
    /// Serial port (RS-232, RS-422, RS-485)
    Serial,
    /// Parallel port
    Parallel,
    /// IEEE-488 (GPIB)
    IEEE488,
    /// Centronics
    Centronics,
    /// Custom port
    Custom { name: String },
}

/// Network requirements for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Network protocols
    pub protocols: Vec<NetworkProtocol>,
    /// Bandwidth requirements
    pub bandwidth: Option<u64>,
    /// Latency requirements
    pub max_latency: Option<Duration>,
}

/// Network protocols for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkProtocol {
    /// TCP/IP
    TCPIP,
    /// IPX/SPX
    IPXSPX,
    /// NetBIOS
    NetBIOS,
    /// DECnet
    DECnet,
    /// SNA
    SNA,
    /// Token ring
    TokenRing,
    /// Ethernet
    Ethernet,
    /// AppleTalk
    AppleTalk,
    /// Banyan VINES
    BanyanVINES,
}

/// Parity settings for serial communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Parity {
    None,
    Even,
    Odd,
    Mark,
    Space,
}

/// Optimization level for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
    Size,
    Speed,
}

