// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    pub socket_path: String,
    pub request_timeout_secs: u64,
    pub token_refresh_interval_secs: u64,
    pub zero_trust_validation_interval_secs: u64,
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
