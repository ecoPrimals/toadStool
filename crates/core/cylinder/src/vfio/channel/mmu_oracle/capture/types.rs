// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serializable page table and channel capture types.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Decode a VRAM physical address from a V2 PDE.
/// Encoding: `(phys >> 4) | flags`, so `addr = (entry & ~0xF) << 4`.
pub fn decode_entry_addr(entry: u64) -> u64 {
    (entry & !0xF) << 4
}

/// Decode aperture and flags from a V2 PDE/PTE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryFlags {
    pub valid: bool,
    pub aperture: u8,
    pub aperture_name: Cow<'static, str>,
    pub vol: bool,
}

impl EntryFlags {
    pub fn decode(entry: u64) -> Self {
        let aperture = ((entry >> 1) & 3) as u8;
        Self {
            valid: (entry & 1) != 0,
            aperture,
            aperture_name: Cow::Borrowed(match aperture {
                0 => "INVALID",
                1 => "VRAM",
                2 => "SYS_COH",
                3 => "SYS_NCOH",
                _ => "?",
            }),
            vol: ((entry >> 3) & 1) != 0,
        }
    }
}

/// A single page directory or page table entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageEntry {
    pub index: u32,
    pub raw: u64,
    pub decoded_addr: u64,
    pub flags: EntryFlags,
}

/// A page directory level (PD3, PD2, PD1, PD0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDirectory {
    pub level: String,
    pub vram_addr: u64,
    pub entries: Vec<PageEntry>,
}

/// PD0 dual entry (small + large PDE per slot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pd0Entry {
    pub index: u32,
    pub small: PageEntry,
    pub large: PageEntry,
}

/// A page table (512 entries of 8 bytes each).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTable {
    pub vram_addr: u64,
    pub pd0_index: u32,
    pub entries: Vec<PageEntry>,
}

/// Channel instance block fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceBlock {
    pub vram_addr: u64,
    pub pdb_lo: u32,
    pub pdb_hi: u32,
    pub pd3_vram_addr: u64,
    pub ramfc_userd_lo: u32,
    pub ramfc_userd_hi: u32,
    pub ramfc_gp_base_lo: u32,
    pub ramfc_gp_base_hi: u32,
    pub sc0_pdb_lo: u32,
    pub sc0_pdb_hi: u32,
    pub addr_limit_lo: u32,
    pub addr_limit_hi: u32,
}

/// Captured channel from PCCSR scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub channel_id: u32,
    pub pccsr_inst_raw: u32,
    pub pccsr_channel_raw: u32,
    pub enabled: bool,
    pub instance_block: InstanceBlock,
}

pub use super::super::engine_regs::EngineRegisters;

/// Full page table dump with engine register state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTableDump {
    pub bdf: String,
    pub driver: String,
    pub boot0: u32,
    pub timestamp: String,
    pub channels: Vec<ChannelCapture>,
    pub engine_registers: EngineRegisters,
}

/// Full capture of a single channel's page table chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCapture {
    pub info: ChannelInfo,
    pub pd3: PageDirectory,
    pub pd2_dirs: Vec<PageDirectory>,
    pub pd1_dirs: Vec<PageDirectory>,
    pub pd0_dirs: Vec<Pd0Directory>,
    pub page_tables: Vec<PageTable>,
}

/// A PD0-level directory with dual entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pd0Directory {
    pub vram_addr: u64,
    pub entries: Vec<Pd0Entry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_entry_addr_zero() {
        assert_eq!(decode_entry_addr(0), 0);
    }

    #[test]
    fn decode_entry_addr_strips_flags() {
        assert_eq!(decode_entry_addr(0xF), 0, "low nibble is flags only");
    }

    #[test]
    fn decode_entry_addr_shifts_correctly() {
        let entry = 0x0010_0000u64;
        let expected = entry << 4;
        assert_eq!(decode_entry_addr(entry), expected);
    }

    #[test]
    fn decode_entry_addr_roundtrip() {
        let phys = 0x0001_2345_6780_0000u64;
        let encoded = (phys >> 4) | 0x5;
        let decoded = decode_entry_addr(encoded);
        assert_eq!(decoded, phys);
    }

    #[test]
    fn entry_flags_invalid_when_zero() {
        let flags = EntryFlags::decode(0);
        assert!(!flags.valid);
        assert_eq!(flags.aperture, 0);
        assert!(!flags.vol);
    }

    #[test]
    fn entry_flags_valid_vram() {
        let entry = 0x1 | (0x1 << 1);
        let flags = EntryFlags::decode(entry);
        assert!(flags.valid);
        assert_eq!(flags.aperture, 1);
        assert_eq!(flags.aperture_name, "VRAM");
    }

    #[test]
    fn entry_flags_valid_sys_coh() {
        let entry = 0x1 | (0x2 << 1);
        let flags = EntryFlags::decode(entry);
        assert!(flags.valid);
        assert_eq!(flags.aperture, 2);
        assert_eq!(flags.aperture_name, "SYS_COH");
    }

    #[test]
    fn entry_flags_valid_sys_ncoh() {
        let entry = 0x1 | (0x3 << 1);
        let flags = EntryFlags::decode(entry);
        assert!(flags.valid);
        assert_eq!(flags.aperture, 3);
        assert_eq!(flags.aperture_name, "SYS_NCOH");
    }

    #[test]
    fn entry_flags_volatile_bit() {
        let entry = 0x1 | (1 << 3);
        let flags = EntryFlags::decode(entry);
        assert!(flags.vol);
    }

    #[test]
    fn entry_flags_serde_roundtrip() {
        let flags = EntryFlags::decode(0x1 | (0x2 << 1) | (1 << 3));
        let json = serde_json::to_string(&flags).expect("serialize");
        let rt: EntryFlags = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt.valid, flags.valid);
        assert_eq!(rt.aperture, flags.aperture);
        assert_eq!(rt.aperture_name, flags.aperture_name);
        assert_eq!(rt.vol, flags.vol);
    }

    #[test]
    fn page_entry_serde_roundtrip() {
        let entry = PageEntry {
            index: 42,
            raw: 0xDEAD_BEEF_0000_0001,
            decoded_addr: decode_entry_addr(0xDEAD_BEEF_0000_0001),
            flags: EntryFlags::decode(0xDEAD_BEEF_0000_0001),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let rt: PageEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt.index, 42);
        assert_eq!(rt.raw, entry.raw);
        assert_eq!(rt.decoded_addr, entry.decoded_addr);
    }
}
