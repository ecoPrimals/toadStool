// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PFB — framebuffer controller, MMU, and WPR (write-protected region).

/// PFB PRI MMU control (enable, invalidate configuration).
pub const MMU_CTRL: u32 = 0x0010_0C80;
/// WPR2 region base address (low 32 bits).
pub const WPR2_ADDR_LO: u32 = 0x0010_0CE0;
/// WPR2 region base address (high 32 bits).
pub const WPR2_ADDR_HI: u32 = 0x0010_0CE4;
/// WPR2 region control (size, enable).
pub const WPR2_CTRL: u32 = 0x0010_0CE8;
