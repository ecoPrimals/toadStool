// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "hardware register constants — comprehensive coverage for evolving absorption"
)]

//! Falcon microcontroller register offsets (relative to any falcon base).
//!
//! Engine bases (`FECS_BASE`, `GPCCS_BASE`, etc.) are absolute BAR0 addresses.
//! Per-falcon offsets apply to any base via `base + offset`.

/// Falcon program counter (read-only execution snapshot).
pub const PC: u32 = 0x030;
/// Host↔falcon mailbox word 0.
pub const MAILBOX0: u32 = 0x040;
/// Host↔falcon mailbox word 1.
pub const MAILBOX1: u32 = 0x044;
/// Interface enable (some falcons — bit 2 = DMA access enable).
pub const ITFEN: u32 = 0x048;
/// CPU control: start, halt, reset.
pub const CPUCTL: u32 = 0x100;
/// Boot vector (PC on start).
pub const BOOTVEC: u32 = 0x104;
/// Hardware config: IMEM/DMEM sizes, security mode.
pub const HWCFG: u32 = 0x108;
/// IMEM control port (PIO upload/read).
pub const IMEMC: u32 = 0x180;
/// IMEM data port.
pub const IMEMD: u32 = 0x184;
/// DMEM control port.
pub const DMEMC: u32 = 0x1C0;
/// DMEM data port.
pub const DMEMD: u32 = 0x1C4;
/// Security mode register (SEC_MODE in bits `[13:12]`).
pub const SCTL: u32 = 0x240;
/// Alternate CPU control register (falcon v5+ / HS falcons).
pub const CPUCTL_ALIAS: u32 = 0x130;
/// Exception info: `[31:16]`=cause, `[15:0]`=PC.
pub const EXCI: u32 = 0x148;
/// Falcon OS / version register.
pub const OS: u32 = 0x180;
/// Method data register (FECS/GPCCS method protocol).
pub const MTHD_DATA: u32 = 0x500;
/// Method command register.
pub const MTHD_CMD: u32 = 0x504;

/// PMU falcon base in BAR0.
pub const PMU_BASE: u32 = 0x0010_A000;
/// FECS (Front-End Command Scheduler) falcon base.
pub const FECS_BASE: u32 = 0x0040_9000;
/// GPCCS (GPC Command Scheduler) falcon base (GPC0 instance).
pub const GPCCS_BASE: u32 = 0x0041_A000;
/// SEC2 falcon base on GV100 (PTOP topology).
pub const SEC2_BASE_GV100: u32 = 0x0008_7000;
/// NVDEC falcon base (GV100).
pub const NVDEC_BASE: u32 = 0x0008_4000;

/// FECS context-switch PC (PGRAPH-wrapped, not the falcon core PC register).
pub const FECS_CTXSW_PC: u32 = 0x0040_9624;
