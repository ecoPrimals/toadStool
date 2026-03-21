// SPDX-License-Identifier: AGPL-3.0-only
//! Parsing and validation helpers for environment variable overrides.

use super::super::{ConfigError, ConfigResult};

/// Parse a string as a boolean ("true" = true, anything else = false).
#[must_use]
pub fn parse_bool(s: &str) -> bool {
    s.to_lowercase() == "true"
}

/// Parse a string as u16, returning a `ConfigError` on failure.
pub fn parse_u16(s: &str, field: &str) -> ConfigResult<u16> {
    s.parse()
        .map_err(|e| ConfigError::Invalid(format!("Invalid {field}: {e}")))
}

/// Parse a string as u32, returning a `ConfigError` on failure.
pub fn parse_u32(s: &str, field: &str) -> ConfigResult<u32> {
    s.parse()
        .map_err(|e| ConfigError::Invalid(format!("Invalid {field}: {e}")))
}

/// Parse a string as u64, returning a `ConfigError` on failure.
pub fn parse_u64(s: &str, field: &str) -> ConfigResult<u64> {
    s.parse()
        .map_err(|e| ConfigError::Invalid(format!("Invalid {field}: {e}")))
}

/// Parse a string as f64, returning a `ConfigError` on failure.
pub fn parse_f64(s: &str, field: &str) -> ConfigResult<f64> {
    s.parse()
        .map_err(|e| ConfigError::Invalid(format!("Invalid {field}: {e}")))
}

/// Parse a string as usize, returning a `ConfigError` on failure.
pub fn parse_usize(s: &str, field: &str) -> ConfigResult<usize> {
    s.parse()
        .map_err(|e| ConfigError::Invalid(format!("Invalid {field}: {e}")))
}
