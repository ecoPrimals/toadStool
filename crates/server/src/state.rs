//! Server state management types and structures

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use toadstool::{ExecutionStatus, ResourceMonitor, RuntimeEngine, RuntimeType};

use crate::config::ServerConfig;

/// Helper to serialize SystemTime as Unix timestamp for JSON
pub(crate) fn timestamp_to_unix_secs(t: &SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Server events for broadcasting
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// New execution started
    ExecutionStarted {
        execution_id: Uuid,
        runtime_type: RuntimeType,
        timestamp: SystemTime,
    },

    /// Execution completed
    ExecutionCompleted {
        execution_id: Uuid,
        status: ExecutionStatus,
        duration_ms: u64,
        timestamp: SystemTime,
    },

    /// Runtime engine registered
    RuntimeEngineRegistered {
        runtime_type: RuntimeType,
        timestamp: SystemTime,
    },

    /// Resource usage update
    ResourceUsageUpdate {
        cpu_usage_percent: f64,
        memory_usage_percent: f64,
        active_executions: u32,
        timestamp: SystemTime,
    },

    /// Health status change
    HealthStatusChanged {
        healthy: bool,
        message: String,
        timestamp: SystemTime,
    },

    /// Error occurred
    ErrorOccurred {
        error_type: String,
        message: String,
        execution_id: Option<Uuid>,
        timestamp: SystemTime,
    },
}

impl ServerEvent {
    /// Serialize event to JSON string for transport.
    /// Used when events need to be forwarded (e.g. via JSON-RPC or logging).
    #[must_use]
    pub fn to_json(&self) -> String {
        match self {
            Self::ExecutionStarted {
                execution_id,
                runtime_type,
                timestamp,
            } => serde_json::json!({
                "type": "execution_started",
                "data": {
                    "execution_id": execution_id,
                    "runtime_type": runtime_type,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
            Self::ExecutionCompleted {
                execution_id,
                status,
                duration_ms,
                timestamp,
            } => serde_json::json!({
                "type": "execution_completed",
                "data": {
                    "execution_id": execution_id,
                    "status": status,
                    "duration_ms": duration_ms,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
            Self::RuntimeEngineRegistered {
                runtime_type,
                timestamp,
            } => serde_json::json!({
                "type": "runtime_engine_registered",
                "data": {
                    "runtime_type": runtime_type,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
            Self::ResourceUsageUpdate {
                cpu_usage_percent,
                memory_usage_percent,
                active_executions,
                timestamp,
            } => serde_json::json!({
                "type": "resource_usage_update",
                "data": {
                    "cpu_usage_percent": cpu_usage_percent,
                    "memory_usage_percent": memory_usage_percent,
                    "active_executions": active_executions,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
            Self::HealthStatusChanged {
                healthy,
                message,
                timestamp,
            } => serde_json::json!({
                "type": "health_status_changed",
                "data": {
                    "healthy": healthy,
                    "message": message,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
            Self::ErrorOccurred {
                error_type,
                message,
                execution_id,
                timestamp,
            } => serde_json::json!({
                "type": "error_occurred",
                "data": {
                    "error_type": error_type,
                    "message": message,
                    "execution_id": execution_id,
                    "timestamp": timestamp_to_unix_secs(timestamp),
                }
            })
            .to_string(),
        }
    }
}

/// Information about an active execution
#[derive(Debug, Clone)]
pub struct ActiveExecution {
    pub execution_id: Uuid,
    pub runtime_type: RuntimeType,
    pub started_at: SystemTime,
    pub timeout: Duration,
    pub status: ExecutionStatus,
    pub client_info: ClientInfo,
}

/// Client information for tracking
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub api_key: Option<String>,
    pub authenticated_user: Option<String>,
}

/// Server state container
#[derive(Clone)]
pub struct ServerState {
    /// Registered runtime engines
    pub runtime_engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,

    /// Active executions
    pub active_executions: Arc<RwLock<HashMap<Uuid, ActiveExecution>>>,

    /// Event broadcaster (real-time events via JSON-RPC 2.0 polling, no WebSocket)
    pub event_broadcaster: broadcast::Sender<ServerEvent>,

    /// Server configuration
    pub config: ServerConfig,

    /// Resource monitor
    pub resource_monitor: Arc<dyn ResourceMonitor>,

    /// Server statistics
    pub stats: Arc<RwLock<ServerStatistics>>,

    /// Capability provider for primal integration (optional)
    pub capability_provider:
        Option<Arc<toadstool_distributed::primal_capabilities::CapabilityProvider>>,
}

/// Server statistics tracking
#[derive(Debug, Clone)]
pub struct ServerStatistics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub peak_concurrent_executions: u32,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub errors_count: u64,
}

impl Default for ServerStatistics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time_ms: 0.0,
            peak_concurrent_executions: 0,
            uptime_seconds: 0,
            total_requests: 0,
            errors_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::ExecutionStatus;

    #[test]
    fn test_server_event_to_json_execution_started() {
        let event = ServerEvent::ExecutionStarted {
            execution_id: Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("execution_started"));
        assert!(json.contains("execution_id"));
    }

    #[test]
    fn test_server_event_to_json_execution_completed() {
        let event = ServerEvent::ExecutionCompleted {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            duration_ms: 1500,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("execution_completed"));
        assert!(json.contains("duration_ms"));
    }

    #[test]
    fn test_server_event_to_json_health_status_changed() {
        let event = ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "ok".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("health_status_changed"));
        assert!(json.contains("healthy"));
    }

    #[test]
    fn test_server_event_to_json_error_occurred() {
        let event = ServerEvent::ErrorOccurred {
            error_type: "Network".to_string(),
            message: "timeout".to_string(),
            execution_id: None,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("error_occurred"));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn test_server_statistics_default() {
        let stats = ServerStatistics::default();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.successful_executions, 0);
        assert_eq!(stats.failed_executions, 0);
        assert!((stats.average_execution_time_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.peak_concurrent_executions, 0);
        assert_eq!(stats.uptime_seconds, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.errors_count, 0);
    }

    #[test]
    fn test_client_info_fields() {
        let info = ClientInfo {
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test".to_string()),
            api_key: None,
            authenticated_user: Some("alice".to_string()),
        };
        assert_eq!(info.ip_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(info.authenticated_user.as_deref(), Some("alice"));
    }
}
