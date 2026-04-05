// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Standard timeout configuration for network operations
///
/// This provides a common set of timeout values that can be embedded
/// in various configuration structs using `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connection establishment timeout
    #[serde(default = "default_connection_timeout", with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Request/response timeout
    #[serde(default = "default_request_timeout", with = "humantime_serde")]
    pub request_timeout: Duration,

    /// Socket read timeout
    #[serde(default = "default_read_timeout", with = "humantime_serde")]
    pub read_timeout: Duration,

    /// Socket write timeout
    #[serde(default = "default_write_timeout", with = "humantime_serde")]
    pub write_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connection_timeout: default_connection_timeout(),
            request_timeout: default_request_timeout(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
        }
    }
}

const fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

const fn default_request_timeout() -> Duration {
    Duration::from_secs(60)
}

const fn default_read_timeout() -> Duration {
    Duration::from_secs(30)
}

const fn default_write_timeout() -> Duration {
    Duration::from_secs(30)
}
