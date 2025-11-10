//! Configuration type definitions for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use chrono::{DateTime, Utc};
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};

// Import canonical JobPriority for conversions
use toadstool::JobPriority as CanonicalJobPriority;

// Import types from parent modules
use crate::{LegacySystemType, LegacyArchitecture};

/// Target formats for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetFormat {
    /// Executable program
    Executable,
    /// Object file
    Object,
    /// Library
    Library,
    /// ROM image
    ROMImage,
    /// Disk image
    DiskImage,
}

/// Paper tape formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperTapeFormat {
    /// ASCII
    ASCII,
    /// Binary
    Binary,
    /// BASIC
    BASIC,
    /// Assembly
    Assembly,
    /// Custom format
    Custom { name: String },
}

/// ROM formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ROMFormat {
    /// Intel HEX
    IntelHex,
    /// Motorola S-record
    MotorolaS,
    /// Binary
    Binary,
    /// Custom format
    Custom { name: String },
}

/// Terminal types for interactive sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalType {
    /// VT100
    VT100,
    /// VT220
    VT220,
    /// VT320
    VT320,
    /// IBM 3270
    IBM3270,
    /// Tektronix 4010
    Tektronix4010,
    /// ANSI terminal
    ANSI,
    /// Dumb terminal
    Dumb,
    /// Custom terminal
    Custom { name: String, capabilities: Vec<String> },
}

/// Session configuration for interactive sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Terminal width
    pub width: u16,
    /// Terminal height
    pub height: u16,
    /// Line ending style
    pub line_ending: LineEnding,
    /// Character encoding
    pub encoding: CharacterEncoding,
    /// Flow control
    pub flow_control: FlowControl,
}

/// Line ending styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix (LF)
    Unix,
    /// Windows (CRLF)
    Windows,
    /// Classic Mac (CR)
    ClassicMac,
    /// Custom
    Custom { sequence: String },
}

/// Character encodings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterEncoding {
    /// ASCII
    ASCII,
    /// EBCDIC
    EBCDIC,
    /// UTF-8
    UTF8,
    /// ISO-8859-1
    ISO8859_1,
    /// CP437 (PC)
    CP437,
    /// PETSCII (Commodore)
    PETSCII,
    /// ATASCII (Atari)
    ATASCII,
    /// Custom encoding
    Custom { name: String },
}

/// Flow control types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowControl {
    /// No flow control
    None,
    /// Hardware flow control (RTS/CTS)
    Hardware,
    /// Software flow control (XON/XOFF)
    Software,
}

/// File transfer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    /// Upload to legacy system
    Upload,
    /// Download from legacy system
    Download,
    /// Bidirectional transfer
    Bidirectional,
}

/// System monitoring types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringType {
    /// CPU usage
    CPU,
    /// Memory usage
    Memory,
    /// Storage usage
    Storage,
    /// Network traffic
    Network,
    /// System performance
    Performance,
    /// Process monitoring
    Process,
    /// Custom monitoring
    Custom { name: String, parameters: HashMap<String, String> },
}

/// System administration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdministrationType {
    /// User management
    UserManagement,
    /// File system management
    FileSystemManagement,
    /// Process management
    ProcessManagement,
    /// System configuration
    SystemConfiguration,
    /// Backup and restore
    BackupRestore,
    /// Custom administration
    Custom { name: String },
}

/// Legacy job priorities (for backward compatibility with legacy systems)
/// 
/// Note: For new code, use `toadstool::JobPriority` (canonical definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
    /// Real-time priority (maps to Emergency in canonical)
    RealTime,
}

impl From<JobPriority> for CanonicalJobPriority {
    fn from(legacy: JobPriority) -> Self {
        match legacy {
            JobPriority::Low => CanonicalJobPriority::Low,
            JobPriority::Normal => CanonicalJobPriority::Normal,
            JobPriority::High => CanonicalJobPriority::High,
            JobPriority::Critical => CanonicalJobPriority::Critical,
            JobPriority::RealTime => CanonicalJobPriority::Emergency,
        }
    }
}

impl From<CanonicalJobPriority> for JobPriority {
    fn from(canonical: CanonicalJobPriority) -> Self {
        match canonical {
            CanonicalJobPriority::Emergency => JobPriority::RealTime,
            CanonicalJobPriority::Critical => JobPriority::Critical,
            CanonicalJobPriority::High => JobPriority::High,
            CanonicalJobPriority::Normal => JobPriority::Normal,
            CanonicalJobPriority::Low => JobPriority::Low,
            CanonicalJobPriority::Background => JobPriority::Low, // Map Background to Low
        }
    }
}

/// Optimization levels for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization
    None,
    /// Basic optimization
    Basic,
    /// Standard optimization
    Standard,
    /// Maximum optimization
    Maximum,
}

/// Communication settings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationSettings {
    /// Connection type
    pub connection_type: ConnectionType,
    
    /// Timeout configuration (connection, request, read, write)
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    /// Retry configuration (max attempts, backoff, jitter)
    #[serde(flatten)]
    pub retries: RetryConfig,
    
    /// Authentication settings
    pub authentication: Option<AuthenticationSettings>,
}

impl Default for CommunicationSettings {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::LocalEmulation,
            timeouts: TimeoutConfig::default(),
            retries: RetryConfig::default(),
            authentication: None,
        }
    }
}

/// Connection types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Direct serial connection
    DirectSerial { port: String, baud_rate: u32 },
    /// Telnet connection
    Telnet { host: String, port: u16 },
    /// SSH connection
    SSH { host: String, port: u16 },
    /// IBM 3270 terminal emulation
    IBM3270 { host: String, port: u16 },
    /// Local emulation
    LocalEmulation,
    /// Custom connection
    Custom { name: String, parameters: HashMap<String, String> },
}

/// Authentication settings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationSettings {
    /// Authentication type
    pub auth_type: AuthenticationType,
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// Key file
    pub key_file: Option<PathBuf>,
    /// Certificate
    pub certificate: Option<PathBuf>,
}

/// Authentication types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationType {
    /// No authentication
    None,
    /// Username/password
    UsernamePassword,
    /// Public key
    PublicKey,
    /// Certificate
    Certificate,
    /// Custom authentication
    Custom { name: String },
}

/// Toolchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    /// Toolchain name
    pub name: String,
    /// Toolchain path
    pub path: PathBuf,
    /// Compiler executable
    pub compiler: String,
    /// Linker executable
    pub linker: String,
    /// Assembler executable
    pub assembler: String,
    /// Archiver executable
    pub archiver: String,
    /// Debugger executable
    pub debugger: Option<String>,
    /// Cross-compilation target
    pub target: String,
    /// Environment variables
    pub environment: HashMap<String, String>,
}

/// Mainframe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainframeConfig {
    /// Mainframe type
    pub system_type: LegacySystemType,
    /// Connection settings
    pub connection: ConnectionSettings,
    /// Dataset configuration
    pub datasets: HashMap<String, DatasetConfig>,
    /// JCL settings
    pub jcl_settings: JCLSettings,
    /// COBOL settings
    pub cobol_settings: COBOLSettings,
}

/// Connection settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Connection type
    pub connection_type: MainframeConnectionType,
    /// Authentication
    pub authentication: AuthenticationSettings,
}

/// Mainframe connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MainframeConnectionType {
    /// IBM 3270 terminal
    IBM3270,
    /// IBM 5250 terminal
    IBM5250,
    /// FTP
    FTP,
    /// SFTP
    SFTP,
    /// HTTP/HTTPS
    HTTP,
    /// Custom connection
    Custom { name: String },
}

/// Dataset configuration for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Dataset name
    pub name: String,
    /// Dataset type
    pub dataset_type: DatasetType,
    /// Record format
    pub record_format: RecordFormat,
    /// Record length
    pub record_length: u32,
    /// Block size
    pub block_size: u32,
    /// Space allocation
    pub space_allocation: SpaceAllocation,
}

/// Dataset types for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasetType {
    /// Sequential dataset
    Sequential,
    /// Partitioned dataset
    Partitioned,
    /// Indexed dataset
    Indexed,
    /// Direct access dataset
    DirectAccess,
    /// VSAM dataset
    VSAM,
}

/// Record formats for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordFormat {
    /// Fixed length
    Fixed,
    /// Variable length
    Variable,
    /// Fixed blocked
    FixedBlocked,
    /// Variable blocked
    VariableBlocked,
    /// Undefined
    Undefined,
}

/// Space allocation for datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAllocation {
    /// Primary space
    pub primary: u64,
    /// Secondary space
    pub secondary: u64,
    /// Space unit
    pub unit: SpaceUnit,
}

/// Space units for datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpaceUnit {
    /// Tracks
    Tracks,
    /// Cylinders
    Cylinders,
    /// Blocks
    Blocks,
    /// Bytes
    Bytes,
}

/// JCL settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JCLSettings {
    /// Job class
    pub job_class: String,
    /// Message class
    pub message_class: String,
    /// Priority
    pub priority: u8,
    /// Time limit
    pub time_limit: Duration,
    /// Region size
    pub region_size: u64,
}

/// COBOL settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct COBOLSettings {
    /// COBOL compiler
    pub compiler: String,
    /// Compilation options
    pub compile_options: Vec<String>,
    /// Link options
    pub link_options: Vec<String>,
    /// Runtime options
    pub runtime_options: Vec<String>,
}

/// Embedded system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    /// Target architecture
    pub architecture: LegacyArchitecture,
    /// Memory layout
    pub memory_layout: MemoryLayout,
    /// Peripheral configuration
    pub peripherals: Vec<PeripheralConfig>,
    /// Programming interface
    pub programming_interface: ProgrammingInterface,
}

/// Memory layout for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLayout {
    /// ROM/Flash regions
    pub rom_regions: Vec<MemoryRegion>,
    /// RAM regions
    pub ram_regions: Vec<MemoryRegion>,
    /// I/O regions
    pub io_regions: Vec<MemoryRegion>,
}

/// Memory region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Region name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// End address
    pub end_address: u32,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Access permissions
    pub permissions: MemoryPermissions,
}

/// Memory region types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRegionType {
    /// ROM
    ROM,
    /// Flash
    Flash,
    /// RAM
    RAM,
    /// I/O
    IO,
    /// Reserved
    Reserved,
}

/// Memory permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

/// Peripheral configuration for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralConfig {
    /// Peripheral name
    pub name: String,
    /// Peripheral type
    pub peripheral_type: PeripheralType,
    /// Base address
    pub base_address: u32,
    /// Interrupt vector
    pub interrupt_vector: Option<u8>,
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
}

/// Peripheral types for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeripheralType {
    /// UART
    UART,
    /// SPI
    SPI,
    /// I2C
    I2C,
    /// GPIO
    GPIO,
    /// Timer
    Timer,
    /// ADC
    ADC,
    /// DAC
    DAC,
    /// PWM
    PWM,
    /// CAN
    CAN,
    /// USB
    USB,
    /// Ethernet
    Ethernet,
    /// Custom peripheral
    Custom { name: String },
}

/// Programming interface for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammingInterface {
    /// Interface type
    pub interface_type: ProgrammingInterfaceType,
    /// Connection parameters
    pub connection_params: HashMap<String, String>,
}

/// Programming interface types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgrammingInterfaceType {
    /// In-System Programming (ISP)
    ISP,
    /// In-Circuit Serial Programming (ICSP)
    ICSP,
    /// JTAG
    JTAG,
    /// SWD (Serial Wire Debug)
    SWD,
    /// Parallel programmer
    Parallel,
    /// Serial programmer
    Serial,
    /// Custom interface
    Custom { name: String },
}

/// Industrial system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialConfig {
    /// System type
    pub system_type: IndustrialSystemType,
    /// Communication protocols
    pub protocols: Vec<IndustrialProtocol>,
    /// Device configuration
    pub devices: Vec<IndustrialDevice>,
    /// Safety configuration
    pub safety_config: SafetyConfig,
}

/// Industrial system types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialSystemType {
    /// PLC (Programmable Logic Controller)
    PLC,
    /// SCADA (Supervisory Control And Data Acquisition)
    SCADA,
    /// DCS (Distributed Control System)
    DCS,
    /// HMI (Human Machine Interface)
    HMI,
    /// MES (Manufacturing Execution System)
    MES,
    /// Custom system
    Custom { name: String },
}

/// Industrial communication protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialProtocol {
    /// Modbus RTU
    ModbusRTU,
    /// Modbus TCP
    ModbusTCP,
    /// Profibus
    Profibus,
    /// Profinet
    Profinet,
    /// DeviceNet
    DeviceNet,
    /// ControlNet
    ControlNet,
    /// EtherNet/IP
    EtherNetIP,
    /// CAN bus
    CANBus,
    /// Foundation Fieldbus
    FoundationFieldbus,
    /// HART
    HART,
    /// AS-Interface
    ASInterface,
    /// Custom protocol
    Custom { name: String },
}

/// Industrial device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialDevice {
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: IndustrialDeviceType,
    /// Device address
    pub address: String,
    /// Communication protocol
    pub protocol: IndustrialProtocol,
    /// Device parameters
    pub parameters: HashMap<String, String>,
}

/// Industrial device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndustrialDeviceType {
    /// Input/Output module
    IOModule,
    /// Sensor
    Sensor,
    /// Actuator
    Actuator,
    /// Motor drive
    MotorDrive,
    /// Valve
    Valve,
    /// Transmitter
    Transmitter,
    /// Controller
    Controller,
    /// Custom device
    Custom { name: String },
}

/// Safety configuration for industrial systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Safety integrity level
    pub sil_level: SILLevel,
    /// Safety functions
    pub safety_functions: Vec<SafetyFunction>,
    /// Emergency stop configuration
    pub emergency_stop: EmergencyStopConfig,
}

/// Safety Integrity Level (SIL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SILLevel {
    /// SIL 1
    SIL1,
    /// SIL 2
    SIL2,
    /// SIL 3
    SIL3,
    /// SIL 4
    SIL4,
}

/// Safety function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyFunction {
    /// Function name
    pub name: String,
    /// Function type
    pub function_type: SafetyFunctionType,
    /// Response time
    pub response_time: Duration,
    /// Test interval
    pub test_interval: Duration,
}

/// Safety function types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyFunctionType {
    /// Emergency stop
    EmergencyStop,
    /// Safety door
    SafetyDoor,
    /// Light curtain
    LightCurtain,
    /// Pressure sensitive mat
    PressureMat,
    /// Two-hand control
    TwoHandControl,
    /// Custom function
    Custom { name: String },
}

/// Emergency stop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStopConfig {
    /// Emergency stop devices
    pub devices: Vec<String>,
    /// Response time
    pub response_time: Duration,
    /// Reset procedure
    pub reset_procedure: ResetProcedure,
}

/// Reset procedure types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResetProcedure {
    /// Automatic reset
    Automatic,
    /// Manual reset
    Manual,
    /// Key reset
    KeyReset,
    /// Custom procedure
    Custom { name: String },
}

/// Real-time system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    /// Real-time OS
    pub rtos: RealtimeOS,
    /// Scheduling policy
    pub scheduling_policy: SchedulingPolicy,
    /// Task configuration
    pub tasks: Vec<TaskConfig>,
    /// Interrupt configuration
    pub interrupts: Vec<InterruptConfig>,
}

/// Real-time operating systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RealtimeOS {
    /// VxWorks
    VxWorks,
    /// QNX
    QNX,
    /// RT-11
    RT11,
    /// RTOS-32
    RTOS32,
    /// FreeRTOS
    FreeRTOS,
    /// embOS
    EmbOS,
    /// µC/OS
    MicroCOS,
    /// Custom RTOS
    Custom { name: String },
}

/// Scheduling policies for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Preemptive scheduling
    Preemptive,
    /// Cooperative scheduling
    Cooperative,
    /// Round-robin scheduling
    RoundRobin,
    /// Priority-based scheduling
    Priority,
    /// Rate-monotonic scheduling
    RateMonotonic,
    /// Earliest deadline first
    EarliestDeadlineFirst,
    /// Custom scheduling
    Custom { name: String },
}

/// Task configuration for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    /// Task priority
    pub priority: u8,
    /// Stack size
    pub stack_size: u32,
    /// Task period
    pub period: Duration,
    /// Task deadline
    pub deadline: Duration,
    /// Task function
    pub function: String,
}

/// Interrupt configuration for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptConfig {
    /// Interrupt number
    pub interrupt_number: u8,
    /// Interrupt priority
    pub priority: u8,
    /// Interrupt handler
    pub handler: String,
    /// Interrupt type
    pub interrupt_type: InterruptType,
}

/// Interrupt types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptType {
    /// Hardware interrupt
    Hardware,
    /// Software interrupt
    Software,
    /// Timer interrupt
    Timer,
    /// External interrupt
    External,
    /// Custom interrupt
    Custom { name: String },
}

/// Emulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulationConfig {
    /// Emulator type
    pub emulator_type: EmulatorType,
    /// Emulator path
    pub emulator_path: PathBuf,
    /// Emulator parameters
    pub parameters: HashMap<String, String>,
    /// ROM/BIOS files
    pub rom_files: Vec<ROMFile>,
    /// Disk images
    pub disk_images: Vec<DiskImage>,
}

/// Emulator types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    /// SIMH emulator
    SIMH,
    /// MAME emulator
    MAME,
    /// MESS emulator
    MESS,
    /// Virtual machine
    VirtualMachine,
    /// Custom emulator
    Custom { name: String },
}

/// ROM file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROMFile {
    /// File name
    pub name: String,
    /// File path
    pub path: PathBuf,
    /// Load address
    pub load_address: u32,
    /// File size
    pub size: u64,
    /// Checksum
    pub checksum: String,
}

/// Disk image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskImage {
    /// Image name
    pub name: String,
    /// Image path
    pub path: PathBuf,
    /// Image type
    pub image_type: DiskImageType,
    /// Image size
    pub size: u64,
    /// Read-only flag
    pub read_only: bool,
}

/// Disk image types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiskImageType {
    /// Raw disk image
    Raw,
    /// IMG file
    IMG,
    /// ISO file
    ISO,
    /// VDI file
    VDI,
    /// VMDK file
    VMDK,
    /// VHD file
    VHD,
    /// Custom format
    Custom(String),
}