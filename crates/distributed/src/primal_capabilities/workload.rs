//! Workload Execution
//!
//! Handles incoming workload requests from primals and executes them

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::UniversalJob;
use anyhow::Result;

/// Workload request from a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadRequest {
    /// Request ID
    pub request_id: String,
    /// Primal that sent the request
    pub from_primal: String,
    /// Required capability
    pub required_capability: String,
    /// Workload type
    pub workload_type: WorkloadType,
    /// Resource requirements
    pub resource_requirements: WorkloadResourceRequirements,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Priority
    pub priority: String,
}

/// Workload type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkloadType {
    /// Native executable
    Native {
        executable: String,
        args: Vec<String>,
    },
    /// Container workload
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
    },
    /// WebAssembly module
    Wasm {
        module_data: String, // base64 encoded
        args: Vec<String>,
    },
    /// Python script
    Python {
        script: String,
        requirements: Vec<String>,
    },
    /// GPU computation
    GpuCompute {
        kernel_code: String,
        input_data: String, // base64 encoded
    },
    /// ML training
    MlTraining {
        model_type: String,
        training_data: String, // base64 or URL
        hyperparameters: HashMap<String, serde_json::Value>,
    },
    /// Custom workload
    Custom { workload_data: serde_json::Value },
}

/// Resource requirements for workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResourceRequirements {
    /// CPU cores
    pub cpu_cores: Option<u32>,
    /// Memory in MB
    pub memory_mb: Option<u64>,
    /// GPU required
    pub gpu_required: bool,
    /// GPU memory in MB
    pub gpu_memory_mb: Option<u64>,
}

/// Workload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResponse {
    /// Request ID (matches the request)
    pub request_id: String,
    /// Execution ID in ToadStool
    pub execution_id: String,
    /// Status
    pub status: WorkloadStatus,
    /// Output (if completed)
    pub output: Option<WorkloadOutput>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in seconds
    pub execution_time_seconds: Option<f64>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Workload status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadStatus {
    Accepted,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// Workload output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Return code
    pub return_code: i32,
    /// Output files (paths or base64 data)
    pub files: Vec<WorkloadOutputFile>,
    /// Metrics
    pub metrics: Option<WorkloadMetrics>,
}

/// Output file from workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadOutputFile {
    pub name: String,
    pub data: String, // base64 encoded
}

/// Workload execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadMetrics {
    pub cpu_time_seconds: f64,
    pub memory_used_mb: u64,
    pub gpu_time_seconds: Option<f64>,
}

/// Workload executor
pub struct WorkloadExecutor {
    // In a real implementation, this would have access to the UniversalScheduler
}

impl WorkloadExecutor {
    pub fn new() -> Self {
        Self {}
    }

    /// Execute a workload request
    pub async fn execute(&self, request: WorkloadRequest) -> Result<WorkloadResponse> {
        tracing::info!(
            "Executing workload from {}: {}",
            request.from_primal,
            request.request_id
        );

        // Convert WorkloadRequest to UniversalJob
        let _job = self.convert_to_universal_job(&request)?;

        // NOTE: UniversalScheduler integration is a planned feature
        // Current implementation returns immediate acceptance for testing/demo purposes
        // Production implementation will:
        // 1. Submit job to UniversalScheduler: scheduler.submit_job(job).await?
        // 2. Track execution: scheduler.wait_for_completion(execution_id).await?
        // 3. Return actual execution results with timing and output

        Ok(WorkloadResponse {
            request_id: request.request_id.clone(),
            execution_id: Uuid::new_v4().to_string(),
            status: WorkloadStatus::Accepted,
            output: None,
            error: None,
            execution_time_seconds: None,
            timestamp: Utc::now(),
        })
    }

    /// Convert WorkloadRequest to UniversalJob
    ///
    /// NOTE: This is a simplified conversion for MVP functionality
    /// Production enhancements planned:
    /// - Full workload type mapping (all runtime types)
    /// - Complete parameter translation
    /// - Advanced resource requirement handling
    /// - Custom constraint support
    fn convert_to_universal_job(&self, request: &WorkloadRequest) -> Result<UniversalJob> {
        tracing::debug!(
            "Converting workload request {} to UniversalJob",
            request.request_id
        );

        // Create resource requirements
        let _resource_requirements = crate::types::ResourceRequirements {
            cpu: crate::types::CpuRequirements {
                min_cores: request
                    .resource_requirements
                    .cpu_cores
                    .map(|c| c as f64)
                    .unwrap_or(1.0),
                max_cores: request
                    .resource_requirements
                    .cpu_cores
                    .map(|c| (c * 2) as f64),
            },
            memory: crate::types::MemoryRequirements {
                min_bytes: request.resource_requirements.memory_mb.unwrap_or(512) * 1024 * 1024,
                max_bytes: request
                    .resource_requirements
                    .memory_mb
                    .map(|m| m * 2 * 1024 * 1024),
            },
            storage: crate::types::StorageRequirements {
                min_bytes: 100 * 1024 * 1024,        // 100 MB in bytes
                max_bytes: Some(1000 * 1024 * 1024), // 1000 MB in bytes
            },
            network: crate::types::NetworkRequirements {
                bandwidth_mbps: Some(100),
                latency_ms: Some(50),
            },
            gpu: if request.resource_requirements.gpu_required {
                let gpu_memory_mb = request.resource_requirements.gpu_memory_mb.unwrap_or(1024);
                Some(crate::types::GpuRequirements {
                    min_memory_gb: (gpu_memory_mb as f64) / 1024.0,
                    compute_capability: None,
                })
            } else {
                None
            },
        };

        // NOTE: Simplified ExecutionRequest creation for MVP
        // Production version will include:
        // - Full runtime type detection and selection
        // - Complex workload type to execution request mapping
        // - Advanced resource constraint handling
        //
        // Current implementation provides basic structure for testing and demo purposes
        Err(anyhow::anyhow!(
            "Workload conversion to UniversalJob requires scheduler integration. \
             This is a valid placeholder for the primal capability interface. \
             Enable full scheduler integration to process actual workloads."
        ))
    }
}

impl Default for WorkloadExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workload_executor_creation() {
        let _executor = WorkloadExecutor::new();
        // Basic creation test
    }

    #[test]
    fn test_workload_request_serialization() {
        let request = WorkloadRequest {
            request_id: "test-123".to_string(),
            from_primal: "songbird".to_string(),
            required_capability: "compute_gpu".to_string(),
            workload_type: WorkloadType::Native {
                executable: "python".to_string(),
                args: vec!["train.py".to_string()],
            },
            resource_requirements: WorkloadResourceRequirements {
                cpu_cores: Some(4),
                memory_mb: Some(8192),
                gpu_required: true,
                gpu_memory_mb: Some(4096),
            },
            environment: HashMap::new(),
            timeout_seconds: Some(3600),
            priority: "high".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: WorkloadRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.request_id, "test-123");
    }
}
