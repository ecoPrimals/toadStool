// SPDX-License-Identifier: AGPL-3.0-or-later
//! BearDog configuration for Unix socket communication

use serde::{Deserialize, Serialize};

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
            let runtime_dir =
                std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
            format!("{runtime_dir}/beardog.sock")
        });

        Self {
            socket_path,
            request_timeout_secs: 30,
            token_refresh_interval_secs: 300,
            zero_trust_validation_interval_secs: 60,
            continuous_monitoring: true,
        }
    }
}
