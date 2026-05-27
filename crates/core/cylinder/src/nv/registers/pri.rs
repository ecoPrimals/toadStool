// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PRI (Primary Register Interface) ring — bus routing and station enumeration.

/// PRI ring interrupt status.
pub const INTR_STATUS: u32 = 0x0012_0048;
/// PRI ring status after enumerate/start commands.
pub const STATUS_ENUM: u32 = 0x0012_0050;
/// PRI ring command (ack, enumerate, start).
pub const COMMAND: u32 = 0x0012_004C;
/// PRI station acknowledge register.
pub const STATION_ACK: u32 = 0x0012_0004;
/// PRI ringmaster command register.
pub const RINGMASTER_COMMAND: u32 = 0x0012_2000;
/// PRI ringmaster interrupt status.
pub const RINGMASTER_INTR_STATUS: u32 = 0x0012_200C;
