// SPDX-License-Identifier: AGPL-3.0-only
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
    /// Convert granularity to sampling interval duration
    #[must_use]
    pub const fn to_duration(self) -> Duration {
        match self {
            Self::SubMillisecond => Duration::from_micros(100),
            Self::Millisecond => Duration::from_millis(1),
            Self::HighFrequency => Duration::from_millis(10),
            Self::Standard => Duration::from_millis(100),
            Self::LowFrequency => Duration::from_secs(1),
            Self::Custom(duration) => duration,
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
    /// Process was not registered for monitoring
    ProcessNotRegistered(String),
    /// Process not found in system
    ProcessNotFound(String),
    /// Command execution failed
    CommandExecutionFailed(String),
    /// Parse error reading metrics
    ParseError(String),
    /// Platform does not support this operation
    PlatformNotSupported(String),
    /// Resource limit exceeded
    ResourceLimitExceeded {
        /// Process identifier
        process_id: String,
        /// Resource type (CPU, memory, etc.)
        resource_type: String,
        /// Current usage value
        current_value: f64,
        /// Configured limit
        limit: f64,
    },
    /// Network monitoring unavailable on platform
    NetworkMonitoringNotAvailable,
    /// Threshold violation detected
    ThresholdViolation {
        /// Workload identifier
        workload_id: String,
        /// Resource type
        resource_type: String,
        /// Current value
        current_value: f64,
        /// Threshold exceeded
        threshold: f64,
    },
    /// Other error
    Other(String),
}

impl std::fmt::Display for ResourceMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProcessNotRegistered(id) => {
                write!(f, "Process not registered for monitoring: {id}")
            }
            Self::ProcessNotFound(id) => {
                write!(f, "Process not found: {id}")
            }
            Self::CommandExecutionFailed(msg) => {
                write!(f, "Command execution failed: {msg}")
            }
            Self::ParseError(msg) => {
                write!(f, "Parse error: {msg}")
            }
            Self::PlatformNotSupported(platform) => {
                write!(f, "Platform not supported: {platform}")
            }
            Self::ResourceLimitExceeded {
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
            Self::NetworkMonitoringNotAvailable => {
                write!(f, "Network monitoring is not available on this platform")
            }
            Self::ThresholdViolation {
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
            Self::Other(msg) => write!(f, "Other error: {msg}"),
        }
    }
}

impl std::error::Error for ResourceMonitorError {}

impl From<ResourceMonitorError> for ToadStoolError {
    fn from(err: ResourceMonitorError) -> Self {
        Self::resource(err.to_string())
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
        let log = ThresholdAction::Log;
        let alert = ThresholdAction::Alert;
        let terminate = ThresholdAction::Terminate;
        assert!(matches!(log, ThresholdAction::Log));
        assert!(matches!(alert, ThresholdAction::Alert));
        assert!(matches!(terminate, ThresholdAction::Terminate));
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
