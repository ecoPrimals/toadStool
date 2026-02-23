//! Client configuration types
//!
//! This module defines configuration structures for the ToadStool client,
//! including authentication methods and connection settings.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_config::defaults;

/// ToadStool client configuration
///
/// # Environment Variables
///
/// The following environment variables can be used to override defaults:
/// - `TOADSTOOL_API_URL`: Base URL of the ToadStool server
/// - `TOADSTOOL_REQUEST_TIMEOUT_MS`: HTTP request timeout in milliseconds
/// - `TOADSTOOL_WEBSOCKET_TIMEOUT_MS`: WebSocket connection timeout in milliseconds
/// - `TOADSTOOL_MAX_RETRIES`: Maximum retry attempts
/// - `TOADSTOOL_RETRY_BACKOFF_MS`: Retry backoff duration in milliseconds
/// - `TOADSTOOL_ENABLE_WEBSOCKET`: Enable WebSocket real-time events (true/false)
///
/// # Example
///
/// ```rust
/// use toadstool_client::ClientConfig;
///
/// // Use default configuration (respects env vars)
/// let config = ClientConfig::default();
///
/// // Or customize programmatically
/// let config = ClientConfig {
///     base_url: "https://my-toadstool-server.com".to_string(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the ToadStool server
    pub base_url: String,

    /// HTTP request timeout
    ///
    /// Note: For full timeout configuration (connection, read, write), consider
    /// creating a more complete client config struct that uses TimeoutConfig base pattern.
    pub request_timeout: Duration,

    /// WebSocket connection timeout
    #[deprecated(
        since = "0.5.0",
        note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
    )]
    pub websocket_timeout: Duration,

    /// Maximum retry attempts
    ///
    /// Note: For full retry configuration (backoff, jitter), consider using
    /// `toadstool::config_bases::RetryConfig` for consistency.
    pub max_retries: u32,

    /// Retry backoff strategy (simple exponential backoff)
    ///
    /// Note: For more sophisticated retry logic with exponential backoff multiplier
    /// and jitter, use `toadstool::config_bases::RetryConfig`.
    pub retry_backoff: Duration,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Enable WebSocket real-time events
    #[deprecated(
        since = "0.5.0",
        note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
    )]
    pub enable_websocket: bool,

    /// Custom HTTP headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            base_url: std::env::var("TOADSTOOL_API_URL").unwrap_or_else(|_| {
                format!(
                    "http://{}:{}",
                    defaults::network::LOCALHOST,
                    defaults::network::API_PORT
                )
            }),
            request_timeout: Duration::from_millis(
                std::env::var("TOADSTOOL_REQUEST_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(defaults::timeouts::REQUEST_MS),
            ),
            websocket_timeout: Duration::from_millis(
                std::env::var("TOADSTOOL_WEBSOCKET_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(defaults::timeouts::CONNECTION_MS),
            ),
            max_retries: std::env::var("TOADSTOOL_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults::retries::MAX_ATTEMPTS),
            retry_backoff: Duration::from_millis(
                std::env::var("TOADSTOOL_RETRY_BACKOFF_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(defaults::retries::BACKOFF_MS),
            ),
            auth: None,
            enable_websocket: std::env::var("TOADSTOOL_ENABLE_WEBSOCKET")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            custom_headers: HashMap::new(),
        }
    }
}

impl ClientConfig {
    /// Build an API URL with the given endpoint (zero-copy optimization)
    #[cfg(test)]
    pub(crate) fn api_url(&self, endpoint: &str) -> String {
        format!("{}/api/v1/{}", self.base_url, endpoint)
    }
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// API key authentication
    ApiKey { key: String, header_name: String },

    /// Bearer token authentication
    BearerToken { token: String },

    /// Basic authentication
    Basic { username: String, password: String },

    /// Custom authentication
    Custom { headers: HashMap<String, String> },
}
