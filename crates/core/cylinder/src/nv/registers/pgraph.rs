// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PGRAPH — graphics/compute engine status and control.

/// PGRAPH engine status (PRI health, idle/busy, fault indicators).
pub const STATUS: u32 = 0x0040_0700;
/// PGRAPH engine control.
pub const CONTROL: u32 = 0x0040_0110;
