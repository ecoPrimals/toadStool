//! Client error types
//!
//! This module defines the error types used throughout the ToadStool client library.
//! ClientError integrates with the unified ToadStool error system while providing
//! client-specific error handling.

use thiserror::Error;
use toadstool_common::error::{
    ConfigError, ExecutionError, NetworkError, SecurityError, SystemError, ToadStoolError,
};

/// ToadStool client errors
///
/// Client-specific error type that wraps common client library errors and provides
/// bidirectional conversion with ToadStoolError for seamless integration.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

// ============================================================================
// Conversions: ClientError → ToadStoolError
// ============================================================================

impl From<ClientError> for ToadStoolError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Http(e) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: format!("HTTP request: {}", e.url().map(|u| u.as_str()).unwrap_or("unknown")),
                reason: e.to_string(),
            }),
            ClientError::WebSocket(msg) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: "websocket".to_string(),
                reason: msg,
            }),
            ClientError::Authentication(msg) => {
                ToadStoolError::Security(SecurityError::AuthenticationFailed { reason: msg })
            }
            ClientError::Configuration(msg) => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: msg })
            }
            ClientError::Server(msg) => {
                ToadStoolError::System(SystemError::Internal { reason: msg })
            }
            ClientError::Timeout(msg) => {
                ToadStoolError::Execution(ExecutionError::Timeout {
                    duration: std::time::Duration::from_secs(0), // Unknown duration
                    operation: msg,
                })
            }
            ClientError::Serialization(e) => {
                ToadStoolError::System(SystemError::Serialization {
                    reason: e.to_string(),
                })
            }
            ClientError::UrlParse(e) => {
                ToadStoolError::Configuration(ConfigError::ValidationError {
                    reason: format!("Invalid URL: {}", e),
                })
            }
        }
    }
}

// ============================================================================
// Conversions: ToadStoolError → ClientError  
// ============================================================================

impl From<ToadStoolError> for ClientError {
    fn from(error: ToadStoolError) -> Self {
        match error {
            ToadStoolError::Network(_) => ClientError::Server(error.to_string()),
            ToadStoolError::Security(_) => ClientError::Authentication(error.to_string()),
            ToadStoolError::Configuration(_) => ClientError::Configuration(error.to_string()),
            ToadStoolError::Execution(ExecutionError::Timeout { .. }) => {
                ClientError::Timeout(error.to_string())
            }
            _ => ClientError::Server(error.to_string()),
        }
    }
}

/// Result type alias for client operations
pub type ClientResult<T> = Result<T, ClientError>;
