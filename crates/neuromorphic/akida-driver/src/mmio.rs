//! Memory-Mapped I/O for Akida NPU
//!
//! Provides safe abstractions for accessing Akida hardware registers.
//! Based on VFIO region mapping.

use crate::error::{AkidaError, Result};
use std::fs::File;
use std::os::unix::io::AsRawFd;

/// AKD1000 BAR regions
#[derive(Debug, Clone, Copy)]
pub enum Bar {
    /// Control/status registers (BAR0)
    Control = 0,
    /// Model memory (BAR1)
    Model = 1,
    /// Data buffers (BAR2)
    Data = 2,
}

/// AKD1000 register offsets (inferred from behavior)
pub mod regs {
    /// Device identification register
    pub const DEVICE_ID: usize = 0x0000;
    /// Device version register
    pub const VERSION: usize = 0x0004;
    /// Device status register
    pub const STATUS: usize = 0x0008;
    /// Control register
    pub const CONTROL: usize = 0x000C;
    /// NPU count register
    pub const NPU_COUNT: usize = 0x0010;
    /// SRAM size register (in KB)
    pub const SRAM_SIZE: usize = 0x0014;
    /// Interrupt status
    pub const IRQ_STATUS: usize = 0x0020;
    /// Interrupt enable
    pub const IRQ_ENABLE: usize = 0x0024;
    /// Model load address
    pub const MODEL_ADDR_LO: usize = 0x0100;
    /// Model load address high
    pub const MODEL_ADDR_HI: usize = 0x0104;
    /// Model size
    pub const MODEL_SIZE: usize = 0x0108;
    /// Model load trigger
    pub const MODEL_LOAD: usize = 0x010C;
    /// Input buffer address
    pub const INPUT_ADDR_LO: usize = 0x0200;
    /// Input buffer address high
    pub const INPUT_ADDR_HI: usize = 0x0204;
    /// Input size
    pub const INPUT_SIZE: usize = 0x0208;
    /// Output buffer address
    pub const OUTPUT_ADDR_LO: usize = 0x0300;
    /// Output buffer address high
    pub const OUTPUT_ADDR_HI: usize = 0x0304;
    /// Output size
    pub const OUTPUT_SIZE: usize = 0x0308;
    /// Inference trigger
    pub const INFER_START: usize = 0x0400;
    /// Inference status
    pub const INFER_STATUS: usize = 0x0404;

    /// Status bits
    pub mod status {
        pub const READY: u32 = 1 << 0;
        pub const BUSY: u32 = 1 << 1;
        pub const ERROR: u32 = 1 << 2;
        pub const MODEL_LOADED: u32 = 1 << 3;
    }

    /// Control bits
    pub mod control {
        pub const RESET: u32 = 1 << 0;
        pub const ENABLE: u32 = 1 << 1;
        pub const POWER_SAVE: u32 = 1 << 2;
    }
}

/// VFIO region info structure
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioRegionInfo {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

/// Mapped BAR region for MMIO access
pub struct MappedRegion {
    /// Memory-mapped pointer
    ptr: *mut u8,
    /// Size of the mapping
    size: usize,
    /// BAR index
    bar: Bar,
}

// SAFETY: MappedRegion owns exclusive access to the mapped memory
unsafe impl Send for MappedRegion {}
unsafe impl Sync for MappedRegion {}

impl MappedRegion {
    /// Map a BAR region via VFIO
    pub fn map(device_fd: &File, bar: Bar) -> Result<Self> {
        // Query region info
        #[allow(clippy::cast_possible_truncation)]
        let mut region_info = VfioRegionInfo {
            argsz: std::mem::size_of::<VfioRegionInfo>() as u32,
            index: bar as u32,
            ..Default::default()
        };

        // VFIO_DEVICE_GET_REGION_INFO = _IOWR(';', 100 + 8, ...)
        const VFIO_DEVICE_GET_REGION_INFO: libc::c_ulong = 0xc018_3b68;

        // SAFETY: VFIO ioctl with valid structure
        let ret = unsafe {
            libc::ioctl(
                device_fd.as_raw_fd(),
                VFIO_DEVICE_GET_REGION_INFO,
                &mut region_info as *mut _,
            )
        };

        if ret < 0 {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to get BAR{} info: {}",
                bar as u32,
                std::io::Error::last_os_error()
            )));
        }

        tracing::debug!(
            "BAR{}: size={:#x}, offset={:#x}, flags={:#x}",
            bar as u32,
            region_info.size,
            region_info.offset,
            region_info.flags
        );

        // Map the region
        // SAFETY: We have exclusive access via VFIO device fd
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                region_info.size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                device_fd.as_raw_fd(),
                region_info.offset as i64,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to mmap BAR{}: {}",
                bar as u32,
                std::io::Error::last_os_error()
            )));
        }

        tracing::info!("Mapped BAR{} at {:p}, size={:#x}", bar as u32, ptr, region_info.size);

        Ok(Self {
            ptr: ptr.cast(),
            size: region_info.size as usize,
            bar,
        })
    }

    /// Read a 32-bit register
    pub fn read32(&self, offset: usize) -> u32 {
        assert!(offset + 4 <= self.size, "Register offset out of bounds");
        // SAFETY: Offset is within bounds, ptr is valid
        unsafe {
            std::ptr::read_volatile(self.ptr.add(offset).cast::<u32>())
        }
    }

    /// Write a 32-bit register
    pub fn write32(&self, offset: usize, value: u32) {
        assert!(offset + 4 <= self.size, "Register offset out of bounds");
        // SAFETY: Offset is within bounds, ptr is valid
        unsafe {
            std::ptr::write_volatile(self.ptr.add(offset).cast::<u32>(), value);
        }
    }

    /// Read a 64-bit register
    pub fn read64(&self, offset: usize) -> u64 {
        assert!(offset + 8 <= self.size, "Register offset out of bounds");
        // SAFETY: Offset is within bounds, ptr is valid
        unsafe {
            std::ptr::read_volatile(self.ptr.add(offset).cast::<u64>())
        }
    }

    /// Write a 64-bit register
    pub fn write64(&self, offset: usize, value: u64) {
        assert!(offset + 8 <= self.size, "Register offset out of bounds");
        // SAFETY: Offset is within bounds, ptr is valid
        unsafe {
            std::ptr::write_volatile(self.ptr.add(offset).cast::<u64>(), value);
        }
    }

    /// Get BAR type
    pub fn bar(&self) -> Bar {
        self.bar
    }

    /// Get region size
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for MappedRegion {
    fn drop(&mut self) {
        // SAFETY: ptr was created by mmap with this size
        unsafe {
            libc::munmap(self.ptr.cast(), self.size);
        }
        tracing::debug!("Unmapped BAR{}", self.bar as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_offsets() {
        // Sanity check register layout
        assert_eq!(regs::DEVICE_ID, 0x0000);
        assert_eq!(regs::INFER_START, 0x0400);
        assert!(regs::status::READY != 0);
    }
}
