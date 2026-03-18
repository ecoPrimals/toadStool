// SPDX-License-Identifier: AGPL-3.0-or-later
//! Legacy job type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

// Import from other modules
use super::configs::{CommunicationSettings, SessionConfig};
use super::requirements::{CompilationRequirements, LegacyRuntimeRequirements};
use super::systems::{LegacyArchitecture, LegacySystemType};
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
    #[serde(rename = "FORTRAN_77")]
    Fortran77,
    #[serde(rename = "FORTRAN_IV")]
    FortranIv,
    PASCAL,
    #[serde(rename = "PL_I")]
    PlI,
    RPG,
    BASIC,
    #[serde(rename = "Assembly_6502")]
    Assembly6502,
    #[serde(rename = "Assembly_Z80")]
    AssemblyZ80,
    #[serde(rename = "Assembly_8080")]
    Assembly8080,
    #[serde(rename = "Assembly_8086")]
    Assembly8086,
    #[serde(rename = "Assembly_68000")]
    Assembly68000,
    #[serde(rename = "Assembly_PDP11")]
    AssemblyPdp11,
    #[serde(rename = "Assembly_System360")]
    AssemblySystem360,
    #[serde(rename = "C_K_R")]
    Ckr,
    JCL,
    REXX,
    CLIST,
    DCL,
    #[serde(rename = "Shell_Bourne")]
    ShellBourne,
    #[serde(rename = "Shell_Csh")]
    ShellCsh,
    #[serde(rename = "Ladder_Logic")]
    LadderLogic,
    #[serde(rename = "Structured_Text")]
    StructuredText,
    #[serde(rename = "Function_Block")]
    FunctionBlock,
    #[serde(rename = "Instruction_List")]
    InstructionList,
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
    #[serde(rename = "CPM_COM")]
    CpmCom,
    /// DOS EXE file
    #[serde(rename = "DOS_EXE")]
    DosExe,
    /// VAX executable
    #[serde(rename = "VAX_EXE")]
    VaxExe,
    /// IBM load module
    #[serde(rename = "IBM_LoadModule")]
    IbmLoadModule,
    /// Paper tape binary
    PaperTapeBinary,
    /// ROM/EPROM image
    ROMImage,
}
