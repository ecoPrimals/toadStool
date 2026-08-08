// SPDX-License-Identifier: AGPL-3.0-or-later

//! VFIO ioctl wrappers — delegates to `toadstool-hw-safe` for all kernel calls.
//!
//! This module re-exports types and constants from hw-safe and provides
//! crate-local error-wrapping adapters. No direct `libc::ioctl` calls remain.

#![allow(
    unsafe_code,
    reason = "BorrowedFd::borrow_raw and DMA map require unsafe"
)]
#![allow(clippy::redundant_pub_crate)]

use std::ffi::CStr;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::io::RawFd;

use crate::error::{AkidaError, Result};

pub use toadstool_hw_safe::vfio_dma::{VfioDmaMap, VfioDmaUnmap, flags as dma_flags};
pub use toadstool_hw_safe::vfio_setup::{
    VFIO_API_VERSION, VFIO_GROUP_FLAGS_VIABLE, VFIO_TYPE1V2_IOMMU, VfioDeviceInfo, VfioGroupStatus,
};

pub const VFIO_DMA_MAP_FLAG_READ: u32 = dma_flags::READ;
pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = dma_flags::WRITE;

const fn borrow(raw: RawFd) -> BorrowedFd<'static> {
    // SAFETY: callers guarantee the raw fd is valid for the duration of the
    // ioctl call. The 'static lifetime is sound because the BorrowedFd is
    // not stored — it's consumed immediately by the ioctl wrapper.
    unsafe { BorrowedFd::borrow_raw(raw) }
}

pub(crate) fn ioctl_vfio_get_api_version(container_fd: RawFd) -> i32 {
    toadstool_hw_safe::vfio_setup::get_api_version(borrow(container_fd)).unwrap_or(-1)
}

pub(crate) fn ioctl_vfio_check_type1v2(container_fd: RawFd) -> Result<()> {
    let has_type1 =
        toadstool_hw_safe::vfio_setup::check_extension(borrow(container_fd), VFIO_TYPE1V2_IOMMU)
            .unwrap_or(0);
    if has_type1 != 1 {
        return Err(AkidaError::capability_query_failed(
            "VFIO Type1v2 IOMMU not supported",
        ));
    }
    Ok(())
}

pub(crate) fn ioctl_vfio_group_get_status(group_fd: RawFd) -> Result<VfioGroupStatus> {
    let status =
        toadstool_hw_safe::vfio_setup::group_get_status(borrow(group_fd)).map_err(|e| {
            AkidaError::capability_query_failed(format!("Failed to get group status: {e}"))
        })?;
    if (status.flags & VFIO_GROUP_FLAGS_VIABLE) == 0 {
        return Err(AkidaError::capability_query_failed(
            "VFIO group not viable (all devices must be bound to vfio-pci)",
        ));
    }
    Ok(status)
}

pub(crate) fn ioctl_vfio_group_set_container(group_fd: RawFd, container_fd: RawFd) -> Result<()> {
    toadstool_hw_safe::vfio_setup::group_set_container(borrow(group_fd), borrow(container_fd))
        .map_err(|e| AkidaError::capability_query_failed(format!("Failed to set container: {e}")))
}

pub(crate) fn ioctl_vfio_set_iommu(container_fd: RawFd) -> Result<()> {
    toadstool_hw_safe::vfio_setup::set_iommu(borrow(container_fd), VFIO_TYPE1V2_IOMMU)
        .map_err(|e| AkidaError::capability_query_failed(format!("Failed to set IOMMU: {e}")))
}

pub(crate) fn ioctl_vfio_group_get_device_fd(
    group_fd: RawFd,
    pcie_address: &CStr,
) -> Result<OwnedFd> {
    toadstool_hw_safe::vfio_setup::group_get_device_fd(borrow(group_fd), pcie_address)
        .map_err(|e| AkidaError::capability_query_failed(format!("Failed to get device fd: {e}")))
}

pub(crate) fn ioctl_vfio_device_get_info(device_fd: impl AsFd) -> Result<VfioDeviceInfo> {
    toadstool_hw_safe::vfio_setup::device_get_info(device_fd.as_fd())
        .map_err(|e| AkidaError::capability_query_failed(format!("Failed to get device info: {e}")))
}

/// Map a user buffer to IOVA via `VFIO_IOMMU_MAP_DMA`.
pub(crate) fn ioctl_vfio_iommu_map_dma(container_fd: RawFd, map: &VfioDmaMap) -> Result<()> {
    // SAFETY: caller guarantees vaddr is valid for map.size bytes, iova is free.
    unsafe { toadstool_hw_safe::vfio_dma::dma_map(borrow(container_fd), map) }
        .map_err(|e| AkidaError::transfer_failed(format!("Failed to map DMA: {e}")))
}

/// Unmap IOVA via `VFIO_IOMMU_UNMAP_DMA` (best-effort; used from `Drop`).
pub(crate) fn ioctl_vfio_iommu_unmap_dma(container_fd: RawFd, unmap: &VfioDmaUnmap) {
    // SAFETY: best-effort unmap — iova/size from a prior successful map.
    let _ = unsafe { toadstool_hw_safe::vfio_dma::dma_unmap(borrow(container_fd), unmap) };
}

/// Issue `VFIO_DEVICE_RESET` to reset the device through the VFIO subsystem.
pub(crate) fn ioctl_vfio_device_reset(device_fd: impl AsFd) -> Result<()> {
    toadstool_hw_safe::vfio_setup::device_reset(device_fd.as_fd())
        .map_err(|e| AkidaError::hardware_error(format!("VFIO device reset failed: {e}")))
}
