// SPDX-License-Identifier: AGPL-3.0-only
//! Legacy system and architecture type definitions

use serde::{Deserialize, Serialize};

/// Types of legacy systems supported by the specialty runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacySystemType {
    /// IBM System/360 mainframe.
    #[serde(rename = "IBM_System360")]
    IbmSystem360,
    /// IBM System/370 mainframe.
    #[serde(rename = "IBM_System370")]
    IbmSystem370,
    /// IBM z/Series mainframe.
    #[serde(rename = "IBM_zSeries")]
    IbmZSeries,
    /// DEC VAX running VMS.
    #[serde(rename = "VAX_VMS")]
    VaxVms,
    /// IBM AS/400 midrange system.
    AS400,
    /// Unisys `ClearPath` mainframe.
    #[serde(rename = "Unisys_ClearPath")]
    UnisysClearPath,

    /// DEC PDP-11 minicomputer.
    PDP11,
    /// Sun Microsystems `SunOS`.
    SunOS,
    /// Legacy IBM AIX.
    #[serde(rename = "AIX_Legacy")]
    AixLegacy,
    /// Legacy HP-UX.
    #[serde(rename = "HPUX_Legacy")]
    HpuxLegacy,
    /// Legacy Sun Solaris.
    #[serde(rename = "Solaris_Legacy")]
    SolarisLegacy,

    /// Intel 8080 8-bit microprocessor.
    Intel8080,
    /// Intel 8086 16-bit microprocessor.
    Intel8086,
    /// MOS Technology 6502.
    MOS6502,
    /// Zilog Z80 8-bit microprocessor.
    #[serde(rename = "Zilog_Z80")]
    ZilogZ80,
    /// Motorola 68000 16/32-bit.
    Motorola68000,
    /// Intel 8051 microcontroller.
    Intel8051,
    /// PIC microcontroller family.
    #[serde(rename = "PIC_Microcontroller")]
    PicMicrocontroller,

    /// Wind River `VxWorks` RTOS.
    VxWorks,
    /// QNX real-time OS.
    #[serde(rename = "QNX_Legacy")]
    QnxLegacy,
    /// DEC RT-11 real-time OS.
    RT11,
    /// RTOS-32 real-time OS.
    RTOS32,

    /// PLC with ladder logic.
    #[serde(rename = "PLC_Ladder")]
    PlcLadder,
    /// SCADA supervisory control system.
    #[serde(rename = "SCADA_System")]
    ScadaSystem,
    /// Distributed control system.
    #[serde(rename = "DCS_System")]
    DcsSystem,
    /// Human-machine interface system.
    #[serde(rename = "HMI_System")]
    HmiSystem,

    /// 16-bit MS-DOS.
    #[serde(rename = "DOS_16bit")]
    Dos16bit,
    /// CP/M operating system.
    #[serde(rename = "CPM_System")]
    CpmSystem,
    /// Apple II microcomputer.
    #[serde(rename = "Apple_II")]
    AppleIi,
    /// Commodore 64 home computer.
    #[serde(rename = "Commodore_64")]
    Commodore64,
    /// Atari 8-bit family.
    #[serde(rename = "Atari_8bit")]
    Atari8bit,
}

/// Legacy computer architectures for cross-compilation targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegacyArchitecture {
    /// Intel 8080 8-bit.
    Intel8080,
    /// Intel 8086 16-bit.
    Intel8086,
    /// MOS 6502 8-bit.
    MOS6502,
    /// Zilog Z80 8-bit.
    #[serde(rename = "Zilog_Z80")]
    ZilogZ80,
    /// Motorola 68000 16/32-bit.
    Motorola68000,
    /// DEC PDP-11.
    PDP11,
    /// IBM System/360.
    #[serde(rename = "IBM_System360")]
    IbmSystem360,
    /// DEC VAX.
    VAX,
    /// SPARC v7.
    #[serde(rename = "SPARC_v7")]
    SparcV7,
    /// MIPS R2000.
    #[serde(rename = "MIPS_R2000")]
    MipsR2000,
    /// DEC Alpha.
    Alpha,
    /// `PowerPC` 601.
    #[serde(rename = "PowerPC_601")]
    PowerPc601,
    /// ARM v4.
    #[serde(rename = "ARM_v4")]
    ArmV4,
    /// Intel i386 32-bit.
    #[serde(rename = "Intel_i386")]
    IntelI386,
    /// Intel i486 32-bit.
    #[serde(rename = "Intel_i486")]
    IntelI486,
    /// Motorola 68HC11 microcontroller.
    #[serde(rename = "Motorola_68HC11")]
    Motorola68Hc11,
    /// Intel 8051 microcontroller.
    #[serde(rename = "Intel_8051")]
    Intel8051,
    /// PIC 16-bit microcontroller.
    #[serde(rename = "PIC_16bit")]
    Pic16bit,
    /// AVR 8-bit microcontroller.
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
