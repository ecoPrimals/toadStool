// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed errors for sysfs, swap, and device lifecycle operations.
//!
//! Absorbed from coralReef `coral-ember` and adapted for toadStool's
//! hardware-agnostic device holder model.
#![expect(
    missing_docs,
    reason = "Variants are self-describing via `#[error]` and thiserror `Display`."
)]

/// Errors from sysfs driver operations.
#[derive(Debug, thiserror::Error)]
pub enum SysfsError {
    #[error("sysfs write to {path}: {reason}")]
    Write { path: String, reason: String },
    #[error("sysfs read from {path}: {reason}")]
    Read { path: String, reason: String },
    #[error("driver bind failed for {bdf}: {reason}")]
    DriverBind { bdf: String, reason: String },
    #[error("PCI reset failed for {bdf}: {reason}")]
    PciReset { bdf: String, reason: String },
    #[error("parent PCI bridge not found for device {bdf}")]
    BridgeNotFound { bdf: String },
    #[error("parent bridge {bridge_bdf} has no sysfs reset file (device {bdf})")]
    BridgeResetMissing { bdf: String, bridge_bdf: String },
    #[error("PCI device {bdf} did not re-appear after bus rescan")]
    DeviceNotReappeared { bdf: String },
    #[error("{bdf}: PM power cycle resulted in D3cold")]
    PmCycleD3cold { bdf: String },
}

/// Errors from swap orchestration (preflight, sysfs, DRM isolation, trace).
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("preflight check failed for {bdf}: {reason}")]
    Preflight { bdf: String, reason: String },
    #[error("DRM isolation check failed: {0}")]
    DrmIsolation(String),
    #[error("external VFIO holders detected for {bdf}: {count} holders")]
    ExternalVfioHolders { bdf: String, count: usize },
    #[error("sysfs operation failed: {0}")]
    Sysfs(#[from] SysfsError),
    #[error("unknown target driver: {0}")]
    UnknownTarget(String),
    #[error("trace operation failed: {0}")]
    Trace(String),
    #[error("post-bind verification failed for {bdf}: {detail}")]
    VerifyHealth { bdf: String, detail: String },
    #[error("swap blocked: active display GPU at {bdf} — unbinding would crash the system")]
    ActiveDisplayGpu { bdf: String },
    #[error("VFIO reacquire failed for {bdf}: {reason}")]
    VfioReacquire { bdf: String, reason: String },
    #[error("unknown or unsupported reset method: {0}")]
    InvalidResetMethod(String),
    #[error("{0}")]
    Other(String),
}

impl From<String> for SwapError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}
