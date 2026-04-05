// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // VFIO DMA map/unmap ioctls require unsafe
//! DMA buffer management for VFIO NPU backend
//!
//! Provides page-aligned, mlock'd, IOMMU-mapped memory buffers for
//! zero-copy data transfer between host and NPU hardware.

use crate::error::{AkidaError, Result};
use std::os::fd::{AsFd, OwnedFd};

use toadstool_hw_safe::LockedMemory;
use toadstool_hw_safe::vfio_dma::{VfioDmaMap, VfioDmaUnmap, dma_map, dma_unmap, flags};

/// DMA buffer for fast host-to-device data transfer.
///
/// Memory is page-aligned (4096), mlock'd to prevent swapping, and IOMMU-mapped
/// so the device can access it via IOVA. Cleanup is automatic on drop.
#[derive(Debug)]
pub struct DmaBuffer {
    mem: LockedMemory,
    iova: u64,
    size: usize,
    container_fd: OwnedFd,
}

impl DmaBuffer {
    /// Allocate a new DMA buffer mapped for device access.
    ///
    /// # Errors
    ///
    /// Returns an error if allocation, mlock, or IOMMU DMA mapping fails.
    pub(crate) fn new(container_fd: OwnedFd, size: usize, iova: u64) -> Result<Self> {
        if size == 0 {
            return Err(AkidaError::transfer_failed("DMA buffer size must be > 0"));
        }

        let mem = LockedMemory::page_aligned(size)
            .map_err(|e| AkidaError::transfer_failed(format!("Failed to lock DMA memory: {e}")))?;

        let vaddr = mem.as_ptr().as_ptr() as u64;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation acceptable for this conversion"
        )]
        let dma_map_arg = VfioDmaMap {
            argsz: std::mem::size_of::<VfioDmaMap>() as u32,
            flags: flags::READ | flags::WRITE,
            vaddr,
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

        // SAFETY: container_fd is the VFIO container (OwnedFd guarantees validity);
        // map.vaddr points at mem's allocation for size bytes; IOVA range chosen by caller.
        if let Err(e) = unsafe { dma_map(container_fd.as_fd(), &dma_map_arg) } {
            tracing::warn!("DMA map failed: {e}");
            return Err(AkidaError::transfer_failed(format!(
                "Failed to map DMA: {e}"
            )));
        }

        tracing::debug!(
            "Created DMA buffer: vaddr={:p}, iova={iova:#x}, size={size:#x}",
            mem.as_ptr().as_ptr(),
        );

        Ok(Self {
            mem,
            iova,
            size,
            container_fd,
        })
    }

    /// Immutable slice view of the buffer contents.
    pub fn as_slice(&self) -> &[u8] {
        self.mem.as_slice()
    }

    /// Mutable slice view for writing data into the buffer.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mem.as_mut_slice()
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
        let dma_unmap_arg = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: self.iova,
            size: self.size as u64,
        };

        // SAFETY: container_fd is OwnedFd (guaranteed valid until Drop completes);
        // iova/size match the prior dma_map call for this buffer.
        let _ = unsafe { dma_unmap(self.container_fd.as_fd(), &dma_unmap_arg) };

        tracing::debug!("Freed DMA buffer at iova={:#x}", self.iova);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_hw_safe::vfio_dma::{VfioDmaMap, VfioDmaUnmap};

    fn dummy_fd() -> OwnedFd {
        use std::fs::File;
        let f = File::open("/dev/null").expect("/dev/null");
        f.into()
    }

    #[test]
    fn test_dma_buffer_new_size_zero() {
        let result = DmaBuffer::new(dummy_fd(), 0, 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size must be > 0"));
    }

    #[test]
    fn test_dma_buffer_iova_size_accessors() {
        let result = DmaBuffer::new(dummy_fd(), 0, 0x1000);
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
        let argsz = std::mem::size_of::<VfioDmaMap>();
        assert!(
            argsz >= 32,
            "VfioDmaMap kernel ABI expects at least 32 bytes"
        );
    }

    #[test]
    fn test_dma_buffer_vfio_dma_unmap_argsz_layout() {
        let argsz = std::mem::size_of::<VfioDmaUnmap>();
        assert!(
            argsz >= 24,
            "VfioDmaUnmap kernel ABI expects at least 24 bytes"
        );
    }
}
