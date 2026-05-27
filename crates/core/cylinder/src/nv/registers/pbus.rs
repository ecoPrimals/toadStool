// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PBUS — host bus interface, BAR windows, and bind status.

/// BAR0 window selector for PRAMIN and other aperture mappings.
pub const BAR0_WINDOW: u32 = 0x0000_1700;
/// BAR1 mapping block configuration.
pub const BAR1_BLOCK: u32 = 0x0000_1704;
/// Instance block bind status.
pub const BIND_STATUS: u32 = 0x0000_1710;
/// BAR2 mapping block configuration.
pub const BAR2_BLOCK: u32 = 0x0000_1714;
/// PROM (parallel ROM) access enable.
pub const PROM_ENABLE: u32 = 0x0000_1854;
