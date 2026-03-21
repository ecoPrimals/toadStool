// SPDX-License-Identifier: AGPL-3.0-only
//! Error types for nvPmu.

/// Result alias using [`NvPmuError`].
pub type Result<T> = std::result::Result<T, NvPmuError>;

/// Errors from nvPmu operations.
#[derive(Debug, thiserror::Error)]
pub enum NvPmuError {
    /// Sysfs I/O error (e.g. reading hwmon or PCI device attributes).
    #[error("I/O error: {0}")]
    Sysfs(#[from] std::io::Error),

    /// Hwmon sensor or nvidia-smi not found or inaccessible.
    #[error("hwmon sensor not found: {0}")]
    SensorNotFound(String),

    /// Failed to parse a sysfs value (e.g. hex vendor ID, decimal temperature).
    #[error("parse error: {path}: {source}")]
    Parse {
        /// Sysfs path that failed to parse.
        path: String,
        /// Underlying parse error.
        source: std::num::ParseIntError,
    },

    /// No NVIDIA GPU found on the PCI bus.
    #[error("no NVIDIA GPU found")]
    NoGpu,

    /// Hardware error (e.g. register access failure, PCI reset failure).
    #[error("hardware error: {0}")]
    Hardware(String),

    /// GPU temperature exceeded critical thermal safety threshold.
    #[error("thermal safety limit exceeded: {temp_mc} m°C > {limit_mc} m°C")]
    ThermalLimit {
        /// Current temperature in millidegrees Celsius.
        temp_mc: i64,
        /// Configured critical limit in millidegrees Celsius.
        limit_mc: i64,
    },

    /// PMU init recipe partially failed; rollback was attempted.
    #[error("partial init failure: {applied}/{total} steps applied, rollback {rollback_status}")]
    PartialInit {
        /// Number of steps successfully applied before failure.
        applied: usize,
        /// Total number of steps in the recipe.
        total: usize,
        /// Rollback outcome (e.g. "succeeded" or "partial — GPU may need reset").
        rollback_status: String,
    },

    /// Power state transition failed (e.g. Warm → Glow, Sleep → D0).
    #[error("power transition {from} → {to} failed: {reason}")]
    PowerTransition {
        /// Source power state.
        from: String,
        /// Target power state.
        to: String,
        /// Failure reason.
        reason: String,
    },

    /// BAR0 register readback did not match expected value after write.
    #[error("register timeout at {offset:#x}: expected {expected:#010x}, got {got:#010x}")]
    RegisterTimeout {
        /// BAR0-relative register offset.
        offset: u64,
        /// Expected register value.
        expected: u32,
        /// Actual value read back.
        got: u32,
    },

    /// HBM2 framebuffer is untrained; D3cold recovery or nouveau warm cycle required.
    #[error("framebuffer/HBM2 is not trained — D3cold recovery or nouveau warm cycle required")]
    FbUntrained,
}
