// SPDX-License-Identifier: AGPL-3.0-or-later
//! Health check types.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Health status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health
    pub healthy: bool,
    /// Health checks
    pub checks: Vec<HealthCheck>,
    /// Last health check timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_check: std::time::SystemTime,
}

/// Individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthCheckStatus,
    /// Check message
    pub message: Option<String>,
    /// Check duration
    pub duration: Duration,
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthCheckStatus {
    /// Check passed
    Healthy,
    /// Check failed
    Unhealthy,
    /// Check in progress
    Pending,
}
