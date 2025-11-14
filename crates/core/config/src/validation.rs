//! Common validation utilities for ToadStool configurations
//!
//! This module provides reusable validation functions and traits for
//! configuration validation across the ToadStool platform.

use std::time::Duration;
use toadstool_common::error::{ConfigError, ConfigResult};

/// Trait for types that can be validated
pub trait Validate {
    /// Validate the instance
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if validation fails
    fn validate(&self) -> ConfigResult<()>;
}

/// Validate a port number is in valid range
pub fn validate_port(port: u16, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    // Note: MAX_PORT is u16::MAX (65535), so no need to check upper bound
    if port < validation::MIN_PORT {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: port.to_string(),
            reason: format!(
                "Port must be at least {} (privileged ports < 1024 should be avoided)",
                validation::MIN_PORT
            ),
        });
    }
    Ok(())
}

/// Validate a timeout duration is in valid range
pub fn validate_timeout(timeout: Duration, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    let timeout_ms = timeout.as_millis() as u64;

    if timeout_ms < validation::MIN_TIMEOUT_MS {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: format!("{}ms", timeout_ms),
            reason: format!("Timeout must be at least {}ms", validation::MIN_TIMEOUT_MS),
        });
    }

    if timeout_ms > validation::MAX_TIMEOUT_MS {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: format!("{}ms", timeout_ms),
            reason: format!(
                "Timeout must not exceed {}ms (1 hour)",
                validation::MAX_TIMEOUT_MS
            ),
        });
    }

    Ok(())
}

/// Validate a worker thread count
pub fn validate_worker_threads(count: usize, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    if count < validation::MIN_WORKER_THREADS {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: count.to_string(),
            reason: format!(
                "Worker thread count must be at least {}",
                validation::MIN_WORKER_THREADS
            ),
        });
    }

    if count > validation::MAX_WORKER_THREADS {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: count.to_string(),
            reason: format!(
                "Worker thread count must not exceed {}",
                validation::MAX_WORKER_THREADS
            ),
        });
    }

    Ok(())
}

/// Validate a pool size
pub fn validate_pool_size(size: u32, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    if (size as usize) < validation::MIN_POOL_SIZE {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: size.to_string(),
            reason: format!("Pool size must be at least {}", validation::MIN_POOL_SIZE),
        });
    }

    if (size as usize) > validation::MAX_POOL_SIZE {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: size.to_string(),
            reason: format!("Pool size must not exceed {}", validation::MAX_POOL_SIZE),
        });
    }

    Ok(())
}

/// Validate a cache size
pub fn validate_cache_size(size: usize, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    if size < validation::MIN_CACHE_SIZE {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: size.to_string(),
            reason: format!("Cache size must be at least {}", validation::MIN_CACHE_SIZE),
        });
    }

    if size > validation::MAX_CACHE_SIZE {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: size.to_string(),
            reason: format!("Cache size must not exceed {}", validation::MAX_CACHE_SIZE),
        });
    }

    Ok(())
}

/// Validate retry attempts
pub fn validate_retry_attempts(attempts: u32, field_name: &str) -> ConfigResult<()> {
    use crate::defaults::validation;

    // Note: MIN_RETRY_ATTEMPTS is 0 (u32::MIN), so no need to check lower bound
    if attempts > validation::MAX_RETRY_ATTEMPTS {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: attempts.to_string(),
            reason: format!(
                "Retry attempts must not exceed {}",
                validation::MAX_RETRY_ATTEMPTS
            ),
        });
    }

    Ok(())
}

/// Validate a non-empty string
pub fn validate_non_empty(value: &str, field_name: &str) -> ConfigResult<()> {
    if value.trim().is_empty() {
        return Err(ConfigError::MissingField {
            field: field_name.to_string(),
        });
    }
    Ok(())
}

/// Validate a URL format
pub fn validate_url(value: &str, field_name: &str) -> ConfigResult<()> {
    validate_non_empty(value, field_name)?;

    // Basic URL validation - starts with http:// or https://
    if !value.starts_with("http://") && !value.starts_with("https://") {
        return Err(ConfigError::InvalidValue {
            field: field_name.to_string(),
            value: value.to_string(),
            reason: "URL must start with http:// or https://".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_validate_port() {
        // Valid ports
        assert!(validate_port(1024, "port").is_ok());
        assert!(validate_port(8080, "port").is_ok());
        assert!(validate_port(65535, "port").is_ok());

        // Invalid ports
        assert!(validate_port(80, "port").is_err()); // Privileged
        assert!(validate_port(1023, "port").is_err());
    }

    #[test]
    fn test_validate_timeout() {
        // Valid timeouts
        assert!(validate_timeout(Duration::from_millis(100), "timeout").is_ok());
        assert!(validate_timeout(Duration::from_secs(30), "timeout").is_ok());

        // Invalid timeouts
        assert!(validate_timeout(Duration::from_millis(50), "timeout").is_err()); // Too short
        assert!(validate_timeout(Duration::from_secs(7200), "timeout").is_err());
        // Too long
    }

    #[test]
    fn test_validate_worker_threads() {
        // Valid counts
        assert!(validate_worker_threads(1, "workers").is_ok());
        assert!(validate_worker_threads(4, "workers").is_ok());
        assert!(validate_worker_threads(128, "workers").is_ok());

        // Invalid counts
        assert!(validate_worker_threads(0, "workers").is_err());
        assert!(validate_worker_threads(256, "workers").is_err());
    }

    #[test]
    fn test_validate_non_empty() {
        assert!(validate_non_empty("hello", "field").is_ok());
        assert!(validate_non_empty("", "field").is_err());
        assert!(validate_non_empty("   ", "field").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("http://localhost:8080", "url").is_ok());
        assert!(validate_url("https://example.com", "url").is_ok());
        assert!(validate_url("ftp://example.com", "url").is_err());
        assert!(validate_url("", "url").is_err());
    }
}
