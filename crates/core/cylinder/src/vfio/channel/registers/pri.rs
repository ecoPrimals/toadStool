// SPDX-License-Identifier: AGPL-3.0-or-later
//! PRI (Primary Register Interface) bus monitoring and recovery.
//!
//! When a BAR0 read returns `0xBADxxxxx`, the PRI bus has faulted — the
//! target domain didn't respond within the timeout window. This happens
//! when writing to clock-gated or power-gated domains. Without detection,
//! subsequent writes pile up and lock the entire bus.

/// PMC master interrupt status. Bit 26 = PRIV_RING fault pending.
pub const PMC_INTR: usize = 0x0000_0100;
/// Bitmask for the PRIV_RING fault bit in `PMC_INTR`.
pub const PMC_INTR_PRIV_RING_BIT: u32 = 1 << 26;

/// PRIV ring interrupt status — reports which hub/GPC/FBP faulted.
pub const PRIV_RING_INTR_STATUS: usize = 0x0012_0058;
/// PRIV ring command — write 0x2 to ack/clear faults.
pub const PRIV_RING_COMMAND: usize = 0x0012_004C;
/// Acknowledge/clear command value for `PRIV_RING_COMMAND`.
pub const PRIV_RING_CMD_ACK: u32 = 0x0000_0002;

/// PRI master IOCTL — controls timeout duration and enable.
pub const PRI_IOCTL: usize = 0x0012_2120;

/// PRI ringmaster command register (Kepler+).
pub const PRI_RINGMASTER_COMMAND: usize = 0x0012_2000;
/// PRI ringmaster interrupt status (Kepler+).
pub const PRI_RINGMASTER_INTR_STATUS: usize = 0x0012_200C;
/// Enumerate command for PRI ringmaster — re-discovers all ring stations.
pub const PRI_RINGMASTER_CMD_ENUMERATE: u32 = 0x0000_0004;
/// Start command for PRI ringmaster — starts the ring bus.
pub const PRI_RINGMASTER_CMD_START: u32 = 0x0000_0001;

/// Sentinel values returned when a PRI target doesn't respond.
/// The upper 16 bits encode the error type.
pub const fn is_pri_error(val: u32) -> bool {
    let hi = val >> 16;
    hi == 0xBADF || hi == 0xBAD0 || hi == 0xBAD1
}

/// Check if a specific PRI error indicates a timeout (domain unresponsive).
pub const fn is_pri_timeout(val: u32) -> bool {
    (val & 0xFFFF_0000) == 0xBAD0_0000
}

/// Check if a specific PRI error indicates an access violation.
pub const fn is_pri_access_error(val: u32) -> bool {
    (val & 0xFFFF_0000) == 0xBADF_0000
}

/// Decode a PRI error value into a human-readable description.
///
/// NVIDIA PRI errors encode the source and reason:
/// - `0xBADF_xxxx`: PRIV fault — target domain rejected the access
///   - `0xBADF1100`: FBPA partition powered down (BLCG/SLCG gated)
///   - `0xBADF3000`: Domain clock-gated at hub level (PRIV ring)
///   - `0xBADF5040`: Clock domain not configured (PLL not locked)
/// - `0xBAD0_xxxx`: PRI timeout — no response within timeout window
///   - `0xBAD0_0200`: PBUS timeout (bus controller not responding)
///   - `0xBAD0_AC0x`: PRAMIN/VRAM timeout (memory not trained)
///   - `0xBAD0_DA00`: PFIFO timeout (scheduler not initialized)
pub fn decode_pri_error(val: u32) -> &'static str {
    match val & 0xFFFF_FF00 {
        0xBADF_1100 => "FBPA power-gated (BLCG/SLCG)",
        0xBADF_3000 => "Hub-level clock gate (PRIV ring)",
        0xBADF_5000 => "Clock domain unconfigured (PLL unlocked)",
        0xBAD0_0200 => "PBUS timeout",
        0xBAD0_AC00 => "PRAMIN/VRAM timeout (memory untrained)",
        0xBAD0_DA00 => "PFIFO scheduler timeout",
        _ => match val & 0xFFFF_0000 {
            0xBADF_0000 => "PRIV fault (domain rejected access)",
            0xBAD0_0000 => "PRI timeout (no response)",
            0xBAD1_0000 => "PRI target error",
            _ => "Unknown PRI error pattern",
        },
    }
}

/// Classify a BAR0 domain by address range.
pub fn domain_name(offset: usize) -> &'static str {
    match offset {
        0x000000..=0x000FFF => "PMC",
        0x001000..=0x001FFF => "PBUS",
        0x002000..=0x003FFF => "PFIFO",
        0x009000..=0x009FFF => "PTIMER",
        0x00D000..=0x00DFFF => "PGRAPH_GLOBAL",
        0x020000..=0x022FFF => "PTOP/FUSE",
        0x040000..=0x09FFFF => "PBDMA",
        // Narrower PFB sub-regions first
        0x100800..=0x100AFF => "FBHUB",
        0x100C00..=0x100FFF => "PFB_NISO/MMU",
        0x100000..=0x101FFF => "PFB",
        0x10A000..=0x10BFFF => "PMU_FALCON",
        0x122000..=0x122FFF => "PRI_MASTER",
        0x130000..=0x139FFF => "PCLOCK/CLK",
        0x140000..=0x17DFFF => "GPC",
        0x17E000..=0x18FFFF => "LTC",
        0x1FA000..=0x1FAFFF => "PMEM",
        0x700000..=0x7FFFFF => "PRAMIN",
        0x800000..=0x8FFFFF => "PCCSR",
        0x900000..=0x9BFFFF => "FBPA",
        _ => "UNKNOWN",
    }
}
