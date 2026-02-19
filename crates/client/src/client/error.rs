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
    Http(String), // EVOLVED: No reqwest dependency! ✅

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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error), // EVOLVED: For Unix socket errors ✅
}

// ============================================================================
// Conversions: ClientError → ToadStoolError
// ============================================================================

impl From<ClientError> for ToadStoolError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Http(e) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: "json-rpc".to_string(),
                reason: e,
            }),
            ClientError::WebSocket(msg) => {
                ToadStoolError::Network(NetworkError::ConnectionFailed {
                    endpoint: "websocket".to_string(),
                    reason: msg,
                })
            }
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
            ClientError::Serialization(e) => ToadStoolError::System(SystemError::Serialization {
                reason: e.to_string(),
            }),
            ClientError::UrlParse(e) => {
                ToadStoolError::Configuration(ConfigError::ValidationError {
                    reason: format!("Invalid URL: {}", e),
                })
            }
            ClientError::Io(e) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: "unix-socket".to_string(),
                reason: e.to_string(),
            }),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_error_http_display() {
        let e = ClientError::Http("connection refused".into());
        assert!(format!("{e}").contains("connection refused"));
    }

    #[test]
    fn test_client_error_authentication_display() {
        let e = ClientError::Authentication("invalid token".into());
        assert!(format!("{e}").contains("invalid token"));
    }

    #[test]
    fn test_client_error_timeout_display() {
        let e = ClientError::Timeout("operation took too long".into());
        assert!(format!("{e}").contains("operation took too long"));
    }

    #[test]
    fn test_client_error_server_display() {
        let e = ClientError::Server("internal server error".into());
        assert!(format!("{e}").contains("internal server error"));
    }

    #[test]
    fn test_client_error_configuration_display() {
        let e = ClientError::Configuration("missing field".into());
        assert!(format!("{e}").contains("missing field"));
    }

    #[test]
    fn test_client_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let client_err: ClientError = io_err.into();
        assert!(
            format!("{client_err}").contains("IO error")
                || matches!(client_err, ClientError::Io(_))
        );
    }

    #[test]
    fn test_client_error_from_url_parse() {
        let url_err = url::Url::parse("not_a_url").unwrap_err();
        let client_err: ClientError = url_err.into();
        assert!(matches!(client_err, ClientError::UrlParse(_)));
    }

    #[test]
    fn test_client_error_to_toadstool_error_http() {
        let client_err = ClientError::Http("bad".into());
        let toadstool_err: ToadStoolError = client_err.into();
        assert!(matches!(toadstool_err, ToadStoolError::Network(_)));
    }

    #[test]
    fn test_client_error_to_toadstool_error_auth() {
        let client_err = ClientError::Authentication("bad token".into());
        let toadstool_err: ToadStoolError = client_err.into();
        assert!(matches!(toadstool_err, ToadStoolError::Security(_)));
    }

    #[test]
    fn test_client_error_to_toadstool_error_configuration() {
        let client_err = ClientError::Configuration("bad config".into());
        let toadstool_err: ToadStoolError = client_err.into();
        assert!(matches!(toadstool_err, ToadStoolError::Configuration(_)));
    }

    #[test]
    fn test_client_error_to_toadstool_error_timeout() {
        let client_err = ClientError::Timeout("10s timeout".into());
        let toadstool_err: ToadStoolError = client_err.into();
        assert!(matches!(toadstool_err, ToadStoolError::Execution(_)));
    }

    #[test]
    fn test_toadstool_error_to_client_error_network() {
        use toadstool_common::error::NetworkError;
        let ts_err = ToadStoolError::Network(NetworkError::ConnectionFailed {
            endpoint: "test".into(),
            reason: "refused".into(),
        });
        let client_err: ClientError = ts_err.into();
        assert!(matches!(client_err, ClientError::Server(_)));
    }
}
