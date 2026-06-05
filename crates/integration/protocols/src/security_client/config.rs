// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security capability provider configuration (crypto / PKI).
//!
//! Socket discovery follows `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2:
//!   1. `BIOMEOS_CRYPTO_SOCKET` env var (capability-based, preferred)
//!   2. `BEARDOG_SOCKET` env var (legacy identity-based fallback)
//!   3. `$XDG_RUNTIME_DIR/biomeos/security.sock` (capability-domain socket)

use toadstool_common::interned_strings::socket_env;
use serde::{Deserialize, Serialize};

const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
const DEFAULT_TOKEN_REFRESH_INTERVAL_SECS: u64 = 300;
const DEFAULT_ZERO_TRUST_VALIDATION_INTERVAL_SECS: u64 = 60;

/// Capability-domain socket filename per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` Tier 3.
const CAPABILITY_SOCKET_FILENAME: &str = "security.sock";

/// Security capability provider configuration (crypto / PKI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Unix socket path for JSON-RPC (resolved via capability discovery)
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

impl Default for SecurityConfig {
    #[expect(deprecated, reason = "reads legacy BEARDOG_SOCKET as backward-compat fallback")]
    fn default() -> Self {
        let socket_path = match std::env::var(socket_env::BIOMEOS_CRYPTO_SOCKET) {
            Ok(path) => path,
            Err(_) => {
                if let Ok(path) = std::env::var(socket_env::LEGACY_BEARDOG_SOCKET_ENV) {
                    tracing::warn!(
                        env_var = %socket_env::LEGACY_BEARDOG_SOCKET_ENV,
                        value = %path,
                        "deprecated LEGACY env variable used — migrate to capability-based discovery"
                    );
                    path
                } else {
                    let runtime_dir = std::env::var(socket_env::XDG_RUNTIME_DIR)
                        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
                    format!("{runtime_dir}/biomeos/{CAPABILITY_SOCKET_FILENAME}")
                }
            }
        };

        Self {
            socket_path,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            token_refresh_interval_secs: DEFAULT_TOKEN_REFRESH_INTERVAL_SECS,
            zero_trust_validation_interval_secs: DEFAULT_ZERO_TRUST_VALIDATION_INTERVAL_SECS,
            continuous_monitoring: true,
        }
    }
}
