// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "hardware register constants — comprehensive coverage for evolving absorption"
)]

//! PTIMER — GPU timestamp counter.

/// PTIMER low 32 bits of the nanosecond counter.
pub const TIME_0: u32 = 0x0000_9400;
/// PTIMER high 32 bits of the nanosecond counter.
pub const TIME_1: u32 = 0x0000_9410;
