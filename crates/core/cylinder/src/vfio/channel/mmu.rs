// SPDX-License-Identifier: AGPL-3.0-or-later
//! MMU fault buffer and page table helpers for [`super::VfioChannel`].

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;

use super::VfioChannel;
use super::page_tables;
use super::registers::{self, FAULT_BUF_IOVA, INSTANCE_IOVA, PT_ENTRIES, pfb};

/// Configure non-replayable and replayable MMU fault buffers.
pub(super) fn configure_fault_buffers(bar0: &MappedBar) -> DriverResult<()> {
    use registers::mmu;

    let fb_lo = (FAULT_BUF_IOVA >> 12) as u32;
    let fb_entries: u32 = 64;
    bar0.write_u32(mmu::FAULT_BUF0_LO, fb_lo)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF0_LO: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF0_HI, 0)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF0_HI: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF0_SIZE, fb_entries)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF0_SIZE: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF0_GET, 0)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF0_GET: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF0_PUT, 0x8000_0000)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF0_PUT: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF1_LO, fb_lo)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF1_LO: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF1_HI, 0)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF1_HI: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF1_SIZE, fb_entries)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF1_SIZE: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF1_GET, 0)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF1_GET: {e}"))))?;
    bar0.write_u32(mmu::FAULT_BUF1_PUT, 0x8000_0000)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("FAULT_BUF1_PUT: {e}"))))?;
    tracing::info!(
        fault_buf_iova = format_args!("{FAULT_BUF_IOVA:#x}"),
        entries = fb_entries,
        "MMU fault buffers configured (non-replayable + replayable)"
    );
    Ok(())
}

impl VfioChannel {
    /// Invalidate the GPU MMU TLB for our page directory base.
    ///
    /// Matches nouveau's `gf100_vmm_invalidate`: write the PDB address to
    /// `MMU_INVALIDATE_PDB`, then trigger with `PAGE_ALL | HUB_ONLY`.
    /// For system memory targets, PDB addr uses the IOVA with target=SYS_COH.
    pub(super) fn invalidate_tlb(bar0: &MappedBar, pd3_iova: u64) -> DriverResult<()> {
        // Wait for flush slot availability.
        for _ in 0..200 {
            let ctrl = bar0.read_u32(pfb::MMU_CTRL).unwrap_or(0);
            if ctrl & 0x00FF_0000 != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        // PDB address for invalidation: (iova >> 12) << 4 | target.
        // target=2 (SYS_MEM_COH) to match our page table aperture.
        let pdb_inv = ((pd3_iova >> 12) << 4) | 2; // SYS_MEM_COH target
        bar0.write_u32(pfb::MMU_INVALIDATE_PDB, pdb_inv as u32)
            .map_err(|e| {
                DriverError::SubmitFailed(Cow::Owned(format!("MMU_INVALIDATE_PDB: {e}")))
            })?;
        bar0.write_u32(pfb::MMU_INVALIDATE_PDB_HI, (pd3_iova >> 32) as u32)
            .map_err(|e| {
                DriverError::SubmitFailed(Cow::Owned(format!("MMU_INVALIDATE_PDB_HI: {e}")))
            })?;

        // Trigger: PAGE_ALL (bit 0) | HUB_ONLY (bit 2) | trigger (bit 31).
        bar0.write_u32(pfb::MMU_INVALIDATE, 0x8000_0005)
            .map_err(|e| {
                DriverError::SubmitFailed(Cow::Owned(format!("MMU_INVALIDATE trigger: {e}")))
            })?;

        // Wait for flush acknowledgement.
        for _ in 0..200 {
            let ctrl = bar0.read_u32(pfb::MMU_CTRL).unwrap_or(0);
            if ctrl & 0x0000_8000 != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        tracing::info!(
            pd3_iova = format_args!("{pd3_iova:#x}"),
            "GPU MMU TLB invalidated"
        );
        Ok(())
    }

    /// Write the GR context buffer virtual address into the instance block
    /// at offsets 0x210/0x214 (Nouveau: RAMFC_GR_CTX_PTR_LO/HI).
    ///
    /// The address should include the CB_RESERVED offset if using Nouveau's
    /// context generation layout. `flags` is OR'd into the low word (e.g. 4).
    pub fn write_gr_context_ptr(&mut self, ctx_vaddr: u64, flags: u32) {
        let lo = (ctx_vaddr as u32) | flags;
        let hi = (ctx_vaddr >> 32) as u32;
        let inst = self.instance.as_mut_slice();
        inst[0x210..0x214].copy_from_slice(&lo.to_le_bytes());
        inst[0x214..0x218].copy_from_slice(&hi.to_le_bytes());
        tracing::info!(
            ctx_vaddr = format_args!("{ctx_vaddr:#x}"),
            lo = format_args!("{lo:#010x}"),
            hi = format_args!("{hi:#010x}"),
            "wrote GR context pointer into instance block (0x210/0x214)"
        );
    }

    /// Return the DMA IOVA of this channel's instance block.
    pub fn instance_iova(&self) -> u64 {
        INSTANCE_IOVA
    }

    /// Map a VRAM region into this channel's page table.
    ///
    /// Writes VRAM-aperture PTEs into PT0 so the GPU can DMA between
    /// system memory and VRAM using the same virtual address space.
    /// `gpu_va_base` is the GPU virtual address where the mapping starts.
    /// `vram_phys_base` is the VRAM physical address to map.
    /// `num_pages` is the number of 4 KiB pages to map.
    ///
    /// The GPU VA must fall within the PT0 coverage range (first 2 MiB).
    pub fn map_vram_pages(
        &mut self,
        gpu_va_base: u64,
        vram_phys_base: u64,
        num_pages: usize,
    ) -> DriverResult<()> {
        let pt0_start_index = (gpu_va_base / 4096) as usize;
        if pt0_start_index + num_pages > PT_ENTRIES {
            return Err(DriverError::MmapFailed(Cow::Owned(format!(
                "VRAM mapping {gpu_va_base:#x}+{num_pages} pages exceeds PT0 range"
            ))));
        }
        let pt0 = self.pt0.as_mut_slice();
        for i in 0..num_pages {
            page_tables::write_vram_pte(
                pt0,
                pt0_start_index + i,
                vram_phys_base + (i as u64) * 4096,
            );
        }
        tracing::info!(
            gpu_va = format_args!("{gpu_va_base:#x}"),
            vram_phys = format_args!("{vram_phys_base:#x}"),
            num_pages,
            "mapped VRAM pages into PT0"
        );
        Ok(())
    }
}
