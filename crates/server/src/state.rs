//! Server state management types and structures

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use toadstool::{ExecutionStatus, ResourceMonitor, RuntimeEngine, RuntimeType};

use crate::config::ServerConfig;

/// Server events for broadcasting
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// New execution started
    ExecutionStarted {
        execution_id: Uuid,
        runtime_type: RuntimeType,
        timestamp: DateTime<Utc>,
    },

    /// Execution completed
    ExecutionCompleted {
        execution_id: Uuid,
        status: ExecutionStatus,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// Runtime engine registered
    RuntimeEngineRegistered {
        runtime_type: RuntimeType,
        timestamp: DateTime<Utc>,
    },

    /// Resource usage update
    ResourceUsageUpdate {
        cpu_usage_percent: f64,
        memory_usage_percent: f64,
        active_executions: u32,
        timestamp: DateTime<Utc>,
    },

    /// Health status change
    HealthStatusChanged {
        healthy: bool,
        message: String,
        timestamp: DateTime<Utc>,
    },

    /// Error occurred
    ErrorOccurred {
        error_type: String,
        message: String,
        execution_id: Option<Uuid>,
        timestamp: DateTime<Utc>,
    },
}

/// Information about an active execution
#[derive(Debug, Clone)]
pub struct ActiveExecution {
    pub execution_id: Uuid,
    pub runtime_type: RuntimeType,
    pub started_at: DateTime<Utc>,
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

    /// Event broadcaster for WebSocket clients
    pub event_broadcaster: broadcast::Sender<ServerEvent>,

    /// Server configuration
    pub config: ServerConfig,

    /// Resource monitor
    pub resource_monitor: Arc<dyn ResourceMonitor>,

    /// Server statistics
    pub stats: Arc<RwLock<ServerStatistics>>,
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
