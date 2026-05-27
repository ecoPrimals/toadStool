// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PMU falcon registers beyond the common falcon layout.

use super::falcon;

/// PMU falcon base in BAR0.
pub const BASE: u32 = falcon::PMU_BASE;

/// DMATRF base register (Volta falcon v5 layout).
pub const DMATRFBASE: u32 = BASE + 0x054;
/// DMATRF IMEM/DMEM offset (v5 layout).
pub const DMATRFMOFFS: u32 = BASE + 0x11C;
/// DMATRF framebuffer/external offset (v5 layout).
pub const DMATRFFBOFFS: u32 = BASE + 0x120;
/// DMATRF command register (v5 layout).
pub const DMATRFCMD: u32 = BASE + 0x124;

/// DMATRF base register (falcon v4 layout).
pub const FALCON_DMATRFBASE: u32 = BASE + 0x110;
/// DMATRF IMEM/DMEM offset (v4 layout).
pub const FALCON_DMATRFMOFFS: u32 = BASE + 0x114;
/// DMATRF framebuffer/external offset (v4 layout).
pub const FALCON_DMATRFFBOFFS: u32 = BASE + 0x118;
/// DMATRF command register (v4 layout).
pub const FALCON_DMATRFCMD: u32 = BASE + 0x11C;
