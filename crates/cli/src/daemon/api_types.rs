//! API types for ToadStool daemon HTTP API
//!
//! Request and response types for workload submission and management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version
pub const API_VERSION: &str = "v1";

/// Workload submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWorkloadRequest {
    /// biome.yaml manifest content
    pub biome_yaml: String,
    
    /// Requester identity (primal name or external client ID)
    pub requester: String,
    
    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,
    
    /// Resource requirements (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements>,
    
    /// Timeout in seconds (optional, defaults to 3600)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    
    /// Whether this is a persistent workload (keep running)
    #[serde(default)]
    pub persistent: bool,
}

/// Resource requirements for a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU limit (cores)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<f64>,
    
    /// Memory limit (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<u64>,
    
    /// GPU required
    #[serde(default)]
    pub gpu_required: bool,
    
    /// Storage limit (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_limit: Option<u64>,
}

/// Workload submission response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWorkloadResponse {
    /// Workload ID (UUID)
    pub workload_id: String,
    
    /// Status
    pub status: WorkloadStatus,
    
    /// Message
    pub message: String,
}

/// Workload status query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadStatusResponse {
    /// Workload ID
    pub workload_id: String,
    
    /// Status
    pub status: WorkloadStatus,
    
    /// Started at (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    
    /// Completed at (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    
    /// Exit code (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Resource usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<ResourceUsage>,
}

/// List workloads response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkloadsResponse {
    /// Workloads
    pub workloads: Vec<WorkloadSummary>,
    
    /// Total count
    pub total: usize,
}

/// Workload summary (for list view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSummary {
    /// Workload ID
    pub workload_id: String,
    
    /// Status
    pub status: WorkloadStatus,
    
    /// Requester
    pub requester: String,
    
    /// Started at (ISO 8601)
    pub started_at: String,
    
    /// Persistent
    pub persistent: bool,
}

/// Workload status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkloadStatus {
    /// Queued, waiting to start
    Queued,
    
    /// Running
    Running,
    
    /// Completed successfully
    Completed,
    
    /// Failed
    Failed,
    
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for WorkloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadStatus::Queued => write!(f, "queued"),
            WorkloadStatus::Running => write!(f, "running"),
            WorkloadStatus::Completed => write!(f, "completed"),
            WorkloadStatus::Failed => write!(f, "failed"),
            WorkloadStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage (percent)
    pub cpu_percent: f64,
    
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    
    /// GPU usage (percent, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_percent: Option<f64>,
    
    /// Storage used (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_bytes: Option<u64>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status (always "ok" if responding)
    pub status: String,
    
    /// Daemon version
    pub version: String,
    
    /// Uptime in seconds
    pub uptime_secs: u64,
    
    /// Active workloads count
    pub active_workloads: usize,
    
    /// biomeOS connected
    pub biomeos_connected: bool,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub error: String,
    
    /// Error message
    pub message: String,
    
    /// Additional details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_status_display() {
        assert_eq!(WorkloadStatus::Queued.to_string(), "queued");
        assert_eq!(WorkloadStatus::Running.to_string(), "running");
        assert_eq!(WorkloadStatus::Completed.to_string(), "completed");
        assert_eq!(WorkloadStatus::Failed.to_string(), "failed");
        assert_eq!(WorkloadStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_submit_request_serialization() {
        let req = SubmitWorkloadRequest {
            biome_yaml: "version: 1.0".to_string(),
            requester: "beardog".to_string(),
            environment: HashMap::new(),
            resources: None,
            timeout_secs: Some(3600),
            persistent: false,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        let parsed: SubmitWorkloadRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.biome_yaml, req.biome_yaml);
        assert_eq!(parsed.requester, req.requester);
    }
}

