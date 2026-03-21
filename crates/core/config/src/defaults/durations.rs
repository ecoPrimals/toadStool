// SPDX-License-Identifier: AGPL-3.0-only
//! Helper functions to get Duration values from timeout constants

use super::timeouts;
use std::time::Duration;

/// Default execution timeout as Duration
#[must_use]
pub const fn execution() -> Duration {
    Duration::from_millis(timeouts::EXECUTION_MS)
}

/// Default health check interval as Duration
#[must_use]
pub const fn health_check() -> Duration {
    Duration::from_millis(timeouts::HEALTH_CHECK_MS)
}

/// Default connection timeout as Duration
#[must_use]
pub const fn connection() -> Duration {
    Duration::from_millis(timeouts::CONNECTION_MS)
}

/// Default request timeout as Duration
#[must_use]
pub const fn request() -> Duration {
    Duration::from_millis(timeouts::REQUEST_MS)
}

/// Default idle timeout as Duration
#[must_use]
pub const fn idle() -> Duration {
    Duration::from_millis(timeouts::IDLE_MS)
}

/// Default discovery timeout as Duration
#[must_use]
pub const fn discovery() -> Duration {
    Duration::from_millis(timeouts::DISCOVERY_MS)
}

/// Default discovery interval as Duration
#[must_use]
pub const fn discovery_interval() -> Duration {
    Duration::from_millis(timeouts::DISCOVERY_INTERVAL_MS)
}

/// Default keepalive timeout as Duration
#[must_use]
pub const fn keepalive() -> Duration {
    Duration::from_secs(timeouts::KEEPALIVE_SEC)
}
