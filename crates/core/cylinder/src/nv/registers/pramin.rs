// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PRAMIN — privileged RAMIN window for staging firmware to VRAM.

/// PRAMIN data aperture base in BAR0 (selected via [`super::pbus::BAR0_WINDOW`]).
pub const BASE: u32 = 0x0070_0000;
/// PRAMIN window size (1 MiB).
pub const SIZE: u32 = 0x0010_0000;
