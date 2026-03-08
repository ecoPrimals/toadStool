// SPDX-License-Identifier: AGPL-3.0-or-later
//! DMA buffer management for VFIO NPU backend
//!
//! Provides page-aligned, mlock'd, IOMMU-mapped memory buffers for
//! zero-copy data transfer between host and NPU hardware.

use crate::error::{AkidaError, Result};
use rustix::mm::{mlock, munlock};
use std::os::fd::{BorrowedFd, RawFd};

use super::ioctl;
use super::types::ioctls;
use super::types::{VfioDmaMap, VfioDmaUnmap};

/// DMA buffer for fast host-to-device data transfer.
///
/// Memory is page-aligned (4096), mlock'd to prevent swapping, and IOMMU-mapped
/// so the device can access it via IOVA. Cleanup is automatic on drop.
#[derive(Debug)]
pub struct DmaBuffer {
    vaddr: *mut u8,
    iova: u64,
    size: usize,
    container_fd: RawFd,
}

impl DmaBuffer {
    /// Allocate a new DMA buffer mapped for device access.
    ///
    /// # Errors
    ///
    /// Returns an error if allocation, mlock, or IOMMU DMA mapping fails.
    pub(crate) fn new(container_fd: RawFd, size: usize, iova: u64) -> Result<Self> {
        if size == 0 {
            return Err(AkidaError::transfer_failed("DMA buffer size must be > 0"));
        }

        let layout = std::alloc::Layout::from_size_align(size, 4096)
            .map_err(|e| AkidaError::transfer_failed(format!("Invalid DMA buffer layout: {e}")))?;

        // SAFETY: Page-aligned allocation for DMA. Layout validated above (size>0, align 4096).
        // Dealloc'd in Drop with same layout. Returns null on OOM (checked below).
        let vaddr = unsafe { std::alloc::alloc_zeroed(layout) };
        if vaddr.is_null() {
            return Err(AkidaError::transfer_failed("Failed to allocate DMA buffer"));
        }

        // SAFETY: mlock prevents page-out, required by VFIO DMA. vaddr valid for `size` bytes.
        if let Err(e) = unsafe { mlock(vaddr.cast(), size) } {
            unsafe { std::alloc::dealloc(vaddr, layout) };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to lock DMA memory: {e}"
            )));
        }

        #[allow(clippy::cast_possible_truncation)]
        let dma_map_arg = VfioDmaMap {
            argsz: std::mem::size_of::<VfioDmaMap>() as u32,
            flags: ioctls::VFIO_DMA_MAP_FLAG_READ | ioctls::VFIO_DMA_MAP_FLAG_WRITE,
            vaddr: vaddr as u64,
            iova,
            size: size as u64,
        };

        tracing::debug!(
            "DMA map attempt: vaddr={:#x}, iova={:#x}, size={:#x}, flags={:#x}",
            dma_map_arg.vaddr,
            dma_map_arg.iova,
            dma_map_arg.size,
            dma_map_arg.flags
        );

        // SAFETY: container_fd is valid from VFIO container open; borrow_raw requires valid fd.
        // Caller guarantees: container_fd is an open VFIO container fd, valid for this call.
        let container_borrowed = unsafe { BorrowedFd::borrow_raw(container_fd) };
        if let Err(e) = ioctl::dma_map(container_borrowed, &dma_map_arg) {
            tracing::warn!("DMA map failed: {e}");
            // SAFETY: Cleanup on failure — vaddr was allocated and mlock'd successfully above.
            unsafe {
                let _ = munlock(vaddr.cast(), size);
                std::alloc::dealloc(vaddr, layout);
            };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to map DMA: {e}"
            )));
        }

        tracing::debug!("Created DMA buffer: vaddr={vaddr:p}, iova={iova:#x}, size={size:#x}");

        Ok(Self {
            vaddr,
            iova,
            size,
            container_fd,
        })
    }

    /// Immutable slice view of the buffer contents.
    pub fn as_slice(&self) -> &[u8] {
        debug_assert!(!self.vaddr.is_null(), "DmaBuffer vaddr is null");
        debug_assert!(self.size > 0, "DmaBuffer size is 0");
        // SAFETY: vaddr from alloc in new(), valid for size; &self prevents concurrent mutation.
        unsafe { std::slice::from_raw_parts(self.vaddr, self.size) }
    }

    /// Mutable slice view for writing data into the buffer.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(!self.vaddr.is_null(), "DmaBuffer vaddr is null");
        debug_assert!(self.size > 0, "DmaBuffer size is 0");
        // SAFETY: vaddr valid for size; &mut self guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.vaddr, self.size) }
    }

    /// Device-visible I/O virtual address.
    pub const fn iova(&self) -> u64 {
        self.iova
    }

    /// Buffer size in bytes.
    pub const fn size(&self) -> usize {
        self.size
    }
}

impl Drop for DmaBuffer {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "struct sizes always fit u32"
    )]
    fn drop(&mut self) {
        // SAFETY: munlock matches mlock from new(); must unlock before dealloc.
        unsafe {
            let _ = munlock(self.vaddr.cast(), self.size);
        };

        let dma_unmap = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: self.iova,
            size: self.size as u64,
        };

        // SAFETY: container_fd still valid — DmaBuffer is dropped before the VFIO container
        // (Drop order: DmaBuffer fields, then parent VfioBackend). borrow_raw requires valid fd.
        let container_borrowed = unsafe { BorrowedFd::borrow_raw(self.container_fd) };
        let _ = ioctl::dma_unmap(container_borrowed, &dma_unmap);

        let layout = std::alloc::Layout::from_size_align(self.size, 4096)
            .expect("Layout valid: matches alloc in new()");
        // SAFETY: dealloc matches alloc_zeroed from new(); layout identical; no outstanding refs.
        unsafe { std::alloc::dealloc(self.vaddr, layout) };

        tracing::debug!("Freed DMA buffer at iova={:#x}", self.iova);
    }
}

// SAFETY: DmaBuffer owns its allocation exclusively — no shared mutable state.
unsafe impl Send for DmaBuffer {}

// SAFETY: Reads via &self are safe from multiple threads; writes require &mut self.
unsafe impl Sync for DmaBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_buffer_new_size_zero() {
        let result = DmaBuffer::new(-1, 0, 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size must be > 0"));
    }

    #[test]
    fn test_dma_buffer_iova_size_accessors() {
        // We can't create a real DmaBuffer without VFIO, but we can test the size zero path
        let result = DmaBuffer::new(0, 0, 0x1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_dma_buffer_layout_alignment_4096() {
        let layout = std::alloc::Layout::from_size_align(4096, 4096);
        assert!(layout.is_ok());
        let layout = layout.unwrap();
        assert_eq!(layout.size(), 4096);
        assert_eq!(layout.align(), 4096);
    }

    #[test]
    fn test_dma_buffer_layout_invalid_align_zero() {
        let layout = std::alloc::Layout::from_size_align(4096, 0);
        assert!(layout.is_err());
    }

    #[test]
    fn test_dma_buffer_layout_invalid_align_non_power_of_two() {
        let layout = std::alloc::Layout::from_size_align(4096, 3000);
        assert!(layout.is_err());
    }

    #[test]
    fn test_dma_buffer_alignment_math_page_aligned() {
        let size = 1usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 4096);
    }

    #[test]
    fn test_dma_buffer_alignment_math_exact_page() {
        let size = 8192usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 8192);
    }

    #[test]
    fn test_dma_buffer_alignment_math_multiple_pages() {
        let size = 16_384usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 16_384);
    }

    #[test]
    fn test_dma_buffer_vfio_dma_map_argsz_layout() {
        let argsz = std::mem::size_of::<super::super::types::VfioDmaMap>();
        assert!(
            argsz >= 32,
            "VfioDmaMap kernel ABI expects at least 32 bytes"
        );
    }

    #[test]
    fn test_dma_buffer_vfio_dma_unmap_argsz_layout() {
        let argsz = std::mem::size_of::<super::super::types::VfioDmaUnmap>();
        assert!(
            argsz >= 24,
            "VfioDmaUnmap kernel ABI expects at least 24 bytes"
        );
    }
}
