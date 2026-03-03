// SPDX-License-Identifier: AGPL-3.0-or-later
//! Legacy job type definitions

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;
use std::time::Duration;
use std::path::PathBuf;
use std::collections::HashMap;

// Import from other modules
use super::systems::{LegacySystemType, LegacyArchitecture};
use super::requirements::{CompilationRequirements, LegacyRuntimeRequirements};
use super::configs::{CommunicationSettings, SessionConfig};
use toadstool::JobPriority;

/// Legacy job specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyJob {
    /// Job identifier
    pub job_id: Uuid,
    /// Target legacy system
    pub target_system: LegacySystemType,
    /// Target architecture
    pub target_architecture: LegacyArchitecture,
    /// Job type
    pub job_type: LegacyJobType,
    /// Source code or program
    pub source: LegacyJobSource,
    /// Compilation requirements
    pub compilation_requirements: CompilationRequirements,
    /// Runtime requirements
    pub runtime_requirements: LegacyRuntimeRequirements,
    /// Communication settings
    pub communication_settings: CommunicationSettings,
    /// Job priority
    pub priority: JobPriority,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Timeout
    pub timeout: Duration,
}

/// Target format for compiled code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetFormat {
    Executable,
    ObjectFile,
    Library,
    LoadModule,
}

/// Terminal type for interactive sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalType {
    VT100,
    VT220,
    IBM3270,
    ANSI,
    Dumb,
}

/// Transfer type for file transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    Upload,
    Download,
    Bidirectional,
}

/// Monitoring type for system monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringType {
    CPUUsage,
    MemoryUsage,
    IOActivity,
    NetworkActivity,
    ProcessList,
}

/// Administration type for system administration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdministrationType {
    UserManagement,
    SystemConfiguration,
    SecuritySettings,
    PerformanceTuning,
}

/// Paper tape format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperTapeFormat {
    ASCII,
    Binary,
    EBCDIC,
}

/// ROM format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ROMFormat {
    Raw,
    IntelHex,
    MotorolaS,
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

/// Types of legacy jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyJobType {
    /// Compile legacy source code
    Compilation {
        language: LegacyLanguage,
        target_format: TargetFormat,
    },
    /// Execute pre-compiled program
    Execution {
        program_format: ProgramFormat,
        arguments: Vec<String>,
    },
    /// Interactive session
    InteractiveSession {
        terminal_type: TerminalType,
        session_config: SessionConfig,
    },
    /// File transfer
    FileTransfer {
        transfer_type: TransferType,
        source_path: PathBuf,
        destination_path: PathBuf,
    },
    /// System monitoring
    SystemMonitoring {
        monitoring_type: MonitoringType,
        duration: Duration,
    },
    /// System administration
    SystemAdministration {
        admin_type: AdministrationType,
        commands: Vec<String>,
    },
}

/// Legacy programming languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyLanguage {
    COBOL,
    FORTRAN_77,
    FORTRAN_IV,
    PASCAL,
    PL_I,
    RPG,
    BASIC,
    Assembly_6502,
    Assembly_Z80,
    Assembly_8080,
    Assembly_8086,
    Assembly_68000,
    Assembly_PDP11,
    Assembly_System360,
    C_K_R,
    JCL,
    REXX,
    CLIST,
    DCL,
    Shell_Bourne,
    Shell_Csh,
    Ladder_Logic,
    Structured_Text,
    Function_Block,
    Instruction_List,
}

/// Source code or program for legacy job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyJobSource {
    /// Source code text
    SourceCode {
        language: LegacyLanguage,
        code: String,
    },
    /// Source file path
    SourceFile {
        language: LegacyLanguage,
        file_path: PathBuf,
    },
    /// Binary program
    BinaryProgram {
        format: ProgramFormat,
        data: Vec<u8>,
    },
    /// JCL (Job Control Language) for mainframes
    JCL {
        jcl_text: String,
        datasets: HashMap<String, Vec<u8>>,
    },
    /// Paper tape or card deck
    PaperTape {
        format: PaperTapeFormat,
        data: Vec<u8>,
    },
    /// ROM/EPROM image
    ROMImage {
        format: ROMFormat,
        data: Vec<u8>,
        load_address: u32,
    },
}

/// Program formats for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgramFormat {
    /// Intel HEX format
    IntelHex,
    /// Motorola S-record
    MotorolaS,
    /// Binary executable
    Binary,
    /// CP/M COM file
    CPM_COM,
    /// DOS EXE file
    DOS_EXE,
    /// VAX executable
    VAX_EXE,
    /// IBM load module
    IBM_LoadModule,
    /// Paper tape binary
    PaperTapeBinary,
    /// ROM/EPROM image
    ROMImage,
}

