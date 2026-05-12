// SPDX-License-Identifier: AGPL-3.0-or-later
//! DMA buffer management for VFIO GPU backend.
//!
//! Page-aligned, mlock'd, IOMMU-mapped memory buffers for zero-copy data
//! transfer between host and GPU hardware. Ported from the akida-driver
//! pattern to support sovereign GPU compute dispatch.
//!
//! # Usage
//!
//! DMA buffers are allocated through a [`DmaAllocator`] which manages
//! IOVA (I/O Virtual Address) allocation for a VFIO container.
//!
//! ```rust,no_run
//! # fn example() -> nvpmu::error::Result<()> {
//! // After opening VfioBar0Access, create an allocator for DMA:
//! // let mut alloc = DmaAllocator::new(container_owned_fd);
//! // let buf = alloc.allocate(4096)?;
//! // buf.as_mut_slice()[..4].copy_from_slice(&[1, 2, 3, 4]);
//! // let device_addr = buf.iova(); // pass to GPU command buffer
//! # Ok(())
//! # }
//! ```

use crate::error::{NvPmuError, Result};
use std::os::fd::{AsFd, OwnedFd};
use toadstool_hw_safe::LockedMemory;
use toadstool_hw_safe::huge_page::{self, HugePageMemory};
use toadstool_hw_safe::vfio_dma::{self, flags};

const PAGE_SIZE: usize = 4096;

/// Huge page size for DMA allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    /// Standard 4KB pages (uses regular `alloc`, not `mmap`).
    Standard,
    /// 2 `MiB` huge pages.
    Huge2M,
    /// 1 `GiB` huge pages.
    Huge1G,
}

impl HugePageSize {
    /// Page size in bytes.
    #[must_use]
    pub const fn page_size(&self) -> usize {
        match self {
            Self::Standard => PAGE_SIZE,
            Self::Huge2M => huge_page::HugePageSize::Huge2M.bytes(),
            Self::Huge1G => huge_page::HugePageSize::Huge1G.bytes(),
        }
    }
}
const IOVA_BASE: u64 = 0x1000_0000;

/// Backing storage for a DMA buffer — either standard locked or huge-page.
enum DmaMemory {
    Locked(LockedMemory),
    HugePage(HugePageMemory),
}

/// DMA buffer: page-aligned, mlock'd, IOMMU-mapped memory.
///
/// The buffer is accessible from both host (via slice) and device (via IOVA).
/// Automatically unmapped and freed on drop.
pub struct DmaBuffer {
    mem: DmaMemory,
    iova: u64,
    size: usize,
    container_fd: OwnedFd,
}

impl DmaBuffer {
    /// Device-visible I/O virtual address.
    #[must_use]
    pub const fn iova(&self) -> u64 {
        self.iova
    }

    /// Buffer size in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Host-accessible immutable view.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        match &self.mem {
            DmaMemory::Locked(m) => m.as_slice(),
            DmaMemory::HugePage(m) => m.as_slice(),
        }
    }

    /// Host-accessible mutable view for writing data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        match &mut self.mem {
            DmaMemory::Locked(m) => m.as_mut_slice(),
            DmaMemory::HugePage(m) => m.as_mut_slice(),
        }
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        let _ = vfio_dma::dma_unmap_region(self.container_fd.as_fd(), self.iova, self.size);
        tracing::debug!(iova = %format!("{:#x}", self.iova), "freed DMA buffer");
    }
}

/// IOVA allocator for DMA buffers on a VFIO container.
///
/// Manages the IOVA address space and provides page-aligned, IOMMU-mapped
/// allocations for host-device data transfer.
pub struct DmaAllocator {
    container_fd: OwnedFd,
    next_iova: u64,
}

impl DmaAllocator {
    /// Create a new allocator for the given VFIO container fd.
    #[must_use]
    pub fn new(container_fd: OwnedFd) -> Self {
        Self {
            container_fd,
            next_iova: IOVA_BASE,
        }
    }

    fn iommu_map(&self, buf: &DmaBuffer) -> Result<()> {
        match &buf.mem {
            DmaMemory::Locked(mem) => vfio_dma::dma_map_locked(
                self.container_fd.as_fd(),
                mem,
                buf.iova,
                flags::READ | flags::WRITE,
            )
            .map_err(|e| NvPmuError::Hardware(format!("VFIO DMA map failed: {e}"))),
            DmaMemory::HugePage(mem) => vfio_dma::dma_map_huge(
                self.container_fd.as_fd(),
                mem,
                buf.iova,
                flags::READ | flags::WRITE,
            )
            .map_err(|e| NvPmuError::Hardware(format!("VFIO DMA map failed: {e}"))),
        }
    }

    /// Allocate a DMA buffer of the given size.
    ///
    /// Size is rounded up to page alignment (4096). The buffer is zeroed,
    /// mlock'd, and IOMMU-mapped for bidirectional device access.
    ///
    /// # Errors
    ///
    /// Returns error if allocation, mlock, or IOMMU mapping fails.
    pub fn allocate(&mut self, size: usize) -> Result<DmaBuffer> {
        if size == 0 {
            return Err(NvPmuError::Hardware(
                "DMA buffer size must be > 0".to_string(),
            ));
        }

        let aligned_size = vfio_dma::page_align_up(size, PAGE_SIZE);
        let mem = LockedMemory::page_aligned(aligned_size)
            .map_err(|e| NvPmuError::Hardware(format!("locked DMA buffer: {e}")))?;

        let buf_fd = self
            .container_fd
            .try_clone()
            .map_err(|e| NvPmuError::Hardware(format!("dup container fd: {e}")))?;

        let buf = DmaBuffer {
            iova: self.next_iova,
            size: aligned_size,
            container_fd: buf_fd,
            mem: DmaMemory::Locked(mem),
        };
        self.iommu_map(&buf)?;
        self.next_iova += aligned_size as u64;

        tracing::debug!(
            iova = %format!("{:#x}", buf.iova),
            size = aligned_size,
            "allocated DMA buffer"
        );

        Ok(buf)
    }

    /// Allocate a DMA buffer using huge pages for high-performance transfers.
    ///
    /// For [`HugePageSize::Standard`], delegates to [`allocate`](Self::allocate).
    /// For 2M/1G huge pages, uses [`HugePageMemory`] (mmap + `MAP_HUGETLB`).
    /// Size is rounded up to the page size boundary.
    ///
    /// # Errors
    ///
    /// Returns error if huge pages are unavailable, allocation, mlock, or
    /// IOMMU mapping fails.
    pub fn allocate_huge(&mut self, size: usize, page_size: HugePageSize) -> Result<DmaBuffer> {
        if size == 0 {
            return Err(NvPmuError::Hardware(
                "DMA buffer size must be > 0".to_string(),
            ));
        }

        let hw_page_size = match page_size {
            HugePageSize::Standard => return self.allocate(size),
            HugePageSize::Huge2M => huge_page::HugePageSize::Huge2M,
            HugePageSize::Huge1G => huge_page::HugePageSize::Huge1G,
        };

        let hp_mem = HugePageMemory::new(size, hw_page_size)
            .map_err(|e| NvPmuError::Hardware(format!("huge page alloc: {e}")))?;
        let aligned_size = hp_mem.size();

        let buf_fd = self
            .container_fd
            .try_clone()
            .map_err(|e| NvPmuError::Hardware(format!("dup container fd: {e}")))?;

        let buf = DmaBuffer {
            iova: self.next_iova,
            size: aligned_size,
            container_fd: buf_fd,
            mem: DmaMemory::HugePage(hp_mem),
        };

        self.iommu_map(&buf)?;

        self.next_iova += aligned_size as u64;

        tracing::debug!(
            iova = %format!("{:#x}", buf.iova),
            size = aligned_size,
            page_size = ?page_size,
            "allocated DMA buffer (huge pages)"
        );

        Ok(buf)
    }
}

/// Check if the system supports huge pages (2M pages available).
///
/// Returns true if `/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages`
/// exists and has value > 0.
#[must_use]
pub fn supports_huge_pages() -> bool {
    let path = "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages";
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    content.trim().parse::<u32>().is_ok_and(|n| n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_fd() -> OwnedFd {
        std::fs::File::open("/dev/null").unwrap().into()
    }

    #[test]
    fn zero_size_rejected() {
        let mut alloc = DmaAllocator::new(dummy_fd());
        assert!(alloc.allocate(0).is_err());
    }

    #[test]
    fn page_alignment_math() {
        assert_eq!(vfio_dma::page_align_up(1, PAGE_SIZE), 4096);
        assert_eq!(vfio_dma::page_align_up(4096, PAGE_SIZE), 4096);
        assert_eq!(vfio_dma::page_align_up(4097, PAGE_SIZE), 8192);
        assert_eq!(vfio_dma::page_align_up(8192, PAGE_SIZE), 8192);
    }

    #[test]
    fn huge_page_size_enum() {
        assert_eq!(HugePageSize::Standard.page_size(), PAGE_SIZE);
        assert_eq!(HugePageSize::Huge2M.page_size(), 2 * 1024 * 1024);
        assert_eq!(HugePageSize::Huge1G.page_size(), 1024 * 1024 * 1024);
    }

    #[test]
    fn huge_page_alignment_math() {
        let two_mb = HugePageSize::Huge2M.page_size();
        let one_gb = HugePageSize::Huge1G.page_size();
        assert_eq!(1usize.div_ceil(two_mb) * two_mb, two_mb);
        assert_eq!((two_mb + 1).div_ceil(two_mb) * two_mb, 2 * two_mb);
        assert_eq!(1usize.div_ceil(one_gb) * one_gb, one_gb);
    }

    #[test]
    fn supports_huge_pages_does_not_panic() {
        // May be true or false depending on system; just ensure it doesn't panic.
        let _ = supports_huge_pages();
    }

    #[test]
    fn iova_base_value() {
        assert_eq!(IOVA_BASE, 0x1000_0000);
    }

    #[test]
    fn dma_map_struct_layout() {
        assert!(std::mem::size_of::<vfio_dma::VfioDmaMap>() >= 32);
    }

    #[test]
    fn dma_unmap_struct_layout() {
        assert!(std::mem::size_of::<vfio_dma::VfioDmaUnmap>() >= 24);
    }
}
