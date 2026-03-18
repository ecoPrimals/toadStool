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
//! // let mut alloc = DmaAllocator::new(container_raw_fd);
//! // let buf = alloc.allocate(4096)?;
//! // buf.as_mut_slice()[..4].copy_from_slice(&[1, 2, 3, 4]);
//! // let device_addr = buf.iova(); // pass to GPU command buffer
//! # Ok(())
//! # }
//! ```

use crate::error::{NvPmuError, Result};
use rustix::mm::{MapFlags, ProtFlags, mlock, mmap_anonymous, munlock, munmap};
use std::os::fd::{BorrowedFd, RawFd};

const PAGE_SIZE: usize = 4096;
const HUGE_PAGE_2M: usize = 2 * 1024 * 1024;
const HUGE_PAGE_1G: usize = 1024 * 1024 * 1024;

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
            Self::Huge2M => HUGE_PAGE_2M,
            Self::Huge1G => HUGE_PAGE_1G,
        }
    }
}
const IOVA_BASE: u64 = 0x1000_0000;

/// VFIO DMA mapping request (matches kernel ABI).
#[repr(C)]
struct VfioDmaMap {
    argsz: u32,
    flags: u32,
    vaddr: u64,
    iova: u64,
    size: u64,
}

/// VFIO DMA unmapping request (matches kernel ABI).
#[repr(C)]
struct VfioDmaUnmap {
    argsz: u32,
    flags: u32,
    iova: u64,
    size: u64,
}

const VFIO_DMA_MAP_FLAG_READ: u32 = 1;
const VFIO_DMA_MAP_FLAG_WRITE: u32 = 2;

use rustix::ioctl::{Ioctl, IoctlOutput, Opcode, opcode};

const VFIO_TYPE: u8 = b';';
const VFIO_BASE: u8 = 100;
const OP_IOMMU_MAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 13);
const OP_IOMMU_UNMAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 14);

struct DmaIoctl<const OP: Opcode, T> {
    ptr: *mut T,
}

// SAFETY: T is repr(C) matching kernel ABI; opcode is compile-time constant.
unsafe impl<const OP: Opcode, T> Ioctl for DmaIoctl<OP, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }
    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

/// DMA buffer: page-aligned, mlock'd, IOMMU-mapped memory.
///
/// The buffer is accessible from both host (via slice) and device (via IOVA).
/// Automatically unmapped and freed on drop.
pub struct DmaBuffer {
    vaddr: *mut u8,
    iova: u64,
    size: usize,
    container_fd: RawFd,
    /// When true, memory was allocated via `mmap` (huge pages); use `munmap` on drop.
    /// When false, memory was allocated via `alloc_zeroed`; use `dealloc` on drop.
    huge_page: bool,
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
        debug_assert!(!self.vaddr.is_null());
        // SAFETY: vaddr from alloc in allocate(), valid for size bytes; &self prevents mutation.
        unsafe { std::slice::from_raw_parts(self.vaddr, self.size) }
    }

    /// Host-accessible mutable view for writing data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(!self.vaddr.is_null());
        // SAFETY: vaddr valid for size; &mut self guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.vaddr, self.size) }
    }
}

impl Drop for DmaBuffer {
    #[allow(clippy::cast_possible_truncation, reason = "struct sizes fit u32")]
    fn drop(&mut self) {
        // SAFETY: munlock matches mlock from allocate() or allocate_huge().
        unsafe {
            let _ = munlock(self.vaddr.cast(), self.size);
        }

        let mut unmap = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: self.iova,
            size: self.size as u64,
        };

        // SAFETY: container_fd still valid — DmaBuffer dropped before container.
        let fd = unsafe { BorrowedFd::borrow_raw(self.container_fd) };
        let ioctl = DmaIoctl::<OP_IOMMU_UNMAP_DMA, _> {
            ptr: std::ptr::from_mut(&mut unmap),
        };
        let _ = unsafe { rustix::ioctl::ioctl(fd, ioctl) };

        if self.huge_page {
            // SAFETY: munmap matches mmap_anonymous from allocate_huge(); same ptr and size.
            unsafe {
                let _ = munmap(self.vaddr.cast(), self.size);
            }
        } else {
            let layout = std::alloc::Layout::from_size_align(self.size, PAGE_SIZE)
                .expect("Layout valid: matches alloc");
            // SAFETY: dealloc matches alloc_zeroed from allocate(); same layout.
            unsafe { std::alloc::dealloc(self.vaddr, layout) };
        }

        tracing::debug!(iova = %format!("{:#x}", self.iova), "freed DMA buffer");
    }
}

// SAFETY: DmaBuffer owns its allocation exclusively.
unsafe impl Send for DmaBuffer {}
// SAFETY: Reads via &self are safe; writes require &mut self.
unsafe impl Sync for DmaBuffer {}

/// IOVA allocator for DMA buffers on a VFIO container.
///
/// Manages the IOVA address space and provides page-aligned, IOMMU-mapped
/// allocations for host-device data transfer.
pub struct DmaAllocator {
    container_fd: RawFd,
    next_iova: u64,
}

impl DmaAllocator {
    /// Create a new allocator for the given VFIO container fd.
    #[must_use]
    pub const fn new(container_fd: RawFd) -> Self {
        Self {
            container_fd,
            next_iova: IOVA_BASE,
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
    #[allow(clippy::cast_possible_truncation, reason = "struct sizes fit u32")]
    pub fn allocate(&mut self, size: usize) -> Result<DmaBuffer> {
        if size == 0 {
            return Err(NvPmuError::Hardware(
                "DMA buffer size must be > 0".to_string(),
            ));
        }

        let aligned_size = size.div_ceil(PAGE_SIZE) * PAGE_SIZE;
        let layout = std::alloc::Layout::from_size_align(aligned_size, PAGE_SIZE)
            .map_err(|e| NvPmuError::Hardware(format!("Invalid DMA buffer layout: {e}")))?;

        // SAFETY: Page-aligned allocation; layout validated above; checked for null below.
        let vaddr = unsafe { std::alloc::alloc_zeroed(layout) };
        if vaddr.is_null() {
            return Err(NvPmuError::Hardware(
                "DMA buffer allocation failed (OOM)".to_string(),
            ));
        }

        // SAFETY: mlock prevents page-out; vaddr valid for aligned_size bytes.
        if let Err(e) = unsafe { mlock(vaddr.cast(), aligned_size) } {
            unsafe { std::alloc::dealloc(vaddr, layout) };
            return Err(NvPmuError::Hardware(format!("mlock failed: {e}")));
        }

        let iova = self.next_iova;
        let mut dma_map = VfioDmaMap {
            argsz: std::mem::size_of::<VfioDmaMap>() as u32,
            flags: VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE,
            vaddr: vaddr as u64,
            iova,
            size: aligned_size as u64,
        };

        let fd = unsafe { BorrowedFd::borrow_raw(self.container_fd) };
        let ioctl = DmaIoctl::<OP_IOMMU_MAP_DMA, _> {
            ptr: std::ptr::from_mut(&mut dma_map),
        };

        // SAFETY: container_fd from valid VFIO open; struct has correct argsz and layout.
        if let Err(e) = unsafe { rustix::ioctl::ioctl(fd, ioctl) } {
            unsafe {
                let _ = munlock(vaddr.cast(), aligned_size);
                std::alloc::dealloc(vaddr, layout);
            }
            return Err(NvPmuError::Hardware(format!("VFIO DMA map failed: {e}")));
        }

        self.next_iova += aligned_size as u64;

        tracing::debug!(
            iova = %format!("{iova:#x}"),
            size = aligned_size,
            "allocated DMA buffer"
        );

        Ok(DmaBuffer {
            vaddr,
            iova,
            size: aligned_size,
            container_fd: self.container_fd,
            huge_page: false,
        })
    }

    /// Allocate a DMA buffer using huge pages for high-performance transfers.
    ///
    /// For [`HugePageSize::Standard`], delegates to [`allocate`](Self::allocate).
    /// For 2M/1G huge pages, uses `mmap_anonymous` with `MAP_HUGETLB` instead
    /// of `alloc_zeroed`. Size is rounded up to the page size boundary.
    ///
    /// # Errors
    ///
    /// Returns error if huge pages are unavailable, allocation, mlock, or
    /// IOMMU mapping fails.
    #[allow(clippy::cast_possible_truncation, reason = "struct sizes fit u32")]
    pub fn allocate_huge(&mut self, size: usize, page_size: HugePageSize) -> Result<DmaBuffer> {
        if size == 0 {
            return Err(NvPmuError::Hardware(
                "DMA buffer size must be > 0".to_string(),
            ));
        }

        match page_size {
            HugePageSize::Standard => return self.allocate(size),
            HugePageSize::Huge2M | HugePageSize::Huge1G => {}
        }

        let page_sz = page_size.page_size();
        let aligned_size = size.div_ceil(page_sz) * page_sz;

        let map_flags = match page_size {
            HugePageSize::Huge2M => MapFlags::hugetlb_with_size_log2(21)
                .ok_or_else(|| NvPmuError::Hardware("MAP_HUGETLB 2M flag unsupported".into()))?,
            HugePageSize::Huge1G => MapFlags::hugetlb_with_size_log2(30)
                .ok_or_else(|| NvPmuError::Hardware("MAP_HUGETLB 1G flag unsupported".into()))?,
            HugePageSize::Standard => {
                return Err(NvPmuError::Hardware(
                    "Standard pages handled by allocate(), not allocate_huge()".into(),
                ));
            }
        };

        // SAFETY: mmap_anonymous creates a fresh mapping; we own the returned ptr.
        let vaddr = unsafe {
            mmap_anonymous(
                std::ptr::null_mut(),
                aligned_size,
                ProtFlags::READ | ProtFlags::WRITE,
                map_flags,
            )
        }
        .map_err(|e| NvPmuError::Hardware(format!("huge page mmap failed: {e}")))?
        .cast::<u8>();

        if vaddr.is_null() {
            return Err(NvPmuError::Hardware(
                "DMA huge page allocation failed (mmap returned null)".to_string(),
            ));
        }

        // SAFETY: mlock prevents page-out; vaddr valid for aligned_size bytes.
        if let Err(e) = unsafe { mlock(vaddr.cast(), aligned_size) } {
            unsafe {
                let _ = munmap(vaddr.cast(), aligned_size);
            }
            return Err(NvPmuError::Hardware(format!("mlock failed: {e}")));
        }

        let iova = self.next_iova;
        let mut dma_map = VfioDmaMap {
            argsz: std::mem::size_of::<VfioDmaMap>() as u32,
            flags: VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE,
            vaddr: vaddr as u64,
            iova,
            size: aligned_size as u64,
        };

        let fd = unsafe { BorrowedFd::borrow_raw(self.container_fd) };
        let ioctl = DmaIoctl::<OP_IOMMU_MAP_DMA, _> {
            ptr: std::ptr::from_mut(&mut dma_map),
        };

        if let Err(e) = unsafe { rustix::ioctl::ioctl(fd, ioctl) } {
            unsafe {
                let _ = munlock(vaddr.cast(), aligned_size);
                let _ = munmap(vaddr.cast(), aligned_size);
            }
            return Err(NvPmuError::Hardware(format!("VFIO DMA map failed: {e}")));
        }

        self.next_iova += aligned_size as u64;

        tracing::debug!(
            iova = %format!("{iova:#x}"),
            size = aligned_size,
            page_size = ?page_size,
            "allocated DMA buffer (huge pages)"
        );

        Ok(DmaBuffer {
            vaddr,
            iova,
            size: aligned_size,
            container_fd: self.container_fd,
            huge_page: true,
        })
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

    #[test]
    fn zero_size_rejected() {
        let mut alloc = DmaAllocator::new(-1);
        assert!(alloc.allocate(0).is_err());
    }

    #[test]
    fn page_alignment_math() {
        assert_eq!(1usize.div_ceil(PAGE_SIZE) * PAGE_SIZE, 4096);
        assert_eq!(4096usize.div_ceil(PAGE_SIZE) * PAGE_SIZE, 4096);
        assert_eq!(4097usize.div_ceil(PAGE_SIZE) * PAGE_SIZE, 8192);
        assert_eq!(8192usize.div_ceil(PAGE_SIZE) * PAGE_SIZE, 8192);
    }

    #[test]
    fn huge_page_size_constants() {
        assert_eq!(HUGE_PAGE_2M, 2 * 1024 * 1024);
        assert_eq!(HUGE_PAGE_1G, 1024 * 1024 * 1024);
    }

    #[test]
    fn huge_page_size_enum() {
        assert_eq!(HugePageSize::Standard.page_size(), PAGE_SIZE);
        assert_eq!(HugePageSize::Huge2M.page_size(), HUGE_PAGE_2M);
        assert_eq!(HugePageSize::Huge1G.page_size(), HUGE_PAGE_1G);
    }

    #[test]
    fn huge_page_alignment_math() {
        assert_eq!(1usize.div_ceil(HUGE_PAGE_2M) * HUGE_PAGE_2M, HUGE_PAGE_2M);
        assert_eq!(
            (HUGE_PAGE_2M + 1).div_ceil(HUGE_PAGE_2M) * HUGE_PAGE_2M,
            2 * HUGE_PAGE_2M
        );
        assert_eq!(1usize.div_ceil(HUGE_PAGE_1G) * HUGE_PAGE_1G, HUGE_PAGE_1G);
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
        assert!(std::mem::size_of::<VfioDmaMap>() >= 32);
    }

    #[test]
    fn dma_unmap_struct_layout() {
        assert!(std::mem::size_of::<VfioDmaUnmap>() >= 24);
    }
}
