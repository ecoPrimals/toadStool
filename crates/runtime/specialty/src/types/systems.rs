//! Legacy system and architecture type definitions

use serde::{Deserialize, Serialize};

/// Types of legacy systems supported
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacySystemType {
    // Mainframe systems
    IBM_System360,
    IBM_System370,
    IBM_zSeries,
    VAX_VMS,
    AS400,
    Unisys_ClearPath,
    
    // Early Unix systems
    PDP11,
    SunOS,
    AIX_Legacy,
    HPUX_Legacy,
    Solaris_Legacy,
    
    // Embedded legacy systems
    Intel8080,
    Intel8086,
    MOS6502,
    Zilog_Z80,
    Motorola68000,
    Intel8051,
    PIC_Microcontroller,
    
    // Real-time systems
    VxWorks,
    QNX_Legacy,
    RT11,
    RTOS32,
    
    // Industrial control
    PLC_Ladder,
    SCADA_System,
    DCS_System,
    HMI_System,
    
    // Special systems
    DOS_16bit,
    CPM_System,
    Apple_II,
    Commodore_64,
    Atari_8bit,
}

/// Legacy computer architectures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacyArchitecture {
    Intel8080,
    Intel8086,
    MOS6502,
    Zilog_Z80,
    Motorola68000,
    PDP11,
    IBM_System360,
    VAX,
    SPARC_v7,
    MIPS_R2000,
    Alpha,
    PowerPC_601,
    ARM_v4,
    Intel_i386,
    Intel_i486,
    Motorola_68HC11,
    Intel_8051,
    PIC_16bit,
    AVR_8bit,
}

/// System operational status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemStatus {
    /// System is online and operational
    Online,
    /// System is offline
    Offline,
    /// System is in maintenance mode
    Maintenance,
    /// System status is unknown
    Unknown,
}

impl Default for SystemStatus {
    fn default() -> Self {
        SystemStatus::Unknown
    }
}
