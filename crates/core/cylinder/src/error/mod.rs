// SPDX-License-Identifier: AGPL-3.0-or-later
//! Driver error types.

mod vfio;
pub use vfio::{ChannelError, DevinitError, PciDiscoveryError, SovereignStagesError};

use std::borrow::Cow;

/// Result alias for driver operations.
pub type DriverResult<T> = Result<T, DriverError>;

/// Errors from GPU device operations.
///
/// String-carrying variants use `Cow<'static, str>` so that static messages
/// (the common case) are zero-alloc, while dynamic messages still work via
/// `format!("...").into()`.
#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    /// No matching GPU device was found.
    #[error("device not found: {0}")]
    DeviceNotFound(Cow<'static, str>),

    /// A DRM ioctl syscall failed.
    #[error("DRM ioctl failed: {name} returned {errno}")]
    IoctlFailed {
        /// Name of the ioctl for error reporting.
        name: &'static str,
        /// Kernel errno (negative on Linux).
        errno: i32,
    },

    /// Buffer allocation failed (OOM or invalid domain).
    #[error("buffer allocation failed: size={size}, domain={domain:?} — {detail}")]
    AllocFailed {
        /// Requested buffer size in bytes.
        size: u64,
        /// Memory domain that was requested.
        domain: crate::MemoryDomain,
        /// Additional context.
        detail: String,
    },

    /// The buffer handle is invalid or was already freed.
    #[error("buffer not found: handle={0:?}")]
    BufferNotFound(crate::BufferHandle),

    /// Memory mapping of a GEM buffer failed.
    #[error("mmap failed: {0}")]
    MmapFailed(Cow<'static, str>),

    /// Command submission to the GPU failed.
    #[error("command submission failed: {0}")]
    SubmitFailed(Cow<'static, str>),

    /// The fence did not signal within the timeout period.
    #[error("fence timeout after {ms}ms")]
    FenceTimeout {
        /// Timeout duration in milliseconds.
        ms: u64,
    },

    /// Device open / context creation failed.
    #[error("device open failed: {0}")]
    OpenFailed(Cow<'static, str>),

    /// Compute dispatch (kernel launch) failed.
    #[error("dispatch failed: {0}")]
    DispatchFailed(Cow<'static, str>),

    /// GPU synchronization (fence / stream sync) failed.
    #[error("sync failed: {0}")]
    SyncFailed(Cow<'static, str>),

    /// Oracle / BAR0 register operation failed.
    #[error("oracle error: {0}")]
    OracleError(Cow<'static, str>),

    /// Wrapped I/O error from file operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Operation or API not available for this device / backend.
    #[error("unsupported: {0}")]
    Unsupported(Cow<'static, str>),

    /// Hardware guard refused a register write to protect the GPU.
    #[error("hardware guard: {0}")]
    HardwareGuardRefusal(Cow<'static, str>),

    /// Device is exclusively held by a live ember instance.
    #[error("device {bdf} is held by ember — use EmberSession::connect() instead of direct open")]
    DeviceHeldByEmber {
        /// PCI BDF address of the held device.
        bdf: String,
    },

    /// PCI sysfs/config-space discovery or PM transition failed.
    #[error("PCI discovery: {0}")]
    PciDiscovery(#[from] PciDiscoveryError),

    /// VFIO channel oracle / BAR0 resource access failed.
    #[error("channel: {0}")]
    Channel(#[from] ChannelError),

    /// VBIOS / devinit (PROM, interpreter, PMU upload) failed.
    #[error("devinit: {0}")]
    Devinit(#[from] DevinitError),

    /// Sovereign init stage helpers.
    #[error("sovereign stages: {0}")]
    SovereignStages(#[from] SovereignStagesError),
}

impl DriverError {
    /// Platform overflow during numeric conversion.
    pub(crate) fn platform_overflow(msg: &'static str) -> Self {
        Self::MmapFailed(msg.into())
    }

    /// Create an oracle error from a dynamic string.
    pub fn oracle(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::OracleError(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn error_display_device_not_found() {
        let e = DriverError::DeviceNotFound("no amdgpu".into());
        assert!(e.to_string().contains("no amdgpu"));
    }

    #[test]
    fn error_display_ioctl_failed() {
        let e = DriverError::IoctlFailed {
            name: "drm_ioctl",
            errno: -22,
        };
        let msg = e.to_string();
        assert!(msg.contains("drm_ioctl"));
        assert!(msg.contains("-22"));
    }

    #[test]
    fn error_display_alloc_failed() {
        let e = DriverError::AllocFailed {
            size: 4096,
            domain: crate::MemoryDomain::Vram,
            detail: "oom".into(),
        };
        assert!(e.to_string().contains("4096"));
    }

    #[test]
    fn error_display_buffer_not_found() {
        let e = DriverError::BufferNotFound(crate::BufferHandle(42));
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn error_display_mmap_failed() {
        let e = DriverError::MmapFailed("out of memory".into());
        assert!(e.to_string().contains("out of memory"));
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no device");
        let e: DriverError = io_err.into();
        assert!(e.to_string().contains("no device"));
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(DriverError::DeviceNotFound("test".into()));
        assert!(e.to_string().contains("test"));
    }

    #[test]
    fn error_platform_overflow() {
        let e = DriverError::platform_overflow("offset exceeds platform pointer width");
        assert!(
            e.to_string()
                .contains("offset exceeds platform pointer width")
        );
    }

    #[test]
    fn error_source_chain() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "root required");
        let e: DriverError = io_err.into();
        assert!(e.source().is_some());
    }

    #[test]
    fn error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DriverError>();
    }

    #[test]
    fn error_oracle_helper_builds_variant() {
        let e = DriverError::oracle("dynamic oracle message");
        assert!(matches!(e, DriverError::OracleError(_)));
    }

    #[test]
    fn error_display_unsupported() {
        let e = DriverError::Unsupported("legacy API on iommufd".into());
        assert!(e.to_string().contains("unsupported"));
    }

    #[test]
    fn error_display_pci_discovery_variant() {
        let inner = PciDiscoveryError::InvalidBdf { bdf: "bad".into() };
        let e: DriverError = inner.into();
        assert!(e.to_string().contains("PCI discovery"));
    }

    #[test]
    fn error_display_channel_variant() {
        let inner = ChannelError::Bar0ReadsAllOnes;
        let e: DriverError = inner.into();
        assert!(e.to_string().contains("channel"));
    }

    #[test]
    fn error_display_devinit_variant() {
        let inner = DevinitError::BitSignatureNotFound;
        let e: DriverError = inner.into();
        assert!(e.to_string().contains("devinit"));
    }

    #[test]
    fn error_display_sovereign_stages_variant() {
        let inner = SovereignStagesError::Bar0ProbeTimeout;
        let e: DriverError = inner.into();
        assert!(e.to_string().contains("sovereign stages"));
    }
}
