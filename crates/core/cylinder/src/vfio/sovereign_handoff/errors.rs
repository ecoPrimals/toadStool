// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DriverError;
use crate::vfio::kmod::KmodError;

/// Errors from sovereign warm handoff operations.
#[derive(Debug, thiserror::Error)]
pub enum HandoffError {
    #[error("handoff lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("handoff already in progress for {bdf}")]
    HandoffInProgress { bdf: String },

    #[error("BAR0 access failed: {0}")]
    BarAccessFailed(#[from] DriverError),

    #[error("failed to read /proc/devices: {0}")]
    ProcDevicesRead(#[from] std::io::Error),

    #[error("{module_name} chardev not found in /proc/devices — __register_chrdev may have been NOPed")]
    ChardevNotFound { module_name: String },

    #[error("mknodat({path}): {detail}")]
    DeviceNodeCreateFailed { path: String, detail: String },

    #[error("failed to open {path}: {source}")]
    ChardevOpenFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("dependency resolution failed for {module}: {source}")]
    ModuleDependencyResolutionFailed {
        module: String,
        #[source]
        source: KmodError,
    },
}
