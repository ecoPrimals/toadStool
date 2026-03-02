//! API handler constants
//!
//! Module-level constants for API handlers. These are intentionally kept separate
//! from the central config module as they are implementation details of the API layer.

/// Default node identifier for single-node deployments
///
/// Used when no specific node ID is available or in standalone mode.
/// In multi-node clusters, this will be overridden by the actual node ID.
pub const DEFAULT_NODE_ID: &str = "node-1";

/// Default runtime type for workload execution
///
/// Specifies "native" as the default execution environment when no specific
/// runtime is requested. Other options include: wasm, container, python, gpu, edge.
pub const DEFAULT_RUNTIME_TYPE: &str = "native";

/// Source identifier for execution tracking
///
/// Tags execution events and logs as originating from the executor component.
/// Used in distributed tracing and log aggregation.
pub const EXECUTOR_SOURCE: &str = "executor";

// ============================================================================
// Observability Metric Names
// ============================================================================

/// Metric name for workload execution duration in milliseconds
pub const METRIC_EXECUTION_DURATION: &str = "execution_duration_ms";

/// Metric name for CPU usage percentage (0-100)
pub const METRIC_CPU_USAGE: &str = "cpu_usage";

/// Metric name for memory usage in bytes
pub const METRIC_MEMORY_USAGE: &str = "memory_usage";

/// Metric name for disk usage in bytes
pub const METRIC_DISK_USAGE: &str = "disk_usage";

/// Metric name for network received bytes
pub const METRIC_NETWORK_RX: &str = "network_rx";

/// Metric name for network transmitted bytes
pub const METRIC_NETWORK_TX: &str = "network_tx";

/// Metric name for execution status tracking
pub const METRIC_EXECUTION_STATUS: &str = "execution_status";

// ============================================================================
// Response Templates
// ============================================================================

/// Default message for execution not found errors
pub const MSG_EXECUTION_NOT_FOUND: &str = "Execution not found";

/// Default message for successful health checks
pub const MSG_HEALTHY: &str = "Service is healthy";

/// Default message for service degradation
pub const MSG_DEGRADED: &str = "Service is degraded";

// ============================================================================
// Default Pagination
// ============================================================================

/// Default page size for paginated responses
pub const DEFAULT_PAGE_SIZE: usize = 50;

/// Maximum page size for paginated responses
pub const MAX_PAGE_SIZE: usize = 1000;

/// Default log tail size (lines)
pub const DEFAULT_LOG_TAIL: usize = 100;

/// Maximum log tail size (lines)
pub const MAX_LOG_TAIL: usize = 10000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_constants() {
        assert_eq!(DEFAULT_NODE_ID, "node-1");
        assert_eq!(DEFAULT_RUNTIME_TYPE, "native");
        assert_eq!(EXECUTOR_SOURCE, "executor");
        assert_eq!(MSG_EXECUTION_NOT_FOUND, "Execution not found");
        assert_eq!(MSG_HEALTHY, "Service is healthy");
        assert_eq!(MSG_DEGRADED, "Service is degraded");
    }

    #[test]
    fn test_metric_constants() {
        assert_eq!(METRIC_EXECUTION_DURATION, "execution_duration_ms");
        assert_eq!(METRIC_CPU_USAGE, "cpu_usage");
        assert_eq!(METRIC_MEMORY_USAGE, "memory_usage");
        assert_eq!(METRIC_DISK_USAGE, "disk_usage");
        assert_eq!(METRIC_NETWORK_RX, "network_rx");
        assert_eq!(METRIC_NETWORK_TX, "network_tx");
        assert_eq!(METRIC_EXECUTION_STATUS, "execution_status");
    }

    #[test]
    fn test_pagination_constants() {
        assert_eq!(DEFAULT_PAGE_SIZE, 50);
        assert_eq!(MAX_PAGE_SIZE, 1000);
        const { assert!(DEFAULT_PAGE_SIZE < MAX_PAGE_SIZE) };
    }

    #[test]
    fn test_log_constants() {
        assert_eq!(DEFAULT_LOG_TAIL, 100);
        assert_eq!(MAX_LOG_TAIL, 10000);
        const { assert!(DEFAULT_LOG_TAIL < MAX_LOG_TAIL) };
    }
}
