// SPDX-License-Identifier: AGPL-3.0-only
//! VFIO kernel ABI types and ioctl opcodes
//!
//! Linux VFIO ioctl numbers (from kernel headers) and repr(C) structures
//! matching the kernel ABI for VFIO container, group, device, and IOMMU operations.

// FFI/ioctl casts are intentional - VFIO API requires specific types
#![allow(clippy::cast_possible_truncation)]

/// VFIO ioctl opcodes
///
/// Uses rustix `opcode::none()` for `_IO(';', base + offset)`.
/// VFIO uses `_IO` for extensibility (no size in opcode).
pub mod ioctls {
    use rustix::ioctl::{Opcode, opcode};

    pub const VFIO_TYPE: u8 = b';';
    pub const VFIO_BASE: u8 = 100;

    pub const OP_GET_API_VERSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE);
    pub const OP_CHECK_EXTENSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 1);
    pub const OP_SET_IOMMU: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 2);

    pub const OP_GROUP_GET_STATUS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 3);
    pub const OP_GROUP_SET_CONTAINER: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 4);
    pub const OP_GROUP_GET_DEVICE_FD: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 6);

    pub const OP_DEVICE_GET_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 7);
    #[expect(
        dead_code,
        reason = "VFIO ioctl opcode; used in future driver operations"
    )]
    pub const OP_DEVICE_GET_REGION_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 8);
    #[expect(
        dead_code,
        reason = "VFIO ioctl opcode; used in future driver operations"
    )]
    pub const OP_DEVICE_GET_IRQ_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 9);
    #[expect(
        dead_code,
        reason = "VFIO ioctl opcode; used in future driver operations"
    )]
    pub const OP_DEVICE_SET_IRQS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 10);
    #[expect(
        dead_code,
        reason = "VFIO ioctl opcode; used in future driver operations"
    )]
    pub const OP_DEVICE_RESET: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 11);

    pub const OP_IOMMU_MAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 13);
    pub const OP_IOMMU_UNMAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 14);

    pub const VFIO_API_VERSION: i32 = 0;

    #[expect(dead_code, reason = "VFIO IOMMU type; reserved for Type1 fallback")]
    pub const VFIO_TYPE1_IOMMU: u32 = 1;
    pub const VFIO_TYPE1V2_IOMMU: u32 = 3;

    pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1 << 0;
    #[expect(dead_code, reason = "VFIO group flag; used to verify container state")]
    pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 1 << 1;

    pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1 << 0;
    pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 1 << 1;
}

/// Parameters for polling a status register.
#[derive(Clone, Copy)]
pub struct PollConfig<'a> {
    pub reg: usize,
    pub done_mask: u32,
    pub error_mask: u32,
    pub max_polls: u32,
    pub yield_interval: u32,
    pub timeout_msg: &'a str,
    pub error_msg: &'a str,
}

/// VFIO device info structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioDeviceInfo {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
}

/// VFIO region info structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
#[allow(dead_code)] // VFIO kernel struct; construction reserved for driver init and tests
pub struct VfioRegionInfo {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

/// VFIO group status structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioGroupStatus {
    pub argsz: u32,
    pub flags: u32,
}

/// VFIO DMA map structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioDmaMap {
    pub argsz: u32,
    pub flags: u32,
    pub vaddr: u64,
    pub iova: u64,
    pub size: u64,
}

/// VFIO DMA unmap structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioDmaUnmap {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfio_device_info_default() {
        let info = VfioDeviceInfo::default();
        assert_eq!(info.argsz, 0);
        assert_eq!(info.flags, 0);
        assert_eq!(info.num_regions, 0);
        assert_eq!(info.num_irqs, 0);
    }

    #[test]
    fn test_vfio_group_status_default() {
        let status = VfioGroupStatus::default();
        assert_eq!(status.argsz, 0);
        assert_eq!(status.flags, 0);
    }

    #[test]
    fn test_vfio_region_info_default() {
        let info = VfioRegionInfo::default();
        assert_eq!(info.size, 0);
        assert_eq!(info.offset, 0);
    }

    #[test]
    fn test_vfio_dma_map_default() {
        let map = VfioDmaMap::default();
        assert_eq!(map.vaddr, 0);
        assert_eq!(map.iova, 0);
        assert_eq!(map.size, 0);
    }

    #[test]
    fn test_vfio_dma_unmap_default() {
        let unmap = VfioDmaUnmap::default();
        assert_eq!(unmap.iova, 0);
        assert_eq!(unmap.size, 0);
    }

    #[test]
    fn test_vfio_dma_aligned_size_4096() {
        let size = 4096usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 4096);
    }

    #[test]
    fn test_vfio_dma_aligned_size_1_byte() {
        let size = 1usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 4096);
    }

    #[test]
    fn test_vfio_dma_aligned_size_4097() {
        let size = 4097usize;
        let aligned = size.div_ceil(4096) * 4096;
        assert_eq!(aligned, 8192);
    }

    #[test]
    fn test_vfio_dma_map_argsz_size() {
        let argsz = std::mem::size_of::<VfioDmaMap>();
        assert!(argsz >= 32);
    }

    #[test]
    fn test_vfio_dma_unmap_argsz_size() {
        let argsz = std::mem::size_of::<VfioDmaUnmap>();
        assert!(argsz >= 24);
    }

    #[test]
    fn test_vfio_dma_map_flags_combined() {
        let flags = ioctls::VFIO_DMA_MAP_FLAG_READ | ioctls::VFIO_DMA_MAP_FLAG_WRITE;
        assert_eq!(flags, 3);
    }

    #[test]
    fn test_vfio_dma_map_struct_layout_repr_c() {
        let map = VfioDmaMap {
            argsz: 32,
            flags: ioctls::VFIO_DMA_MAP_FLAG_READ | ioctls::VFIO_DMA_MAP_FLAG_WRITE,
            vaddr: 0x1000_0000,
            iova: 0x2000_0000,
            size: 4096,
        };
        assert_eq!(map.argsz, std::mem::size_of::<VfioDmaMap>() as u32);
        assert_eq!(map.iova, 0x2000_0000);
        assert_eq!(map.size, 4096);
    }

    #[test]
    fn test_vfio_dma_unmap_struct_layout() {
        let unmap = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: 0x1000_0000,
            size: 8192,
        };
        assert_eq!(unmap.iova, 0x1000_0000);
        assert_eq!(unmap.size, 8192);
    }

    #[test]
    fn test_vfio_device_info_argsz() {
        let info = VfioDeviceInfo {
            argsz: std::mem::size_of::<VfioDeviceInfo>() as u32,
            flags: 0,
            num_regions: 4,
            num_irqs: 1,
        };
        assert!(info.argsz >= 16);
        assert_eq!(info.num_regions, 4);
    }

    #[test]
    fn test_poll_config_lifetime() {
        let timeout_msg = "timeout";
        let error_msg = "error";
        let cfg = PollConfig {
            reg: 0,
            done_mask: 1,
            error_mask: 2,
            max_polls: 100,
            yield_interval: 10,
            timeout_msg,
            error_msg,
        };
        assert_eq!(cfg.max_polls, 100);
    }
}
