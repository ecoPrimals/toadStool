//! Primal integration error types
//!
//! This module defines errors for integrating with ecosystem primals (Songbird, BearDog, etc.).
//! PrimalError integrates with the unified ToadStoolError system for consistent error handling.

use thiserror::Error;
use toadstool_common::error::{
    ConfigError, ExecutionError, IntegrationError, NetworkError, ResourceError, SecurityError,
    ToadStoolError,
};

pub type PrimalResult<T> = Result<T, PrimalError>;

/// Primal integration error types
///
/// Errors specific to ecosystem primal integration that can be converted to and from
/// ToadStoolError for seamless error propagation across the platform.
#[derive(Debug, Error)]
pub enum PrimalError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Network error: {source}")]
    Network { source: reqwest::Error },

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Integration error: {primal} - {message}")]
    Integration { primal: String, message: String },

    #[error("Timeout error: {operation}")]
    Timeout { operation: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Resource error: {message}")]
    Resource { message: String },
}

// ============================================================================
// Conversions: PrimalError → ToadStoolError
// ============================================================================

impl From<PrimalError> for ToadStoolError {
    fn from(error: PrimalError) -> Self {
        match error {
            PrimalError::Configuration { message } => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: message })
            }
            PrimalError::Network { source } => {
                ToadStoolError::Network(NetworkError::ConnectionFailed {
                    endpoint: format!("HTTP request: {}", source.url().map(|u| u.as_str()).unwrap_or("unknown")),
                    reason: source.to_string(),
                })
            }
            PrimalError::Authentication { message } => {
                ToadStoolError::Security(SecurityError::AuthenticationFailed { reason: message })
            }
            PrimalError::ServiceUnavailable { service } => {
                ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                    service,
                    reason: "Service is currently unavailable".to_string(),
                })
            }
            PrimalError::Integration { primal, message } => {
                ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                    service: primal,
                    reason: message,
                })
            }
            PrimalError::Timeout { operation } => {
                ToadStoolError::Execution(ExecutionError::Timeout {
                    duration: std::time::Duration::from_secs(0), // Unknown duration
                    operation,
                })
            }
            PrimalError::Validation { message } => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: message })
            }
            PrimalError::Resource { message } => {
                ToadStoolError::Resource(ResourceError::AllocationFailure {
                    resource: "primal".to_string(),
                    reason: message,
                })
            }
        }
    }
}

// ============================================================================
// Conversions: ToadStoolError → PrimalError
// ============================================================================

impl From<ToadStoolError> for PrimalError {
    fn from(error: ToadStoolError) -> Self {
        match error {
            ToadStoolError::Configuration(_) => PrimalError::Configuration {
                message: error.to_string(),
            },
            ToadStoolError::Network(_) => PrimalError::Network {
                source: reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error.to_string(),
                )),
            },
            ToadStoolError::Security(_) => PrimalError::Authentication {
                message: error.to_string(),
            },
            ToadStoolError::Integration(IntegrationError::ServiceUnavailable { service, .. }) => {
                PrimalError::ServiceUnavailable { service }
            }
            ToadStoolError::Integration(_) => PrimalError::Integration {
                primal: "unknown".to_string(),
                message: error.to_string(),
            },
            ToadStoolError::Execution(ExecutionError::Timeout { operation, .. }) => {
                PrimalError::Timeout { operation }
            }
            ToadStoolError::Resource(_) => PrimalError::Resource {
                message: error.to_string(),
            },
            _ => PrimalError::Integration {
                primal: "unknown".to_string(),
                message: error.to_string(),
            },
        }
    }
}
