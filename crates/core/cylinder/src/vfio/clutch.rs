// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clutch — engage/disengage GPU operational state without releasing the anchor fd.
//!
//! The Clutch sits between the VfioAnchor (which holds VFIO fds to prevent bus
//! reset) and the sovereign pipeline (which needs BAR0 + DMA to do work). It
//! allows the daemon to "engage" a GPU (create BAR0 mmap and DMA backend from
//! the anchor's fds) for an operation, then "disengage" (release mappings) while
//! the anchor keeps the GPU warm.
//!
//! ```text
//! VfioAnchor (fd held, never dropped)
//!     └── Clutch::engage(fd, backend_info)
//!             └── ClutchEngaged { bar0, dma_backend }
//!                     ├── sovereign.init uses bar0 + dma
//!                     └── .disengage() drops bar0/dma, anchor stays
//! ```

use std::borrow::Cow;
use std::os::fd::BorrowedFd;
use std::sync::Arc;

use crate::error::DriverError;
use crate::mmio_region::MmioRegion;
use crate::vfio::device::MappedBar;
use crate::vfio::DmaBackend;
use crate::vfio::ioctl;
use crate::vfio::types::VfioRegionInfo;

/// Active GPU engagement — holds BAR0 mmap and DMA backend.
///
/// Created by [`Clutch::engage`], dropped by [`disengage`](ClutchEngaged::disengage)
/// or normal Drop. Dropping unmaps BAR0 and releases the DMA backend Arc, but
/// does NOT close the VFIO device fd (the anchor holds that).
pub struct ClutchEngaged {
    bar0: MappedBar,
    dma_backend: DmaBackend,
    bdf: String,
}

impl ClutchEngaged {
    /// Borrow the BAR0 mapping for register access.
    #[must_use]
    pub fn bar0(&self) -> &MappedBar {
        &self.bar0
    }

    /// Borrow the DMA backend for buffer allocation.
    #[must_use]
    pub fn dma_backend(&self) -> &DmaBackend {
        &self.dma_backend
    }

    /// Clone the DMA backend (Arc clone — cheap).
    #[must_use]
    pub fn dma_backend_clone(&self) -> DmaBackend {
        self.dma_backend.clone()
    }

    /// BDF address of the engaged device.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Explicitly disengage: release BAR0 and DMA mappings.
    /// The anchor's fd remains open.
    pub fn disengage(self) {
        tracing::info!(bdf = %self.bdf, "Clutch disengaged — BAR0 unmapped, DMA released");
    }
}

/// Clutch operations — stateless functions to engage/disengage GPUs.
pub struct Clutch;

impl Clutch {
    /// Engage a GPU: create BAR0 mmap and DMA backend from anchor fds.
    ///
    /// The `device_fd` should be borrowed from a VfioAnchor. The BAR0 mmap
    /// and DMA backend are created without taking ownership of the fd.
    ///
    /// # Arguments
    ///
    /// * `bdf` — PCI BDF address for logging
    /// * `device_fd` — borrowed VFIO device fd from the anchor
    /// * `dma_backend` — DMA backend constructed from anchor's backend fds
    ///
    /// # Errors
    ///
    /// Returns error if BAR0 region info ioctl fails or mmap fails.
    #[expect(clippy::cast_possible_truncation, reason = "struct argsz always fits u32")]
    pub fn engage(
        bdf: &str,
        device_fd: BorrowedFd<'_>,
        dma_backend: DmaBackend,
    ) -> Result<ClutchEngaged, DriverError> {
        let bar0 = map_bar_from_fd(bdf, device_fd, 0)?;
        tracing::info!(
            bdf,
            bar0_size = format_args!("{:#x}", bar0.size()),
            "Clutch engaged — BAR0 mapped, DMA ready"
        );
        Ok(ClutchEngaged {
            bar0,
            dma_backend,
            bdf: bdf.to_string(),
        })
    }

    /// Engage using sysfs BAR0 (fallback when VFIO device fd doesn't
    /// support region info ioctl, e.g. for received/reconstructed fds).
    ///
    /// # Errors
    ///
    /// Returns error if sysfs BAR0 open or mmap fails.
    pub fn engage_sysfs(
        bdf: &str,
        dma_backend: DmaBackend,
    ) -> Result<ClutchEngaged, DriverError> {
        let bar0 = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024)?;
        tracing::info!(
            bdf,
            bar0_size = format_args!("{:#x}", bar0.size()),
            "Clutch engaged via sysfs — BAR0 mapped, DMA ready"
        );
        Ok(ClutchEngaged {
            bar0,
            dma_backend,
            bdf: bdf.to_string(),
        })
    }

    /// Construct a `DmaBackend` from an anchor's backend reference.
    ///
    /// This bridges the ember crate's `AnchorBackendRef` to cylinder's `DmaBackend`
    /// without requiring ember as a dependency — the server crate calls this with
    /// the raw components extracted from `AnchorBackendRef`.
    #[must_use]
    pub fn dma_backend_from_iommufd(iommufd: Arc<std::os::fd::OwnedFd>, ioas_id: u32) -> DmaBackend {
        DmaBackend::Iommufd { fd: iommufd, ioas_id }
    }

    /// Construct a `DmaBackend` for legacy container backend.
    #[must_use]
    pub fn dma_backend_from_legacy(container: Arc<std::os::fd::OwnedFd>) -> DmaBackend {
        DmaBackend::LegacyContainer(container)
    }
}

/// Map a BAR region from a raw VFIO device fd.
///
/// Replicates `VfioDevice::map_bar` but works with a borrowed fd, so the
/// anchor retains ownership. The mmap remains valid as long as the fd
/// (held by the anchor) stays open.
#[expect(clippy::cast_possible_truncation, reason = "struct argsz always fits u32")]
fn map_bar_from_fd(
    bdf: &str,
    device_fd: BorrowedFd<'_>,
    bar_index: u32,
) -> Result<MappedBar, DriverError> {
    let mut region_info = VfioRegionInfo {
        argsz: std::mem::size_of::<VfioRegionInfo>() as u32,
        index: bar_index,
        ..Default::default()
    };
    ioctl::device_get_region_info(device_fd, &mut region_info)?;

    if region_info.size == 0 {
        return Err(DriverError::MmapFailed(Cow::Owned(format!(
            "BAR{bar_index} region has size 0 for {bdf}"
        ))));
    }

    let region_size = region_info.size as usize;

    // SAFETY: device fd is valid (held by anchor); region offset from kernel;
    // size verified non-zero; MAP_SHARED for MMIO semantics.
    let raw_ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            region_size,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            device_fd,
            region_info.offset,
        )
        .map_err(|e| {
            DriverError::MmapFailed(Cow::Owned(format!(
                "BAR{bar_index} mmap failed for {bdf}: {e}"
            )))
        })?
    };

    if raw_ptr.is_null() {
        return Err(DriverError::MmapFailed(Cow::Owned(format!(
            "BAR{bar_index} mmap returned null for {bdf}"
        ))));
    }

    let base_ptr = raw_ptr.cast::<u8>();

    tracing::info!(
        bdf,
        bar = bar_index,
        size = format_args!("{region_size:#x}"),
        "Clutch: BAR mapped from anchor fd"
    );

    // SAFETY: base_ptr/region_size come from the successful mmap above.
    let region = unsafe { MmioRegion::new(base_ptr, region_size) };

    Ok(MappedBar { region })
}
