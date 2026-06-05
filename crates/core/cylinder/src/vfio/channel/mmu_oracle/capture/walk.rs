// SPDX-License-Identifier: AGPL-3.0-or-later
//! Page table hierarchy walk (PD3 → PD2 → PD1 → PD0 → PT) via PRAMIN.

use super::super::super::registers::{pccsr, ramin};
use super::bar0::Bar0Rw;
use super::types::{
    ChannelCapture, ChannelInfo, EntryFlags, InstanceBlock, PageDirectory, PageEntry, PageTable,
    Pd0Directory, Pd0Entry, decode_entry_addr,
};

pub(super) fn scan_channels(bar0: &Bar0Rw) -> Vec<(u32, u32, u32)> {
    let mut channels = Vec::new();
    for id in 0..512u32 {
        let inst_reg = bar0.read_u32(pccsr::inst(id));
        if inst_reg == 0 || inst_reg == 0xFFFF_FFFF || inst_reg == 0xBADF_1000 {
            continue;
        }
        let chan_reg = bar0.read_u32(pccsr::channel(id));
        channels.push((id, inst_reg, chan_reg));
    }
    channels
}

fn read_instance_block(bar0: &Bar0Rw, inst_vram_addr: u64) -> InstanceBlock {
    let pdb_lo = bar0.read_vram_u32(inst_vram_addr + ramin::PAGE_DIR_BASE_LO as u64);
    let pdb_hi = bar0.read_vram_u32(inst_vram_addr + ramin::PAGE_DIR_BASE_HI as u64);
    let pd3_vram_addr = (pdb_lo as u64 & 0xFFFF_F000) | ((pdb_hi as u64) << 32);

    InstanceBlock {
        vram_addr: inst_vram_addr,
        pdb_lo,
        pdb_hi,
        pd3_vram_addr,
        ramfc_userd_lo: bar0.read_vram_u32(inst_vram_addr + 0x008),
        ramfc_userd_hi: bar0.read_vram_u32(inst_vram_addr + 0x00C),
        ramfc_gp_base_lo: bar0.read_vram_u32(inst_vram_addr + 0x010),
        ramfc_gp_base_hi: bar0.read_vram_u32(inst_vram_addr + 0x014),
        sc0_pdb_lo: bar0.read_vram_u32(inst_vram_addr + ramin::SC0_PAGE_DIR_BASE_LO as u64),
        sc0_pdb_hi: bar0.read_vram_u32(inst_vram_addr + ramin::SC0_PAGE_DIR_BASE_HI as u64),
        addr_limit_lo: bar0.read_vram_u32(inst_vram_addr + ramin::ADDR_LIMIT_LO as u64),
        addr_limit_hi: bar0.read_vram_u32(inst_vram_addr + ramin::ADDR_LIMIT_HI as u64),
    }
}

fn read_pd_entries(bar0: &Bar0Rw, pd_vram_addr: u64, max_entries: u32) -> Vec<PageEntry> {
    let mut entries = Vec::new();
    for i in 0..max_entries {
        let raw = bar0.read_vram_u64(pd_vram_addr + (i as u64) * 8);
        if raw == 0 {
            continue;
        }
        entries.push(PageEntry {
            index: i,
            raw,
            decoded_addr: decode_entry_addr(raw),
            flags: EntryFlags::decode(raw),
        });
    }
    entries
}

fn read_pd0_entries(bar0: &Bar0Rw, pd0_vram_addr: u64, max_entries: u32) -> Vec<Pd0Entry> {
    let mut entries = Vec::new();
    for i in 0..max_entries {
        let base = pd0_vram_addr + (i as u64) * 16;
        let small_raw = bar0.read_vram_u64(base);
        let large_raw = bar0.read_vram_u64(base + 8);
        if small_raw == 0 && large_raw == 0 {
            continue;
        }
        entries.push(Pd0Entry {
            index: i,
            small: PageEntry {
                index: i,
                raw: small_raw,
                decoded_addr: decode_entry_addr(small_raw),
                flags: EntryFlags::decode(small_raw),
            },
            large: PageEntry {
                index: i,
                raw: large_raw,
                decoded_addr: decode_entry_addr(large_raw),
                flags: EntryFlags::decode(large_raw),
            },
        });
    }
    entries
}

fn read_pt_entries(bar0: &Bar0Rw, pt_vram_addr: u64) -> Vec<PageEntry> {
    let mut entries = Vec::new();
    for i in 0..512u32 {
        let raw = bar0.read_vram_u64(pt_vram_addr + (i as u64) * 8);
        if raw == 0 {
            continue;
        }
        entries.push(PageEntry {
            index: i,
            raw,
            decoded_addr: decode_entry_addr(raw),
            flags: EntryFlags::decode(raw),
        });
    }
    entries
}

/// Walk the full page table chain for one channel, capturing all non-zero entries.
pub(super) fn walk_channel_page_tables(bar0: &Bar0Rw, info: &ChannelInfo) -> ChannelCapture {
    let pd3_addr = info.instance_block.pd3_vram_addr;

    // PD3: up to 16 entries (covers 512 TB VA space, but most GPUs use fewer)
    let pd3_entries = read_pd_entries(bar0, pd3_addr, 16);
    let pd3 = PageDirectory {
        level: "PD3".into(),
        vram_addr: pd3_addr,
        entries: pd3_entries.clone(),
    };

    let mut pd2_dirs = Vec::new();
    let mut pd1_dirs = Vec::new();
    let mut pd0_dirs = Vec::new();
    let mut page_tables = Vec::new();

    // Walk PD2 for each populated PD3 entry
    for pd3_e in &pd3_entries {
        let pd2_addr = pd3_e.decoded_addr;
        if pd2_addr == 0 || pd3_e.flags.aperture == 0 {
            continue;
        }
        let pd2_entries = read_pd_entries(bar0, pd2_addr, 512);
        pd2_dirs.push(PageDirectory {
            level: format!("PD2[from PD3[{}]]", pd3_e.index),
            vram_addr: pd2_addr,
            entries: pd2_entries.clone(),
        });

        // Walk PD1 for each populated PD2 entry
        for pd2_e in &pd2_entries {
            let pd1_addr = pd2_e.decoded_addr;
            if pd1_addr == 0 || pd2_e.flags.aperture == 0 {
                continue;
            }
            let pd1_entries = read_pd_entries(bar0, pd1_addr, 512);
            pd1_dirs.push(PageDirectory {
                level: format!("PD1[from PD2[{}]]", pd2_e.index),
                vram_addr: pd1_addr,
                entries: pd1_entries.clone(),
            });

            // Walk PD0 for each populated PD1 entry
            for pd1_e in &pd1_entries {
                let pd0_addr = pd1_e.decoded_addr;
                if pd0_addr == 0 || pd1_e.flags.aperture == 0 {
                    continue;
                }
                let pd0_entries = read_pd0_entries(bar0, pd0_addr, 512);
                pd0_dirs.push(Pd0Directory {
                    vram_addr: pd0_addr,
                    entries: pd0_entries.clone(),
                });

                // Walk PT for each populated PD0 small PDE
                for pd0_e in &pd0_entries {
                    let pt_addr = pd0_e.small.decoded_addr;
                    if pt_addr == 0 || pd0_e.small.flags.aperture == 0 {
                        continue;
                    }
                    let pt_entries = read_pt_entries(bar0, pt_addr);
                    if !pt_entries.is_empty() {
                        page_tables.push(PageTable {
                            vram_addr: pt_addr,
                            pd0_index: pd0_e.index,
                            entries: pt_entries,
                        });
                    }
                }
            }
        }
    }

    ChannelCapture {
        info: info.clone(),
        pd3,
        pd2_dirs,
        pd1_dirs,
        pd0_dirs,
        page_tables,
    }
}

pub(super) fn channel_info_from_scan(
    bar0: &Bar0Rw,
    id: u32,
    inst_reg: u32,
    chan_reg: u32,
) -> ChannelInfo {
    let inst_ptr_shifted = inst_reg & 0x0FFF_FFFF;
    let inst_vram_addr = (inst_ptr_shifted as u64) << 12;
    let enabled = (chan_reg & 1) != 0;

    let instance_block = read_instance_block(bar0, inst_vram_addr);
    ChannelInfo {
        channel_id: id,
        pccsr_inst_raw: inst_reg,
        pccsr_channel_raw: chan_reg,
        enabled,
        instance_block,
    }
}
