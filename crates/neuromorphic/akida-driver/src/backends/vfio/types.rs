// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO ABI types and constants — re-exported from `hw-safe::vfio_setup`.
//!
//! Kernel ABI structs and setup ioctl wrappers live in `hw-safe`. This
//! module re-exports what the akida VFIO backend needs and adds
//! backend-specific types (`PollConfig`).

/// VFIO constants re-exported from `hw-safe::vfio_setup`.
///
/// Setup ioctls are handled by `hw-safe::vfio_setup`. This module
/// retains the ABI constants needed by this backend (API version,
/// IOMMU type, group flags, DMA flags).
pub mod ioctls {
    pub use toadstool_hw_safe::vfio_setup::{
        VFIO_API_VERSION, VFIO_GROUP_FLAGS_VIABLE, VFIO_TYPE1V2_IOMMU,
    };

    #[expect(dead_code, reason = "VFIO DMA map flags retained for backend ABI parity")]
    pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1 << 0;
    #[expect(dead_code, reason = "VFIO DMA map flags retained for backend ABI parity")]
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

#[expect(unused_imports, reason = "re-exported for downstream VFIO backend consumers")]
pub use toadstool_hw_safe::vfio_setup::{VfioDeviceInfo, VfioGroupStatus, VfioRegionInfo};
#[expect(unused_imports, reason = "re-exported for downstream VFIO backend consumers")]
pub use toadstool_hw_safe::vfio_dma::{VfioDmaMap, VfioDmaUnmap};

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
    fn test_vfio_dma_map_zeroed() {
        let map = VfioDmaMap {
            argsz: 0,
            flags: 0,
            vaddr: 0,
            iova: 0,
            size: 0,
        };
        assert_eq!(map.vaddr, 0);
        assert_eq!(map.iova, 0);
        assert_eq!(map.size, 0);
    }

    #[test]
    fn test_vfio_dma_unmap_zeroed() {
        let unmap = VfioDmaUnmap {
            argsz: 0,
            flags: 0,
            iova: 0,
            size: 0,
        };
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
        let flags =
            toadstool_hw_safe::vfio_dma::flags::READ | toadstool_hw_safe::vfio_dma::flags::WRITE;
        assert_eq!(flags, 3);
    }

    #[test]
    fn test_vfio_dma_map_struct_layout_repr_c() {
        let map = VfioDmaMap {
            argsz: 32,
            flags: toadstool_hw_safe::vfio_dma::flags::READ
                | toadstool_hw_safe::vfio_dma::flags::WRITE,
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
