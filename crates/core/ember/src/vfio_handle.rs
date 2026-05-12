// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO [`ResourceHandle`] — first production handle type.
//!
//! `VfioResourceHandle` corresponds to coralReef's `HeldDevice`: it represents
//! a PCI device exclusively claimed via VFIO. The held-fd pattern is generic
//! across GPU, NPU, USB, and HSM — this implementation covers the VFIO/PCI case.
//!
//! The actual VFIO ioctl operations (open, reset, region access) are delegated
//! to `hw-safe` and `nvpmu`. This module manages the lifecycle state and
//! metadata, not raw fd operations.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::ring_meta::RingMeta;
use crate::resource_handle::ResourceHandle;

/// Error type for VFIO handle operations.
#[derive(Debug, thiserror::Error)]
pub enum VfioHandleError {
    /// VFIO device fd is no longer valid (e.g. after kernel revocation).
    #[error("VFIO device {bdf} fd is no longer valid: {reason}")]
    FdInvalid {
        /// PCI BDF address.
        bdf: String,
        /// Reason the fd became invalid.
        reason: String,
    },
    /// Device reacquisition failed.
    #[error("failed to reacquire VFIO device {bdf}: {reason}")]
    ReacquireFailed {
        /// PCI BDF address.
        bdf: String,
        /// Failure reason.
        reason: String,
    },
    /// Device is in an unrecoverable power state.
    #[error("device {bdf} is in {state} — requires reboot")]
    BadPowerState {
        /// PCI BDF address.
        bdf: String,
        /// Current power state (e.g. "D3cold").
        state: String,
    },
}

/// A PCI device held open via VFIO — production [`ResourceHandle`] implementation.
///
/// Lifecycle:
/// 1. Constructed with a BDF when the VFIO group/device is opened
/// 2. `is_alive()` returns true as long as the VFIO fd is valid
/// 3. `release()` marks the handle as released (close VFIO group/device externally)
/// 4. `reacquire()` can re-open after a release if the device is still VFIO-bound
///
/// Ring/mailbox metadata is persisted alongside the handle for state
/// reconstruction after daemon restarts.
#[derive(Debug)]
pub struct VfioResourceHandle {
    /// PCI address (`0000:01:00.0` style).
    pub bdf: String,
    /// Ring/mailbox metadata persisted across restarts.
    pub ring_meta: RingMeta,
    alive: AtomicBool,
    /// Optional VFIO device fd number (for external tracking — ember does not own the fd).
    vfio_fd: Option<i32>,
}

impl VfioResourceHandle {
    /// Create a new VFIO handle for a device.
    #[must_use]
    pub fn new(bdf: String) -> Self {
        Self {
            bdf,
            ring_meta: RingMeta::default(),
            alive: AtomicBool::new(true),
            vfio_fd: None,
        }
    }

    /// Create with a known VFIO device fd and ring metadata.
    #[must_use]
    pub fn with_fd_and_meta(bdf: String, fd: i32, ring_meta: RingMeta) -> Self {
        Self {
            bdf,
            ring_meta,
            alive: AtomicBool::new(true),
            vfio_fd: Some(fd),
        }
    }

    /// The VFIO device fd number, if known.
    #[must_use]
    pub fn vfio_fd(&self) -> Option<i32> {
        self.vfio_fd
    }

    /// Set the VFIO device fd (e.g. after opening or receiving via `SCM_RIGHTS`).
    pub fn set_vfio_fd(&mut self, fd: i32) {
        self.vfio_fd = Some(fd);
    }
}

impl ResourceHandle for VfioResourceHandle {
    type Error = VfioHandleError;

    fn handle_type(&self) -> &'static str {
        "vfio"
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        self.alive.store(false, Ordering::Relaxed);
        self.vfio_fd = None;
        Ok(())
    }

    fn reacquire(&mut self) -> Result<bool, Self::Error> {
        let power = crate::sysfs::read_power_state(&self.bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(VfioHandleError::BadPowerState {
                bdf: self.bdf.clone(),
                state: "D3cold".to_string(),
            });
        }

        let driver_path = crate::sysfs::pci_device_path(&self.bdf, "driver");
        if driver_path.exists() {
            if let Ok(link) = std::fs::read_link(&driver_path) {
                let driver = link
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                if driver == "vfio-pci" {
                    self.alive.store(true, Ordering::Relaxed);
                    return Ok(true);
                }
                return Ok(false);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::held_resource::HeldResource;

    #[test]
    fn new_vfio_handle_is_alive() {
        let handle = VfioResourceHandle::new("0000:03:00.0".into());
        assert!(handle.is_alive());
        assert_eq!(handle.handle_type(), "vfio");
        assert_eq!(handle.bdf, "0000:03:00.0");
        assert!(handle.vfio_fd().is_none());
    }

    #[test]
    fn with_fd_and_meta() {
        let meta = RingMeta {
            mailboxes: vec![],
            rings: vec![],
            version: 7,
        };
        let handle = VfioResourceHandle::with_fd_and_meta("0000:4a:00.0".into(), 42, meta);
        assert!(handle.is_alive());
        assert_eq!(handle.vfio_fd(), Some(42));
        assert_eq!(handle.ring_meta.version, 7);
    }

    #[test]
    fn release_marks_dead_and_clears_fd() {
        let mut handle = VfioResourceHandle::with_fd_and_meta(
            "0000:03:00.0".into(),
            5,
            RingMeta::default(),
        );
        handle.release().unwrap();
        assert!(!handle.is_alive());
        assert!(handle.vfio_fd().is_none());
    }

    #[test]
    fn reacquire_returns_false_when_no_sysfs() {
        let mut handle = VfioResourceHandle::new("9999:99:99.9".into());
        handle.release().unwrap();
        let ok = handle.reacquire().unwrap();
        assert!(!ok);
    }

    #[test]
    fn set_vfio_fd() {
        let mut handle = VfioResourceHandle::new("0000:03:00.0".into());
        assert!(handle.vfio_fd().is_none());
        handle.set_vfio_fd(99);
        assert_eq!(handle.vfio_fd(), Some(99));
    }

    #[test]
    fn held_resource_wraps_vfio_handle() {
        let handle = VfioResourceHandle::new("0000:03:00.0".into());
        let mut held = HeldResource::new(handle);
        assert!(held.is_alive());
        assert_eq!(held.handle().handle_type(), "vfio");
        assert_eq!(held.handle().bdf, "0000:03:00.0");

        held.release().unwrap();
        assert!(!held.is_alive());
        assert_eq!(held.release_count(), 1);
    }

    #[test]
    fn ring_meta_persists_through_lifecycle() {
        let mut handle = VfioResourceHandle::new("0000:03:00.0".into());
        handle.ring_meta = RingMeta {
            mailboxes: vec![crate::ring_meta::MailboxMeta {
                engine: "fecs".into(),
                capacity: 16,
            }],
            rings: vec![],
            version: 1,
        };
        assert_eq!(handle.ring_meta.mailboxes.len(), 1);

        handle.release().unwrap();
        assert_eq!(handle.ring_meta.version, 1);
    }
}
