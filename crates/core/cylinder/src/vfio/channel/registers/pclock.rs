// SPDX-License-Identifier: AGPL-3.0-or-later
//! Root PLL and PCLOCK register offsets.
//!
//! The root PLLs at 0x136xxx are in an always-on power domain — they remain
//! accessible even when the main PCLOCK (0x137xxx) is gated behind PLL lock.
//! These are the "crack in the wall" for sovereign GPU initialization.
//!
//! From PCLOCK deep probe (March 2026): 283 readable registers found in
//! the 0x136xxx range on a cold GV100 card.

/// Root PLL domain — always-on, writable from host even when PCLOCK is gated.
pub const ROOT_PLL_BASE: usize = 0x0013_6000;
pub const ROOT_PLL_END: usize = 0x0013_7000;

/// PCLOCK domain — gated behind PLL lock, partially accessible.
pub const PCLOCK_BASE: usize = 0x0013_7000;
pub const PCLOCK_END: usize = 0x0013_8000;

/// Full CLK domain — includes all clock management registers.
pub const CLK_BASE: usize = 0x0013_0000;
pub const CLK_END: usize = 0x0013_8000;

/// PCLOCK control register. PRI error = `0xBADF5040` (PLL unlocked).
pub const PCLOCK_CTL: usize = 0x0013_7000;
pub const PCLOCK_STATUS: usize = 0x0013_7004;
pub const PCLOCK_COEFF: usize = 0x0013_7008;
pub const PCLOCK_PLL0: usize = 0x0013_7010;
pub const PCLOCK_PLL1: usize = 0x0013_7014;

/// PCLOCK bypass — writable even on cold card.
pub const PCLOCK_BYPASS: usize = 0x0013_7020;
/// NVPLL control — writable even on cold card.
pub const NVPLL_CTL: usize = 0x0013_7050;
pub const NVPLL_COEFF: usize = 0x0013_7054;
/// Memory PLL control — writable even on cold card.
pub const MEMPLL_CTL: usize = 0x0013_7100;
pub const MEMPLL_COEFF: usize = 0x0013_7104;

/// Known root PLL sub-ranges (from the deep probe).
/// These are clusters of readable registers within the 0x136xxx range.
pub const ROOT_PLL_CLUSTERS: &[(usize, &str)] = &[
    (0x136400, "Root PLL cluster 0"),
    (0x136600, "Root PLL cluster 1"),
    (0x136800, "Root PLL cluster 2"),
    (0x136A00, "Root PLL cluster 3"),
    (0x136C00, "Root PLL cluster 4"),
    (0x136E00, "Root PLL cluster 5"),
];
