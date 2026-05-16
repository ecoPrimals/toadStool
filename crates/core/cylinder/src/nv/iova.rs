// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unified IOVA (I/O Virtual Address) layout manager for NVIDIA VFIO dispatch.
//!
//! Centralizes the IOVA address space layout that was previously scattered
//! across `registers.rs`, `compute_device.rs`, and `nv_gsp_bridge.rs`.
//! Prevents collisions and documents the full memory map in one place.
//!
//! # IOVA Space Layout (4 MiB total)
//!
//! ```text
//! 0x0_0000 ─ 0x0_2FFF   Guard pages (3 × 4 KiB, unmapped)
//! 0x0_3000               Instance block (4 KiB)
//! 0x0_4000               Runlist (4 KiB)
//! 0x0_5000 ─ 0x0_9000   Page tables: PD3, PD2, PD1, PD0, PT0 (5 × 4 KiB)
//! 0x0_A000               MMU fault buffer (4 KiB)
//! 0x0_B000               NOP pushbuffer (4 KiB)
//! 0x0_C000 ─ 0x0_FFFF   Reserved
//! 0x1_0000               GPFIFO ring (4 KiB)
//! 0x1_1000               USERD page (4 KiB)
//! 0x1_2000 ─ 0x11_FFFF  GR context save area (1 MiB)
//! 0x12_0000+             User DMA buffers (bump-allocated)
//!   ... dynamic region ...
//! 0x30_0000              FECS firmware code (64 KiB)
//! 0x31_0000              FECS firmware data (64 KiB)
//! 0x32_0000              GPCCS firmware code (64 KiB)
//! 0x33_0000              GPCCS firmware data (64 KiB)
//! 0x40_0000              IOVA limit
//! ```

/// Fixed IOVA regions for channel infrastructure.
pub mod channel {
    /// Guard page 0 — null pointer trap (unmapped).
    pub const GUARD0_IOVA: u64 = 0x0000;
    /// Guard page 1.
    pub const GUARD1_IOVA: u64 = 0x1000;
    /// Guard page 2.
    pub const GUARD2_IOVA: u64 = 0x2000;
    /// Instance block.
    pub const INSTANCE_IOVA: u64 = 0x3000;
    /// Runlist.
    pub const RUNLIST_IOVA: u64 = 0x4000;
    /// PD3 (level-4 page directory).
    pub const PD3_IOVA: u64 = 0x5000;
    /// PD2 (level-3 page directory).
    pub const PD2_IOVA: u64 = 0x6000;
    /// PD1 (level-2 page directory).
    pub const PD1_IOVA: u64 = 0x7000;
    /// PD0 (level-1 page directory).
    pub const PD0_IOVA: u64 = 0x8000;
    /// PT0 (small page table).
    pub const PT0_IOVA: u64 = 0x9000;
    /// Non-replayable MMU fault buffer.
    pub const FAULT_BUF_IOVA: u64 = 0xA000;
    /// NOP pushbuffer.
    pub const NOP_PB_IOVA: u64 = 0xB000;
    /// End of channel infrastructure region.
    pub const CHANNEL_REGION_END: u64 = 0x1_0000;
}

/// Fixed IOVA regions for compute dispatch infrastructure.
pub mod dispatch {
    /// GPFIFO ring buffer.
    pub const GPFIFO_IOVA: u64 = 0x1_0000;
    /// USERD (user-driver state descriptor) page.
    pub const USERD_IOVA: u64 = 0x1_1000;
    /// GR context save area start.
    pub const GR_CTX_IOVA: u64 = 0x1_2000;
    /// GR context buffer size (1 MiB — GV100 needs ~500 KiB).
    pub const GR_CTX_SIZE: usize = 0x10_0000;
    /// First IOVA for user DMA buffer allocations.
    pub const USER_BUFFER_BASE_IOVA: u64 = 0x12_0000;
}

/// Fixed IOVA regions for falcon firmware images.
pub mod firmware {
    /// FECS firmware code image.
    pub const FECS_CODE_IOVA: u64 = 0x0030_0000;
    /// FECS firmware data image.
    pub const FECS_DATA_IOVA: u64 = 0x0031_0000;
    /// GPCCS firmware code image.
    pub const GPCCS_CODE_IOVA: u64 = 0x0032_0000;
    /// GPCCS firmware data image.
    pub const GPCCS_DATA_IOVA: u64 = 0x0033_0000;
}

/// Global IOVA space limits.
pub const IOVA_LIMIT: u64 = 0x40_0000;
/// Page size for IOVA alignment.
pub const PAGE_SIZE: u64 = 4096;

/// Bump allocator for the user DMA buffer region.
///
/// Allocates IOVAs sequentially from `USER_BUFFER_BASE_IOVA` upward,
/// preventing collisions with the channel infrastructure and firmware
/// regions. Thread-safe usage is the caller's responsibility (typically
/// held behind `&mut self` on the compute device).
#[derive(Debug, Clone)]
pub struct IovaAllocator {
    next: u64,
    limit: u64,
}

impl IovaAllocator {
    /// Create a new allocator for the user buffer region.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next: dispatch::USER_BUFFER_BASE_IOVA,
            limit: firmware::FECS_CODE_IOVA, // stop before firmware region
        }
    }

    /// Create an allocator with custom bounds.
    #[must_use]
    pub fn with_bounds(base: u64, limit: u64) -> Self {
        Self { next: base, limit }
    }

    /// Allocate the next IOVA for a buffer of `size` bytes.
    ///
    /// Returns the aligned IOVA, or `None` if the space is exhausted.
    pub fn alloc(&mut self, size: usize) -> Option<u64> {
        let aligned = ((size as u64) + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        if self.next + aligned > self.limit {
            return None;
        }
        let iova = self.next;
        self.next += aligned;
        Some(iova)
    }

    /// Current allocation pointer (next IOVA to be returned).
    #[must_use]
    pub fn next_iova(&self) -> u64 {
        self.next
    }

    /// Remaining space in bytes.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.next)
    }

    /// Reset the allocator to the base address (for reuse after sync).
    pub fn reset(&mut self) {
        self.next = dispatch::USER_BUFFER_BASE_IOVA;
    }
}

impl Default for IovaAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_no_overlap() {
        assert!(channel::INSTANCE_IOVA < channel::RUNLIST_IOVA);
        assert!(channel::RUNLIST_IOVA < channel::PD3_IOVA);
        assert!(channel::PT0_IOVA < channel::FAULT_BUF_IOVA);
        assert!(channel::FAULT_BUF_IOVA < channel::NOP_PB_IOVA);
        assert!(channel::CHANNEL_REGION_END <= dispatch::GPFIFO_IOVA);
        assert!(dispatch::GPFIFO_IOVA < dispatch::USERD_IOVA);
        assert!(dispatch::USERD_IOVA < dispatch::GR_CTX_IOVA);
        assert!(dispatch::GR_CTX_IOVA + dispatch::GR_CTX_SIZE as u64 <= dispatch::USER_BUFFER_BASE_IOVA);
        assert!(dispatch::USER_BUFFER_BASE_IOVA < firmware::FECS_CODE_IOVA);
        assert!(firmware::GPCCS_DATA_IOVA + 0x1_0000 <= IOVA_LIMIT);
    }

    #[test]
    fn allocator_basic() {
        let mut alloc = IovaAllocator::new();
        let first = alloc.alloc(4096).unwrap();
        assert_eq!(first, dispatch::USER_BUFFER_BASE_IOVA);
        let second = alloc.alloc(8192).unwrap();
        assert_eq!(second, dispatch::USER_BUFFER_BASE_IOVA + 4096);
    }

    #[test]
    fn allocator_alignment() {
        let mut alloc = IovaAllocator::new();
        let iova = alloc.alloc(100).unwrap();
        assert_eq!(iova, dispatch::USER_BUFFER_BASE_IOVA);
        let next = alloc.next_iova();
        assert_eq!(next % PAGE_SIZE, 0, "next IOVA should be page-aligned");
    }

    #[test]
    fn allocator_exhaustion() {
        let mut alloc = IovaAllocator::with_bounds(0x100000, 0x102000);
        assert!(alloc.alloc(4096).is_some());
        assert!(alloc.alloc(4096).is_some());
        assert!(alloc.alloc(4096).is_none());
    }

    #[test]
    fn allocator_reset() {
        let mut alloc = IovaAllocator::new();
        let _ = alloc.alloc(4096);
        assert_ne!(alloc.next_iova(), dispatch::USER_BUFFER_BASE_IOVA);
        alloc.reset();
        assert_eq!(alloc.next_iova(), dispatch::USER_BUFFER_BASE_IOVA);
    }

    #[test]
    fn channel_regions_page_aligned() {
        for &iova in &[
            channel::INSTANCE_IOVA, channel::RUNLIST_IOVA,
            channel::PD3_IOVA, channel::PD2_IOVA, channel::PD1_IOVA,
            channel::PD0_IOVA, channel::PT0_IOVA, channel::FAULT_BUF_IOVA,
            channel::NOP_PB_IOVA, dispatch::GPFIFO_IOVA, dispatch::USERD_IOVA,
            dispatch::GR_CTX_IOVA, dispatch::USER_BUFFER_BASE_IOVA,
            firmware::FECS_CODE_IOVA, firmware::FECS_DATA_IOVA,
            firmware::GPCCS_CODE_IOVA, firmware::GPCCS_DATA_IOVA,
        ] {
            assert_eq!(iova % PAGE_SIZE, 0, "IOVA {iova:#x} is not page-aligned");
        }
    }
}
