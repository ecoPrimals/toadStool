//! Health check utilities for isomorphic IPC
//!
//! Provides health monitoring for display server endpoints
//! with automatic Unix/TCP adaptation.

use super::client::DisplayClient;
use crate::{DisplayError, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Health status for the display server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// Server is healthy and responding
    Healthy,
    /// Server is running but degraded (slow responses)
    Degraded,
    /// Server is not responding
    Unhealthy,
}

/// Detailed health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall health status
    pub status: HealthStatus,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Transport type used (Unix or TCP)
    pub transport: String,
    /// Server endpoint address
    pub endpoint: String,
    /// Window count (if available)
    pub window_count: Option<usize>,
    /// Server capabilities (if available)
    pub isomorphic: bool,
}

/// Perform health check on discovered display server
///
/// ## Isomorphic Operation
///
/// This function:
/// 1. Discovers endpoint (Unix or TCP)
/// 2. Connects using appropriate transport
/// 3. Sends health check request
/// 4. Returns detailed status
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool_display::ipc::health::check_display_health;
///
/// # async fn example() -> anyhow::Result<()> {
/// let result = check_display_health().await?;
/// println!("Health: {:?}, Response: {}ms", result.status, result.response_time_ms);
/// # Ok(())
/// # }
/// ```
pub async fn check_display_health() -> Result<HealthCheckResult> {
    // 1. Discover endpoint (isomorphic!)
    let mut client = DisplayClient::discover().await?;
    let endpoint = client.endpoint_string();
    let transport = client.transport_name().to_string();

    // 2. Send health check request (getCapabilities)
    let response_time = Instant::now();
    let capabilities = client.get_capabilities().await?;
    let response_time_ms = response_time.elapsed().as_millis() as u64;

    // 3. Determine health status based on response time
    let status = if response_time_ms < 100 {
        HealthStatus::Healthy
    } else if response_time_ms < 500 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    };

    // 4. Build result
    Ok(HealthCheckResult {
        status,
        response_time_ms,
        transport,
        endpoint,
        window_count: Some(capabilities.window_count),
        isomorphic: capabilities.isomorphic,
    })
}

/// Perform health check with timeout
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool_display::ipc::health::check_display_health_with_timeout;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let result = check_display_health_with_timeout(Duration::from_secs(5)).await?;
/// println!("Health check completed: {:?}", result.status);
/// # Ok(())
/// # }
/// ```
pub async fn check_display_health_with_timeout(timeout: Duration) -> Result<HealthCheckResult> {
    tokio::time::timeout(timeout, check_display_health())
        .await
        .map_err(|_| DisplayError::IpcError("Health check timeout".to_string()))?
}

/// Monitor display server health continuously
///
/// Performs periodic health checks and returns when status changes.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool_display::ipc::health::monitor_display_health;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// monitor_display_health(Duration::from_secs(10), |result| {
///     println!("Health: {:?}", result.status);
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn monitor_display_health<F>(interval: Duration, mut callback: F) -> Result<()>
where
    F: FnMut(&HealthCheckResult),
{
    let mut last_status = None;
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        match check_display_health().await {
            Ok(result) => {
                // Only callback if status changed
                if last_status.as_ref() != Some(&result.status) {
                    callback(&result);
                    last_status = Some(result.status);
                }
            }
            Err(e) => {
                tracing::error!("Health check failed: {}", e);
                if last_status != Some(HealthStatus::Unhealthy) {
                    let result = HealthCheckResult {
                        status: HealthStatus::Unhealthy,
                        response_time_ms: 0,
                        transport: "unknown".to_string(),
                        endpoint: "unknown".to_string(),
                        window_count: None,
                        isomorphic: false,
                    };
                    callback(&result);
                    last_status = Some(HealthStatus::Unhealthy);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_ordering() {
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
        assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_result_creation() {
        let result = HealthCheckResult {
            status: HealthStatus::Healthy,
            response_time_ms: 50,
            transport: "unix".to_string(),
            endpoint: "/tmp/test.sock".to_string(),
            window_count: Some(3),
            isomorphic: true,
        };

        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.response_time_ms, 50);
        assert!(result.isomorphic);
    }
}
