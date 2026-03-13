// SPDX-License-Identifier: AGPL-3.0-only
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
}
