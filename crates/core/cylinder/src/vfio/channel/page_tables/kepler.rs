// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kepler (V1, 2-level) page table, instance block, and runlist encoding.
//!
//! GK104/GK110 use a simpler MMU model than Volta: a single page directory
//! (PD) with 16-byte entries (small PT + large PT), and a PT covering
//! 512 × 4 KiB = 2 MiB.

use super::write_u32_le;
use crate::vfio::channel::registers::*;

/// Encode a Kepler V1 PDE pointing to a small page table at `iova`.
///
/// GK110 PDE (from nouveau `gf100_vmm_pgd_pde`):
///   Lower 8 bytes: small PT descriptor
///     `[2:0] = target (2=SYS_COH), [4] = present, addr bits [35:8] at [35:8]`
///   Upper 8 bytes: large PT descriptor (unused here, left as 0).
///
/// The PDE is 16 bytes total (dual-entry), same width as V2 PD0.
pub(in crate::vfio::channel) fn encode_kepler_pde(iova: u64) -> u64 {
    const TARGET_SYS_COH: u64 = 2;
    const PRESENT: u64 = 1 << 4;
    let addr_bits = iova & 0x000F_FFFF_FF00;
    addr_bits | TARGET_SYS_COH | PRESENT
}

/// Encode a Kepler V1 small-page PTE for an identity-mapped physical address.
///
/// GK110 PTE (from nouveau `gf100_vmm_pgt_mem`):
///   `[0]=VALID, [2:1]=target (2=SYS_COH), [3]=VOL, addr bits [35:4] at [35:4]`
pub(in crate::vfio::channel) fn encode_kepler_pte(phys_addr: u64) -> u64 {
    const VALID: u64 = 1;
    const TARGET_SYS_COH: u64 = 2 << 1;
    const VOL: u64 = 1 << 3;
    let addr_bits = phys_addr & 0x000F_FFFF_FFF0;
    addr_bits | TARGET_SYS_COH | VOL | VALID
}

/// Populate Kepler 2-level page tables (PD + PT) with identity mapping.
///
/// Kepler uses a single page directory (PD) whose entries point to page tables.
/// Each PDE is 16 bytes (small PT + large PT). We only use the small PT half.
/// The PT covers 512 × 4 KiB = 2 MiB of VA space per entry.
pub(in crate::vfio::channel) fn populate_kepler_page_tables(pd: &mut [u8], pt0: &mut [u8], pt0_iova: u64) {
    let pde = encode_kepler_pde(pt0_iova);
    pd[0..8].copy_from_slice(&pde.to_le_bytes());

    for i in 1..PT_ENTRIES {
        let phys = (i as u64) * 4096;
        let pte = encode_kepler_pte(phys);
        let off = i * 8;
        pt0[off..off + 8].copy_from_slice(&pte.to_le_bytes());
    }
}

/// Populate a Kepler instance block (RAMFC + simple RAMIN PDB, no subcontexts).
///
/// Kepler's instance block is simpler than Volta's:
/// - RAMFC fields are at the same offsets (USERD, GP_BASE, etc.)
/// - RAMIN PDB at 0x200 uses V1 format: `addr[31:12] | TARGET[1:0] | VOL[2]`
/// - No subcontext array (SC0/SC1 fields don't exist)
/// - VA limit is 40-bit (1 TB), not 48-bit
#[expect(
    clippy::cast_possible_truncation,
    reason = "IOVA values and ilog2 results always fit u32"
)]
pub(in crate::vfio::channel) fn populate_kepler_instance_block(
    inst: &mut [u8],
    gpfifo_iova: u64,
    gpfifo_entries: u32,
    userd_iova: u64,
    channel_id: u32,
    pd_iova: u64,
) {
    let limit2 = gpfifo_entries.ilog2();

    write_u32_le(
        inst,
        ramfc::USERD_LO,
        (userd_iova as u32 & 0xFFFF_FE00) | PBDMA_TARGET_SYS_MEM_COHERENT,
    );
    write_u32_le(inst, ramfc::USERD_HI, (userd_iova >> 32) as u32);
    write_u32_le(inst, ramfc::SIGNATURE, 0x0000_FACE);
    write_u32_le(inst, ramfc::ACQUIRE, 0x7FFF_F902);

    // PB DMA limit/reference + subroutine config — inherited from nv50_chan_ramfc_write.
    // Without these, the PBDMA rejects the context during reload, triggering
    // SCHED_ERROR code=32 (CONTEXT_RELOAD_TIMEOUT) on GK104/GK210B.
    write_u32_le(inst, ramfc::DMA_LIMIT_REF, 0x003F_6078);
    write_u32_le(inst, ramfc::PB_DMA_SUBROUTINE, 0x0100_3FFF);

    write_u32_le(inst, ramfc::GP_BASE_LO, gpfifo_iova as u32);
    write_u32_le(
        inst,
        ramfc::GP_BASE_HI,
        (gpfifo_iova >> 32) as u32 | (limit2 << 16),
    );
    write_u32_le(inst, ramfc::GP_PUT, 0);
    write_u32_le(inst, ramfc::GP_GET, 0);
    write_u32_le(inst, ramfc::GP_FETCH, 0);

    write_u32_le(inst, ramfc::PB_HEADER, 0x2040_0000);
    write_u32_le(inst, ramfc::SUBDEVICE, 0x3000_0000 | 0xFFF);
    write_u32_le(inst, ramfc::HCE_CTRL, 0x0000_0020);
    write_u32_le(inst, ramfc::CHID, channel_id);

    // Kepler uses CONFIG register (0xA8) — unlike GV100 where it doesn't exist.
    write_u32_le(inst, ramfc::CONFIG, 0x0000_0400);
    write_u32_le(inst, ramfc::CHANNEL_INFO, 0x0300_0000 | channel_id);

    // RAMIN PDB — V1 format: `addr[31:12] | VOL[2] | TARGET[1:0]`
    // Target: 2 = SYS_MEM_COHERENT, VOL = bit 2
    let pdb_lo: u32 = ((pd_iova >> 12) as u32) << 12
        | (1 << 2)  // VOL
        | TARGET_SYS_MEM_COHERENT;
    write_u32_le(inst, ramin::PAGE_DIR_BASE_LO, pdb_lo);
    write_u32_le(inst, ramin::PAGE_DIR_BASE_HI, (pd_iova >> 32) as u32);

    // Kepler 40-bit VA limit (1 TB)
    write_u32_le(inst, ramin::ADDR_LIMIT_LO, 0xFFFF_FFFF);
    write_u32_le(inst, ramin::ADDR_LIMIT_HI, 0x0000_00FF);
}

/// Populate a GK104 runlist — 8-byte channel entries, no TSG header.
///
/// GK104 runlist entry format (from nouveau `gk104_runl_insert_chan`):
///   `[31:12] INST_PTR, [9:8] INST_TARGET, [0] CHANNEL_ENABLE`
///
/// Unlike GV100, there's no TSG header and each entry is 8 bytes (not 16).
pub(in crate::vfio::channel) fn populate_kepler_runlist(rl: &mut [u8], _instance_iova: u64, channel_id: u32) {
    // GK104 runlist entry format (from Nouveau gk104_fifo_runlist_commit):
    //   DW0 = channel_id
    //   DW1 = 0x00000004 (entry type = channel)
    // PFIFO reads the instance block address from PCCSR, not the runlist.
    write_u32_le(rl, 0x00, channel_id);
    write_u32_le(rl, 0x04, 0x0000_0004);
}
