// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Distributed Coordinator Executor Wrapper
//!
//! Integrates the `DistributedCoordinator` with the server's `WorkloadExecutor` trait.
//! This enables isomorphic/fractal ToadStool instance coordination while maintaining
//! a clean interface for the tarpc server.
//!
//! ## Deep Debt Principles
//!
//! - **Isomorphic Design**: ToadStool instances are identical, coordinate as peers
//! - **Fractal Architecture**: Each instance can coordinate or execute
//! - **Self-Knowledge Only**: Reports own capabilities, discovers others via Songbird
//! - **Capability-Based**: No hardcoded knowledge of other instances
//! - **Graceful Degradation**: Falls back to standalone if no coordinator available

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use toadstool::{
    ExecutionRequest, RuntimeType, SecurityContext, WorkloadSpec, execution::ExecutionInput,
    resources::ResourceRequirements,
};
use toadstool_distributed::{DistributedConfig, DistributedCoordinator};

// Deep debt solution: Use pure RPC types from local module
use crate::rpc_types::{
    AvailableResources, ComputeCapabilities, ComputeUnit, ExecutionMetrics, WorkloadResult,
    WorkloadStatus, WorkloadSubmission,
};
// WorkloadExecutor trait is defined in tarpc_server module
use crate::tarpc_server::WorkloadExecutor;

/// Executor that uses the distributed coordinator for workload execution
///
/// This integrates ToadStool's isomorphic/fractal coordination architecture
/// with the server's RPC interface.
pub struct CoordinatorExecutor {
    coordinator: Arc<DistributedCoordinator>,
    /// `Arc<str>` avoids allocation on hot-path `query_capabilities` clone
    service_id: Arc<str>,
}

impl CoordinatorExecutor {
    /// Create new coordinator executor
    ///
    /// # Errors
    ///
    /// Returns error if coordinator initialization fails
    pub async fn new(
        config: DistributedConfig,
        service_id: impl AsRef<str>,
    ) -> Result<Self, String> {
        let service_id = Arc::from(service_id.as_ref());
        info!(
            "Initializing coordinator executor for service: {}",
            service_id
        );

        let coordinator = DistributedCoordinator::new(config)
            .await
            .map_err(|e| format!("Failed to create coordinator: {e}"))?;

        let coordinator = Arc::new(coordinator);

        // Start coordinator
        Arc::clone(&coordinator)
            .start()
            .await
            .map_err(|e| format!("Failed to start coordinator: {e}"))?;

        info!("✅ Coordinator executor ready");

        Ok(Self {
            coordinator,
            service_id,
        })
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl WorkloadExecutor for CoordinatorExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        info!(
            "Executing workload via coordinator: {}",
            submission.workload_id.as_ref()
        );

        // Convert WorkloadSubmission to ExecutionRequest (pass by ref to avoid full clone)
        let request = convert_submission_to_request(&submission)?;

        // Submit to coordinator (isomorphic/fractal routing)
        let execution_id = self
            .coordinator
            .submit_execution(request)
            .await
            .map_err(|e| format!("Coordinator execution failed: {e}"))?;

        info!("Workload submitted to coordinator: {}", execution_id);

        // Return immediate result (async execution)
        Ok(WorkloadResult {
            workload_id: submission.workload_id,
            status: WorkloadStatus::Queued,
            data: None,
            error: None,
            metrics: ExecutionMetrics {
                queued_duration_secs: 0.0,
                execution_duration_secs: 0.0,
                cpu_cores_used: 0,
                memory_used_bytes: 0,
                gpu_memory_used_bytes: None,
            },
        })
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        info!("Querying coordinator capabilities (self-knowledge only)");

        // Query local capabilities only (not other instances)
        // The coordinator will report what THIS instance can do

        let cpu_cores = std::thread::available_parallelism()
            .map(|n| u32::try_from(n.get()).unwrap_or(4))
            .unwrap_or(4);

        let mem = toadstool_sysmon::memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
            total: 0,
            available: 0,
            used: 0,
            swap_total: 0,
            swap_free: 0,
        });
        let total_memory = mem.total;
        let available_memory = mem.available;

        Ok(ComputeCapabilities {
            service_id: self.service_id.as_ref().to_string(),
            compute_units: vec![ComputeUnit {
                id: "coordinator-local".to_string(),
                unit_type: "distributed".to_string(),
                name: "Distributed Coordinator".to_string(),
                cores: cpu_cores,
                memory_bytes: total_memory,
                tflops: Some((cpu_cores as f64) * 0.1),
                utilization: 0.0,
            }],
            supported_workload_types: vec![
                "cpu_compute".to_string(),
                "gpu_compute".to_string(),
                "neural_compute".to_string(),
                "distributed".to_string(),
            ],
            available_resources: AvailableResources {
                total_cpu_cores: cpu_cores,
                available_cpu_cores: cpu_cores,
                total_memory_bytes: total_memory,
                available_memory_bytes: available_memory,
                total_gpu_memory_bytes: None,
                available_gpu_memory_bytes: None,
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                gpu_utilization: None,
            },
            metadata: std::collections::HashMap::from([
                ("mode".to_string(), "distributed".to_string()),
                ("coordinator".to_string(), "active".to_string()),
            ]),
        })
    }

    async fn cancel(&self, workload_id: &str) -> Result<(), String> {
        info!(
            "Coordinator cancellation requested for workload: {}",
            workload_id
        );

        let execution_id = uuid::Uuid::parse_str(workload_id)
            .map_err(|e| format!("Invalid workload ID (expected UUID): {workload_id} - {e}"))?;

        self.coordinator
            .cancel_execution(execution_id)
            .await
            .map_err(|e| format!("Failed to cancel workload: {e}"))
    }
}

/// Convert WorkloadSubmission to ExecutionRequest
///
/// Deep debt principle: Type conversion without hardcoding
/// Takes reference to avoid cloning workload_id, workload_type, priority, requirements.
fn convert_submission_to_request(
    submission: &WorkloadSubmission,
) -> Result<ExecutionRequest, String> {
    // Create workload spec from raw binary data
    // Bytes::clone is cheap (refcount); metadata clone necessary for env_vars
    let workload_spec = WorkloadSpec::Native {
        executable: toadstool::workload::ExecutableSource::Bytes {
            data: submission.data.clone(),
        },
        args: None,
        working_dir: None,
        env_vars: submission.metadata.clone(),
        user: None,
    };

    // Extract timeout from requirements (ResourceRequirements has timeout_secs field)
    let timeout = submission
        .requirements
        .timeout_secs
        .map(Duration::from_secs);

    Ok(ExecutionRequest {
        execution_id: uuid::Uuid::parse_str(submission.workload_id.as_ref())
            .unwrap_or_else(|_| uuid::Uuid::new_v4()),
        workload: workload_spec,
        runtime_hint: Some(parse_runtime_type(submission.workload_type.as_ref())),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout,
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    })
}

/// Parse workload type string to RuntimeType hint
fn parse_runtime_type(s: &str) -> RuntimeType {
    match s {
        "native" | "cpu_compute" => RuntimeType::Native,
        "wasm" | "wasm_runtime" => RuntimeType::Wasm,
        "container" | "container_runtime" => RuntimeType::Container,
        "python" => RuntimeType::Python,
        "gpu" | "gpu_compute" => RuntimeType::Gpu,
        _ => RuntimeType::Native,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc_types::{ResourceRequirements, WorkloadPriority, WorkloadSubmission};

    #[test]
    fn test_runtime_type_parsing() {
        assert!(matches!(parse_runtime_type("native"), RuntimeType::Native));
        assert!(matches!(parse_runtime_type("wasm"), RuntimeType::Wasm));
        assert!(matches!(parse_runtime_type("gpu"), RuntimeType::Gpu));
        assert!(matches!(
            parse_runtime_type("cpu_compute"),
            RuntimeType::Native
        ));
    }

    #[test]
    fn test_runtime_type_parsing_all_variants() {
        assert!(matches!(parse_runtime_type("native"), RuntimeType::Native));
        assert!(matches!(
            parse_runtime_type("cpu_compute"),
            RuntimeType::Native
        ));
        assert!(matches!(parse_runtime_type("wasm"), RuntimeType::Wasm));
        assert!(matches!(
            parse_runtime_type("wasm_runtime"),
            RuntimeType::Wasm
        ));
        assert!(matches!(
            parse_runtime_type("container"),
            RuntimeType::Container
        ));
        assert!(matches!(
            parse_runtime_type("container_runtime"),
            RuntimeType::Container
        ));
        assert!(matches!(parse_runtime_type("python"), RuntimeType::Python));
        assert!(matches!(parse_runtime_type("gpu"), RuntimeType::Gpu));
        assert!(matches!(
            parse_runtime_type("gpu_compute"),
            RuntimeType::Gpu
        ));
    }

    #[test]
    fn test_runtime_type_parsing_unknown_defaults_to_native() {
        assert!(matches!(
            parse_runtime_type("neural_compute"),
            RuntimeType::Native
        ));
        assert!(matches!(parse_runtime_type("unknown"), RuntimeType::Native));
        assert!(matches!(parse_runtime_type(""), RuntimeType::Native));
    }

    #[test]
    fn test_convert_submission_to_request() {
        let submission = WorkloadSubmission {
            workload_id: Arc::from("550e8400-e29b-41d4-a716-446655440000"),
            workload_type: Arc::from("gpu_compute"),
            data: vec![1, 2, 3, 4, 5].into(),
            metadata: HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ]),
            priority: WorkloadPriority::High,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: Some(8 * 1024 * 1024 * 1024),
                timeout_secs: Some(300),
            },
        };

        let request =
            convert_submission_to_request(&submission).expect("Conversion should succeed");

        assert_eq!(
            request.execution_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert!(matches!(request.runtime_hint, Some(RuntimeType::Gpu)));
        assert!(request.timeout.is_some());
        assert_eq!(request.timeout.unwrap(), Duration::from_secs(300));

        if let toadstool::workload::WorkloadSpec::Native { env_vars, .. } = &request.workload {
            assert_eq!(env_vars.get("key1"), Some(&"value1".to_string()));
            assert_eq!(env_vars.get("key2"), Some(&"value2".to_string()));
        } else {
            panic!("Expected Native workload spec");
        }
    }

    #[test]
    fn test_convert_submission_invalid_uuid_uses_new_v4() {
        let submission = WorkloadSubmission {
            workload_id: Arc::from("not-a-valid-uuid"),
            workload_type: Arc::from("cpu_compute"),
            data: bytes::Bytes::new(),
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: None,
                memory_bytes: None,
                gpu_memory_bytes: None,
                timeout_secs: None,
            },
        };

        let request =
            convert_submission_to_request(&submission).expect("Conversion should succeed");

        // Should not panic - invalid UUID gets replaced with new_v4
        assert!(uuid::Uuid::parse_str(&request.execution_id.to_string()).is_ok());
    }

    #[test]
    fn test_convert_submission_neural_compute_defaults_to_native() {
        let submission = WorkloadSubmission {
            workload_id: Arc::from(uuid::Uuid::new_v4().to_string()),
            workload_type: Arc::from("neural_compute"),
            data: vec![1, 2, 3].into(),
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: None,
                memory_bytes: None,
                gpu_memory_bytes: None,
                timeout_secs: None,
            },
        };

        let request =
            convert_submission_to_request(&submission).expect("Conversion should succeed");
        assert!(matches!(request.runtime_hint, Some(RuntimeType::Native)));
    }

    #[test]
    fn test_convert_submission_empty_metadata() {
        let submission = WorkloadSubmission {
            workload_id: Arc::from(uuid::Uuid::new_v4().to_string()),
            workload_type: Arc::from("wasm"),
            data: bytes::Bytes::new(),
            metadata: HashMap::new(),
            priority: WorkloadPriority::Low,
            requirements: ResourceRequirements {
                cpu_cores: Some(1),
                memory_bytes: None,
                gpu_memory_bytes: None,
                timeout_secs: None,
            },
        };

        let request =
            convert_submission_to_request(&submission).expect("Conversion should succeed");
        assert!(matches!(request.runtime_hint, Some(RuntimeType::Wasm)));
    }

    #[tokio::test]
    async fn test_coordinator_executor_new() {
        let config = toadstool_distributed::DistributedConfig::default();
        let result = CoordinatorExecutor::new(config, "test-executor".to_string()).await;
        // May succeed in standalone mode or fail if distributed services unavailable
        if let Ok(executor) = result {
            let caps = executor
                .query_capabilities()
                .await
                .expect("Capabilities failed");
            assert_eq!(caps.service_id, "test-executor");
            assert_eq!(caps.metadata.get("mode"), Some(&"distributed".to_string()));
        }
    }
}
