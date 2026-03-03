// SPDX-License-Identifier: AGPL-3.0-or-later
//! Monitoring system type definitions
//!
//! Types and enums for the resource monitoring system

use serde::{Deserialize, Serialize};
use std::time::Duration;
use toadstool::ToadStoolError;

/// Monitoring granularity for high-precision resource tracking
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MonitoringGranularity {
    /// Sub-millisecond monitoring (100μs intervals) - for high-frequency trading, real-time systems
    SubMillisecond,
    /// Millisecond monitoring (1ms intervals) - for latency-sensitive applications
    Millisecond,
    /// High frequency (10ms intervals) - for interactive applications
    HighFrequency,
    /// Standard monitoring (100ms intervals) - for most applications
    Standard,
    /// Low frequency (1s intervals) - for background processes
    LowFrequency,
    /// Custom interval
    Custom(Duration),
}

impl MonitoringGranularity {
    #[must_use]
    pub fn to_duration(self) -> Duration {
        match self {
            MonitoringGranularity::SubMillisecond => Duration::from_micros(100),
            MonitoringGranularity::Millisecond => Duration::from_millis(1),
            MonitoringGranularity::HighFrequency => Duration::from_millis(10),
            MonitoringGranularity::Standard => Duration::from_millis(100),
            MonitoringGranularity::LowFrequency => Duration::from_secs(1),
            MonitoringGranularity::Custom(duration) => duration,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Monitoring granularity
    pub granularity: MonitoringGranularity,
    /// Enable network monitoring
    pub enable_network_monitoring: bool,
    /// Enable threshold monitoring
    pub enable_threshold_monitoring: bool,
    /// Threshold violation action
    pub threshold_action: ThresholdAction,
    /// Metrics retention duration
    pub metrics_retention: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            granularity: MonitoringGranularity::Standard,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Action to take when thresholds are exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdAction {
    /// Log the violation
    Log,
    /// Log and send alert
    Alert,
    /// Log, alert, and terminate process
    Terminate,
}

/// Resource monitoring error types
#[derive(Debug, Clone)]
pub enum ResourceMonitorError {
    ProcessNotRegistered(String),
    ProcessNotFound(String),
    CommandExecutionFailed(String),
    ParseError(String),
    PlatformNotSupported(String),
    ResourceLimitExceeded {
        process_id: String,
        resource_type: String,
        current_value: f64,
        limit: f64,
    },
    NetworkMonitoringNotAvailable,
    ThresholdViolation {
        workload_id: String,
        resource_type: String,
        current_value: f64,
        threshold: f64,
    },
    Other(String),
}

impl std::fmt::Display for ResourceMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceMonitorError::ProcessNotRegistered(id) => {
                write!(f, "Process not registered for monitoring: {id}")
            }
            ResourceMonitorError::ProcessNotFound(id) => {
                write!(f, "Process not found: {id}")
            }
            ResourceMonitorError::CommandExecutionFailed(msg) => {
                write!(f, "Command execution failed: {msg}")
            }
            ResourceMonitorError::ParseError(msg) => {
                write!(f, "Parse error: {msg}")
            }
            ResourceMonitorError::PlatformNotSupported(platform) => {
                write!(f, "Platform not supported: {platform}")
            }
            ResourceMonitorError::ResourceLimitExceeded {
                process_id,
                resource_type,
                current_value,
                limit,
            } => {
                write!(
                    f,
                    "Resource limit exceeded for {process_id}: {resource_type} current={current_value}, limit={limit}"
                )
            }
            ResourceMonitorError::NetworkMonitoringNotAvailable => {
                write!(f, "Network monitoring is not available on this platform")
            }
            ResourceMonitorError::ThresholdViolation {
                workload_id,
                resource_type,
                current_value,
                threshold,
            } => {
                write!(
                    f,
                    "Threshold violation for {workload_id}: {resource_type} current={current_value}, threshold={threshold}"
                )
            }
            ResourceMonitorError::Other(msg) => write!(f, "Other error: {msg}"),
        }
    }
}

impl std::error::Error for ResourceMonitorError {}

impl From<ResourceMonitorError> for ToadStoolError {
    fn from(err: ResourceMonitorError) -> Self {
        ToadStoolError::resource(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitoring_granularity_to_duration() {
        assert_eq!(
            MonitoringGranularity::SubMillisecond.to_duration(),
            Duration::from_micros(100)
        );
        assert_eq!(
            MonitoringGranularity::Millisecond.to_duration(),
            Duration::from_millis(1)
        );
        assert_eq!(
            MonitoringGranularity::HighFrequency.to_duration(),
            Duration::from_millis(10)
        );
        assert_eq!(
            MonitoringGranularity::Standard.to_duration(),
            Duration::from_millis(100)
        );
        assert_eq!(
            MonitoringGranularity::LowFrequency.to_duration(),
            Duration::from_secs(1)
        );
        let custom = Duration::from_millis(500);
        assert_eq!(MonitoringGranularity::Custom(custom).to_duration(), custom);
    }

    #[test]
    fn monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(matches!(
            config.granularity,
            MonitoringGranularity::Standard
        ));
        assert!(config.enable_network_monitoring);
        assert!(config.enable_threshold_monitoring);
        assert!(matches!(config.threshold_action, ThresholdAction::Log));
        assert_eq!(config.metrics_retention, Duration::from_secs(3600));
    }

    #[test]
    fn threshold_action_variants() {
        let _log = ThresholdAction::Log;
        let _alert = ThresholdAction::Alert;
        let _terminate = ThresholdAction::Terminate;
    }

    #[test]
    fn resource_monitor_error_display() {
        let err = ResourceMonitorError::ProcessNotRegistered("workload-1".to_string());
        assert!(err.to_string().contains("workload-1"));

        let err = ResourceMonitorError::ParseError("invalid format".to_string());
        assert!(err.to_string().contains("invalid format"));

        let err = ResourceMonitorError::PlatformNotSupported("freebsd".to_string());
        assert!(err.to_string().contains("freebsd"));

        let err = ResourceMonitorError::ThresholdViolation {
            workload_id: "w1".to_string(),
            resource_type: "CPU".to_string(),
            current_value: 5.0,
            threshold: 4.0,
        };
        assert!(err.to_string().contains("w1"));
        assert!(err.to_string().contains("CPU"));
    }

    #[test]
    fn resource_monitor_error_to_toadstool_error() {
        let err = ResourceMonitorError::ProcessNotRegistered("test".to_string());
        let toad_err: ToadStoolError = err.into();
        assert!(!toad_err.to_string().is_empty());
    }
}
