// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for nvPmu.

/// Result alias using [`NvPmuError`].
pub type Result<T> = std::result::Result<T, NvPmuError>;

/// Errors from nvPmu operations.
#[derive(Debug, thiserror::Error)]
pub enum NvPmuError {
    #[error("I/O error: {0}")]
    Sysfs(#[from] std::io::Error),

    #[error("hwmon sensor not found: {0}")]
    SensorNotFound(String),

    #[error("parse error: {path}: {source}")]
    Parse {
        path: String,
        source: std::num::ParseIntError,
    },

    #[error("no NVIDIA GPU found")]
    NoGpu,

    #[error("hardware error: {0}")]
    Hardware(String),

    #[error("thermal safety limit exceeded: {temp_mc} m°C > {limit_mc} m°C")]
    ThermalLimit { temp_mc: i64, limit_mc: i64 },

    #[error("partial init failure: {applied}/{total} steps applied, rollback {rollback_status}")]
    PartialInit {
        applied: usize,
        total: usize,
        rollback_status: String,
    },

    #[error("power transition {from} → {to} failed: {reason}")]
    PowerTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("register timeout at {offset:#x}: expected {expected:#010x}, got {got:#010x}")]
    RegisterTimeout {
        offset: u64,
        expected: u32,
        got: u32,
    },

    #[error("framebuffer/HBM2 is not trained — D3cold recovery or nouveau warm cycle required")]
    FbUntrained,
}
