// SPDX-License-Identifier: AGPL-3.0-only
//! Legacy system and architecture type definitions

use serde::{Deserialize, Serialize};

/// Types of legacy systems supported
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacySystemType {
    // Mainframe systems
    #[serde(rename = "IBM_System360")]
    IbmSystem360,
    #[serde(rename = "IBM_System370")]
    IbmSystem370,
    #[serde(rename = "IBM_zSeries")]
    IbmZSeries,
    #[serde(rename = "VAX_VMS")]
    VaxVms,
    AS400,
    #[serde(rename = "Unisys_ClearPath")]
    UnisysClearPath,

    // Early Unix systems
    PDP11,
    SunOS,
    #[serde(rename = "AIX_Legacy")]
    AixLegacy,
    #[serde(rename = "HPUX_Legacy")]
    HpuxLegacy,
    #[serde(rename = "Solaris_Legacy")]
    SolarisLegacy,

    // Embedded legacy systems
    Intel8080,
    Intel8086,
    MOS6502,
    #[serde(rename = "Zilog_Z80")]
    ZilogZ80,
    Motorola68000,
    Intel8051,
    #[serde(rename = "PIC_Microcontroller")]
    PicMicrocontroller,

    // Real-time systems
    VxWorks,
    #[serde(rename = "QNX_Legacy")]
    QnxLegacy,
    RT11,
    RTOS32,

    // Industrial control
    #[serde(rename = "PLC_Ladder")]
    PlcLadder,
    #[serde(rename = "SCADA_System")]
    ScadaSystem,
    #[serde(rename = "DCS_System")]
    DcsSystem,
    #[serde(rename = "HMI_System")]
    HmiSystem,

    // Special systems
    #[serde(rename = "DOS_16bit")]
    Dos16bit,
    #[serde(rename = "CPM_System")]
    CpmSystem,
    #[serde(rename = "Apple_II")]
    AppleIi,
    #[serde(rename = "Commodore_64")]
    Commodore64,
    #[serde(rename = "Atari_8bit")]
    Atari8bit,
}

/// Legacy computer architectures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacyArchitecture {
    Intel8080,
    Intel8086,
    MOS6502,
    #[serde(rename = "Zilog_Z80")]
    ZilogZ80,
    Motorola68000,
    PDP11,
    #[serde(rename = "IBM_System360")]
    IbmSystem360,
    VAX,
    #[serde(rename = "SPARC_v7")]
    SparcV7,
    #[serde(rename = "MIPS_R2000")]
    MipsR2000,
    Alpha,
    #[serde(rename = "PowerPC_601")]
    PowerPc601,
    #[serde(rename = "ARM_v4")]
    ArmV4,
    #[serde(rename = "Intel_i386")]
    IntelI386,
    #[serde(rename = "Intel_i486")]
    IntelI486,
    #[serde(rename = "Motorola_68HC11")]
    Motorola68Hc11,
    #[serde(rename = "Intel_8051")]
    Intel8051,
    #[serde(rename = "PIC_16bit")]
    Pic16bit,
    #[serde(rename = "AVR_8bit")]
    Avr8bit,
}

/// System operational status
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemStatus {
    /// System is online and operational
    Online,
    /// System is offline
    Offline,
    /// System is in maintenance mode
    Maintenance,
    /// System status is unknown
    #[default]
    Unknown,
}
