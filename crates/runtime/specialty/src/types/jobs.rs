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

/// Target format for compiled code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetFormat {
    /// Standalone executable.
    Executable,
    /// Object file (.o).
    ObjectFile,
    /// Static or dynamic library.
    Library,
    /// Mainframe load module.
    LoadModule,
}

/// Terminal type for interactive sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalType {
    /// DEC VT100.
    VT100,
    /// DEC VT220.
    VT220,
    /// IBM 3270 block mode.
    IBM3270,
    /// ANSI escape sequences.
    ANSI,
    /// Dumb terminal (line-oriented).
    Dumb,
}

/// Transfer direction for file transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    /// Upload to legacy system.
    Upload,
    /// Download from legacy system.
    Download,
    /// Bidirectional sync.
    Bidirectional,
}

/// System metric to monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringType {
    /// CPU utilization.
    CPUUsage,
    /// Memory usage.
    MemoryUsage,
    /// I/O activity.
    IOActivity,
    /// Network traffic.
    NetworkActivity,
    /// Running process list.
    ProcessList,
}

/// System administration task type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdministrationType {
    /// User account management.
    UserManagement,
    /// System configuration changes.
    SystemConfiguration,
    /// Security policy settings.
    SecuritySettings,
    /// Performance tuning.
    PerformanceTuning,
}

/// Paper tape or card deck encoding format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperTapeFormat {
    /// ASCII 7-bit.
    ASCII,
    /// Raw binary.
    Binary,
    /// EBCDIC mainframe encoding.
    EBCDIC,
}

/// ROM/EPROM image format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ROMFormat {
    /// Raw binary.
    Raw,
    /// Intel HEX.
    IntelHex,
    /// Motorola S-record.
    MotorolaS,
}

/// Optimization level for compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization.
    None,
    /// Basic optimizations.
    Basic,
    /// Aggressive optimizations.
    Aggressive,
    /// Optimize for size.
    Size,
    /// Optimize for speed.
    Speed,
}

/// Types of legacy jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyJobType {
    /// Compile legacy source code.
    Compilation {
        /// Source language.
        language: LegacyLanguage,
        /// Desired output format.
        target_format: TargetFormat,
    },
    /// Execute pre-compiled program.
    Execution {
        /// Program binary format.
        program_format: ProgramFormat,
        /// Command-line arguments.
        arguments: Vec<String>,
    },
    /// Interactive terminal session.
    InteractiveSession {
        /// Terminal emulation type.
        terminal_type: TerminalType,
        /// Session configuration.
        session_config: SessionConfig,
    },
    /// File transfer to/from legacy system.
    FileTransfer {
        /// Transfer direction.
        transfer_type: TransferType,
        /// Source path.
        source_path: PathBuf,
        /// Destination path.
        destination_path: PathBuf,
    },
    /// System monitoring job.
    SystemMonitoring {
        /// Metric to monitor.
        monitoring_type: MonitoringType,
        /// Monitoring duration.
        duration: Duration,
    },
    /// System administration task.
    SystemAdministration {
        /// Administration category.
        admin_type: AdministrationType,
        /// Commands to execute.
        commands: Vec<String>,
    },
}

/// Legacy programming languages supported for compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyLanguage {
    /// COBOL.
    COBOL,
    /// FORTRAN 77.
    #[serde(rename = "FORTRAN_77")]
    Fortran77,
    /// FORTRAN IV.
    #[serde(rename = "FORTRAN_IV")]
    FortranIv,
    /// Pascal.
    PASCAL,
    /// PL/I.
    #[serde(rename = "PL_I")]
    PlI,
    /// RPG (Report Program Generator).
    RPG,
    /// BASIC.
    BASIC,
    /// 6502 assembly.
    #[serde(rename = "Assembly_6502")]
    Assembly6502,
    /// Z80 assembly.
    #[serde(rename = "Assembly_Z80")]
    AssemblyZ80,
    /// 8080 assembly.
    #[serde(rename = "Assembly_8080")]
    Assembly8080,
    /// 8086 assembly.
    #[serde(rename = "Assembly_8086")]
    Assembly8086,
    /// 68000 assembly.
    #[serde(rename = "Assembly_68000")]
    Assembly68000,
    /// PDP-11 assembly.
    #[serde(rename = "Assembly_PDP11")]
    AssemblyPdp11,
    /// IBM System/360 assembly.
    #[serde(rename = "Assembly_System360")]
    AssemblySystem360,
    /// K&R C.
    #[serde(rename = "C_K_R")]
    Ckr,
    /// JCL (Job Control Language).
    JCL,
    /// REXX scripting.
    REXX,
    /// CLIST (Command List).
    CLIST,
    /// DCL (Digital Command Language).
    DCL,
    /// Bourne shell.
    #[serde(rename = "Shell_Bourne")]
    ShellBourne,
    /// C shell.
    #[serde(rename = "Shell_Csh")]
    ShellCsh,
    /// PLC ladder logic.
    #[serde(rename = "Ladder_Logic")]
    LadderLogic,
    /// IEC 61131-3 Structured Text.
    #[serde(rename = "Structured_Text")]
    StructuredText,
    /// IEC 61131-3 Function Block.
    #[serde(rename = "Function_Block")]
    FunctionBlock,
    /// IEC 61131-3 Instruction List.
    #[serde(rename = "Instruction_List")]
    InstructionList,
}

/// Source code or program input for a legacy job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyJobSource {
    /// Inline source code.
    SourceCode {
        /// Programming language.
        language: LegacyLanguage,
        /// Source text.
        code: String,
    },
    /// Source file on disk.
    SourceFile {
        /// Programming language.
        language: LegacyLanguage,
        /// Path to source file.
        file_path: PathBuf,
    },
    /// Pre-built binary program.
    BinaryProgram {
        /// Binary format.
        format: ProgramFormat,
        /// Raw program bytes.
        data: Vec<u8>,
    },
    /// JCL for mainframe jobs.
    JCL {
        /// JCL text.
        jcl_text: String,
        /// Dataset name to content mapping.
        datasets: HashMap<String, Vec<u8>>,
    },
    /// Paper tape or card deck input.
    PaperTape {
        /// Encoding format.
        format: PaperTapeFormat,
        /// Raw tape/card data.
        data: Vec<u8>,
    },
    /// ROM/EPROM image.
    ROMImage {
        /// Image format.
        format: ROMFormat,
        /// Image bytes.
        data: Vec<u8>,
        /// Load address for the image.
        load_address: u32,
    },
}

/// Executable/binary formats for legacy systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgramFormat {
    /// Intel HEX format.
    IntelHex,
    /// Motorola S-record.
    MotorolaS,
    /// Raw binary executable.
    Binary,
    /// CP/M COM file.
    #[serde(rename = "CPM_COM")]
    CpmCom,
    /// DOS EXE file.
    #[serde(rename = "DOS_EXE")]
    DosExe,
    /// VAX executable.
    #[serde(rename = "VAX_EXE")]
    VaxExe,
    /// IBM mainframe load module.
    #[serde(rename = "IBM_LoadModule")]
    IbmLoadModule,
    /// Paper tape binary format.
    PaperTapeBinary,
    /// ROM/EPROM image.
    ROMImage,
}
