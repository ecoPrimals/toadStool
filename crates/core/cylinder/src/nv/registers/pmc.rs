// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PMC (Power Management Controller) — engine enables and boot identity.

/// Chip identity and strap configuration read at boot.
pub const BOOT0: u32 = 0x0000_0000;
/// PMC master interrupt status.
pub const INTR: u32 = 0x0000_0100;
/// PMC interrupt enable mask (engine 0).
pub const INTR_EN_0: u32 = 0x0000_0140;
/// Master engine enable — write `0xFFFF_FFFF` to un-gate all present clock domains.
pub const ENABLE: u32 = 0x0000_0200;
/// Per-device engine enable (also exposed as PBDMA master enable on some GPUs).
pub const DEVICE_ENABLE: u32 = 0x0000_0204;
