// SPDX-License-Identifier: AGPL-3.0-only
//! BearDog configuration for Unix socket communication
//!
//! Socket discovery uses capability-based resolution (`crypto` capability)
//! with env var and XDG fallbacks.

use serde::{Deserialize, Serialize};

const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
const DEFAULT_TOKEN_REFRESH_INTERVAL_SECS: u64 = 300;
const DEFAULT_ZERO_TRUST_VALIDATION_INTERVAL_SECS: u64 = 60;
const SOCKET_FILENAME: &str = "beardog.sock";

/// BearDog PKI security service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    /// Unix socket path for JSON-RPC
    pub socket_path: String,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Token refresh interval in seconds
    pub token_refresh_interval_secs: u64,
    /// Zero-trust validation interval in seconds
    pub zero_trust_validation_interval_secs: u64,
    /// Enable continuous background validation
    pub continuous_monitoring: bool,
}

impl Default for BearDogConfig {
    fn default() -> Self {
        let socket_path = std::env::var("BEARDOG_SOCKET").unwrap_or_else(|_| {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
            format!("{runtime_dir}/{SOCKET_FILENAME}")
        });

        Self {
            socket_path,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            token_refresh_interval_secs: DEFAULT_TOKEN_REFRESH_INTERVAL_SECS,
            zero_trust_validation_interval_secs: DEFAULT_ZERO_TRUST_VALIDATION_INTERVAL_SECS,
            continuous_monitoring: true,
        }
    }
}
