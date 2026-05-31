// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO warm anchor — minimal fd holder that prevents GPU bus resets.
//!
//! On Volta+ GPUs with HBM2, closing all VFIO fds triggers a PCI bus
//! reset that kills HBM2 training state. A cold GPU can only recover
//! via full system power cycle (the boot ROM trains HBM2 from hardware
//! reset vectors — no software path exists).
//!
//! `VfioAnchor` holds the bare minimum VFIO state to keep the kernel
//! from resetting the device: the device fd and container/iommufd fd.
//! No BAR0 mmap, no DMA mappings, no channel state. This is the
//! "clutch disengaged" idle state — just enough backpressure to keep
//! the engine from stalling.
//!
//! # Lifecycle
//!
//! ```text
//! daemon start → open_anchor(bdf) → VfioAnchor (fd held)
//!                                      ↓
//!          sovereign.init / dispatch engage the clutch
//!          (create BAR0 mmap + DMA from anchor's fds)
//!                                      ↓
//!          operation complete → clutch disengages
//!          (BAR0 unmapped, DMA freed — anchor still holds fd)
//!                                      ↓
//! daemon stop (SIGTERM) → anchor.leak() → fds NOT closed
//!                         (GPU stays warm through restart)
//! ```

use std::os::fd::OwnedFd;
use std::sync::Arc;

/// Minimal VFIO fd holder that prevents GPU bus resets.
///
/// Holds the VFIO device fd and backend fd (container or iommufd)
/// without any BAR mappings or DMA state. Dropping this struct
/// closes the fds, which may trigger a kernel bus reset on the GPU.
/// Use [`leak()`](VfioAnchor::leak) before process exit to prevent this.
#[derive(Debug)]
pub struct VfioAnchor {
    /// PCI BDF address (e.g. `"0000:02:00.0"`).
    pub bdf: String,
    /// VFIO device fd — the primary fd that tells the kernel "this
    /// device is in use." Closing it may trigger FLR/bus reset.
    device_fd: OwnedFd,
    /// Backend-specific fd that must outlive the device fd.
    backend: AnchorBackend,
}

/// Backend-specific state held alongside the device fd.
#[derive(Debug)]
enum AnchorBackend {
    /// Modern iommufd path: holds `/dev/iommu` fd + IOAS ID.
    Iommufd {
        iommufd: Arc<OwnedFd>,
        ioas_id: u32,
    },
    /// Legacy container/group path: holds container + group fds.
    LegacyGroup {
        container: Arc<OwnedFd>,
        group: OwnedFd,
    },
}

impl VfioAnchor {
    /// Create an anchor from pre-opened VFIO fds (iommufd backend).
    #[must_use]
    pub fn from_iommufd(bdf: String, device_fd: OwnedFd, iommufd: Arc<OwnedFd>, ioas_id: u32) -> Self {
        Self {
            bdf,
            device_fd,
            backend: AnchorBackend::Iommufd { iommufd, ioas_id },
        }
    }

    /// Create an anchor from pre-opened VFIO fds (legacy group backend).
    #[must_use]
    pub fn from_legacy(bdf: String, device_fd: OwnedFd, container: Arc<OwnedFd>, group: OwnedFd) -> Self {
        Self {
            bdf,
            device_fd,
            backend: AnchorBackend::LegacyGroup { container, group },
        }
    }

    /// Leak all VFIO fds so they are NOT closed when this struct is dropped.
    ///
    /// Call this in the SIGTERM handler before process exit. The kernel
    /// keeps the fds alive (associated with the dying process) but does
    /// not trigger a bus reset because the fds were never explicitly closed.
    /// Combined with cleared `reset_method` sysfs entries, this preserves
    /// GPU warm state across daemon restarts.
    pub fn leak(self) {
        tracing::info!(bdf = %self.bdf, "VfioAnchor: leaking fds to preserve warm state");
        std::mem::forget(self);
    }

    /// Release the anchor after FLR has been suppressed via
    /// `prepare_anchor_release()`.
    ///
    /// Dropping a `VfioAnchor` closes the VFIO device fd, which normally
    /// triggers `vfio_pci_core_release()` → device reset. Callers must
    /// clear `reset_method` first to prevent this. In debug builds, this
    /// method asserts that `reset_method` is empty.
    ///
    /// Exp 225: anchor drop without FLR suppression destroyed VBIOS warm
    /// state (PMC_ENABLE 0x5fecdff1 → 0x40000020).
    pub fn release_prepared(self) {
        #[cfg(debug_assertions)]
        {
            let reset_path = toadstool_common::sysfs_paths::sysfs_pci_device_file(
                &self.bdf,
                "reset_method",
            );
            if let Ok(method) = std::fs::read_to_string(&reset_path) {
                let trimmed = method.trim();
                assert!(
                    trimmed.is_empty(),
                    "release_prepared called on {} but reset_method is '{}' — \
                     call prepare_anchor_release first",
                    self.bdf,
                    trimmed,
                );
            }
        }
        tracing::info!(bdf = %self.bdf, "VfioAnchor: releasing with FLR suppressed");
        drop(self);
    }

    /// Get the BDF address this anchor holds.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Borrow the device fd for creating BAR0 mmaps or DMA backends.
    #[must_use]
    pub fn device_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.device_fd.as_fd()
    }

    /// Get the IOAS ID (iommufd backend only).
    #[must_use]
    pub fn ioas_id(&self) -> Option<u32> {
        match &self.backend {
            AnchorBackend::Iommufd { ioas_id, .. } => Some(*ioas_id),
            AnchorBackend::LegacyGroup { .. } => None,
        }
    }

    /// Get the backend fd for DMA operations.
    /// Returns the iommufd fd or the legacy container fd.
    #[must_use]
    pub fn backend_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        match &self.backend {
            AnchorBackend::Iommufd { iommufd, .. } => iommufd.as_fd(),
            AnchorBackend::LegacyGroup { container, .. } => container.as_fd(),
        }
    }

    /// Get the legacy group fd, if this anchor uses the legacy backend.
    #[must_use]
    pub fn group_fd(&self) -> Option<std::os::fd::BorrowedFd<'_>> {
        use std::os::fd::AsFd;
        match &self.backend {
            AnchorBackend::LegacyGroup { group, .. } => Some(group.as_fd()),
            AnchorBackend::Iommufd { .. } => None,
        }
    }

    /// Clone the backend Arc for creating DMA backends from this anchor.
    #[must_use]
    pub fn backend_arc(&self) -> AnchorBackendRef {
        match &self.backend {
            AnchorBackend::Iommufd { iommufd, ioas_id } => AnchorBackendRef::Iommufd {
                iommufd: Arc::clone(iommufd),
                ioas_id: *ioas_id,
            },
            AnchorBackend::LegacyGroup { container, .. } => AnchorBackendRef::LegacyGroup {
                container: Arc::clone(container),
            },
        }
    }
}

/// Cloneable reference to the anchor's backend fds for creating
/// DMA backends without moving the anchor.
#[derive(Debug, Clone)]
pub enum AnchorBackendRef {
    /// iommufd backend reference.
    Iommufd {
        /// Shared iommufd fd.
        iommufd: Arc<OwnedFd>,
        /// IOAS ID for DMA mappings.
        ioas_id: u32,
    },
    /// Legacy container backend reference.
    LegacyGroup {
        /// Shared container fd.
        container: Arc<OwnedFd>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;

    fn test_fd() -> OwnedFd {
        OwnedFd::from(std::fs::File::open("/dev/null").unwrap())
    }

    #[test]
    fn anchor_iommufd_holds_bdf() {
        let fd = test_fd();
        let iommufd = Arc::new(test_fd());
        let anchor = VfioAnchor::from_iommufd("0000:02:00.0".into(), fd, iommufd, 42);
        assert_eq!(anchor.bdf(), "0000:02:00.0");
        assert_eq!(anchor.ioas_id(), Some(42));
    }

    #[test]
    fn anchor_legacy_holds_bdf() {
        let fd = test_fd();
        let container = Arc::new(test_fd());
        let group = test_fd();
        let anchor = VfioAnchor::from_legacy("0000:49:00.0".into(), fd, container, group);
        assert_eq!(anchor.bdf(), "0000:49:00.0");
        assert_eq!(anchor.ioas_id(), None);
    }

    #[test]
    fn device_fd_is_valid() {
        let fd = test_fd();
        let raw = fd.as_raw_fd();
        let iommufd = Arc::new(test_fd());
        let anchor = VfioAnchor::from_iommufd("0000:02:00.0".into(), fd, iommufd, 1);
        assert_eq!(anchor.device_fd().as_raw_fd(), raw);
    }

    #[test]
    fn backend_ref_is_cloneable() {
        let fd = test_fd();
        let iommufd = Arc::new(test_fd());
        let anchor = VfioAnchor::from_iommufd("0000:02:00.0".into(), fd, iommufd, 1);
        let r1 = anchor.backend_arc();
        let _r2 = r1.clone();
    }
}
