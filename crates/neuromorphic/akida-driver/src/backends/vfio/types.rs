// SPDX-License-Identifier: AGPL-3.0-or-later
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
pub(crate) mod ioctls {
    use rustix::ioctl::{opcode, Opcode};

    pub const VFIO_TYPE: u8 = b';';
    pub const VFIO_BASE: u8 = 100;

    pub const OP_GET_API_VERSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE);
    pub const OP_CHECK_EXTENSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 1);
    pub const OP_SET_IOMMU: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 2);

    pub const OP_GROUP_GET_STATUS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 3);
    pub const OP_GROUP_SET_CONTAINER: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 4);
    pub const OP_GROUP_GET_DEVICE_FD: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 6);

    pub const OP_DEVICE_GET_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 7);
    #[allow(dead_code)]
    pub const OP_DEVICE_GET_REGION_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 8);
    #[allow(dead_code)]
    pub const OP_DEVICE_GET_IRQ_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 9);
    #[allow(dead_code)]
    pub const OP_DEVICE_SET_IRQS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 10);
    #[allow(dead_code)]
    pub const OP_DEVICE_RESET: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 11);

    pub const OP_IOMMU_MAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 13);
    pub const OP_IOMMU_UNMAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 14);

    pub const VFIO_API_VERSION: i32 = 0;

    #[allow(dead_code)]
    pub const VFIO_TYPE1_IOMMU: u32 = 1;
    pub const VFIO_TYPE1V2_IOMMU: u32 = 3;

    pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1 << 0;
    #[allow(dead_code)]
    pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 1 << 1;

    pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1 << 0;
    pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 1 << 1;
}

/// Parameters for polling a status register.
#[derive(Clone, Copy)]
pub(crate) struct PollConfig<'a> {
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
pub(crate) struct VfioDeviceInfo {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
}

/// VFIO region info structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct VfioRegionInfo {
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
pub(crate) struct VfioGroupStatus {
    pub argsz: u32,
    pub flags: u32,
}

/// VFIO DMA map structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct VfioDmaMap {
    pub argsz: u32,
    pub flags: u32,
    pub vaddr: u64,
    pub iova: u64,
    pub size: u64,
}

/// VFIO DMA unmap structure (kernel ABI)
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct VfioDmaUnmap {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub size: u64,
}
