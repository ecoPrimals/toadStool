// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PGRAPH — graphics/compute engine status and control.

/// PGRAPH engine status (PRI health, idle/busy, fault indicators).
pub const STATUS: u32 = 0x0040_0700;
/// PGRAPH engine control.
pub const CONTROL: u32 = 0x0040_0110;

/// PFIFO engine enable (1 = enabled).
pub const PFIFO_ENABLE: u32 = 0x0000_2200;
/// PBDMA0 interrupt status.
pub const PBDMA0_INTR: u32 = 0x0004_0100;
/// THERM clock gating status.
pub const THERM_GATE: u32 = 0x0002_0200;
