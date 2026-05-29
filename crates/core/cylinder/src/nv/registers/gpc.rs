// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! GPC/TPC register address formulas and broadcast controls.

/// Stride between GPC instances in BAR0.
pub const STRIDE: u32 = 0x8000;
/// GPC unit region base.
pub const UNIT_BASE: u32 = 0x0050_0000;

/// Per-GPC base address.
#[must_use]
pub const fn gpc_base(gpc: u32) -> u32 {
    UNIT_BASE + gpc * STRIDE
}

/// TPC enable register for a given GPC.
#[must_use]
pub const fn tpc_enable(gpc: u32) -> u32 {
    0x0050_2000 + gpc * STRIDE + 0x2608
}

/// GPC TPC0 control block base for a given GPC.
#[must_use]
pub const fn gpc_tpc0(gpc: u32) -> u32 {
    0x0050_4000 + gpc * STRIDE
}

/// GPC broadcast control register.
pub const BCAST_CONTROL: u32 = 0x0041_9000;
/// GPC broadcast enables (per-GPC TPC/GPC clock enables).
pub const BCAST_ENABLES: u32 = 0x0041_A004;
/// GPC broadcast PGOB (power gating override block).
pub const BCAST_PGOB: u32 = 0x0041_A028;

/// GPC broadcast MMU control (enable MMU engines).
pub const BCAST_MMU_CTRL: u32 = 0x0041_8880;
/// GPC broadcast MMU performance unit mask.
pub const BCAST_MMU_PM_UNIT_MASK: u32 = 0x0041_8890;
/// GPC broadcast MMU performance request mask.
pub const BCAST_MMU_PM_REQ_MASK: u32 = 0x0041_8894;
/// GPC broadcast MMU debug control.
pub const BCAST_MMU_DEBUG_CTRL: u32 = 0x0041_88A4;
/// GPC broadcast MMU debug write (writeback control).
pub const BCAST_MMU_DEBUG_WR: u32 = 0x0041_88B4;
/// GPC broadcast MMU debug read.
pub const BCAST_MMU_DEBUG_RD: u32 = 0x0041_88B8;
/// GPC broadcast MMU debug register B0 (additional debug control).
pub const BCAST_MMU_DEBUG_B0: u32 = 0x0041_88B0;
/// GPC broadcast TPC control.
pub const BCAST_TPC_CTRL: u32 = 0x0041_9C04;

/// TPC0 SM control base for a given GPC.
#[must_use]
pub const fn gpc_tpc0_sm(gpc: u32) -> u32 {
    0x0050_4200 + gpc * STRIDE
}
