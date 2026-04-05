// SPDX-License-Identifier: AGPL-3.0-or-later
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
/// - `TOADSTOOL_SERVER_URL`: Base URL of the ToadStool server
/// - `TOADSTOOL_REQUEST_TIMEOUT_MS`: HTTP request timeout in milliseconds
/// - `TOADSTOOL_MAX_RETRIES`: Maximum retry attempts
/// - `TOADSTOOL_RETRY_BACKOFF_MS`: Retry backoff duration in milliseconds
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
    /// creating a more complete client config struct that uses `TimeoutConfig` base pattern.
    pub request_timeout: Duration,

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

    /// Custom HTTP headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            // `TOADSTOOL_SERVER_URL` overrides; default base is loopback + `defaults::network::API_PORT` (0 = OS-assigned).
            base_url: std::env::var("TOADSTOOL_SERVER_URL").unwrap_or_else(|_| {
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

/// Authentication configuration for the client.
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// API key in a custom header.
    ApiKey {
        /// API key value.
        key: String,
        /// HTTP header name (e.g. X-API-Key).
        header_name: String,
    },

    /// Bearer token authentication.
    BearerToken {
        /// Bearer token value.
        token: String,
    },

    /// HTTP Basic authentication.
    Basic {
        /// Username.
        username: String,
        /// Password.
        password: String,
    },

    /// Custom authentication headers.
    Custom {
        /// Headers to add to requests.
        headers: HashMap<String, String>,
    },
}
