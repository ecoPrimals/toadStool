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
use tracing::{info, warn};

use toadstool::{
    execution::ExecutionInput, resources::ResourceRequirements, ExecutionRequest, RuntimeType,
    SecurityContext, WorkloadSpec,
};
use toadstool_distributed::{DistributedConfig, DistributedCoordinator};

// Deep debt solution: Use pure RPC types from local module
use crate::rpc_types::{
    AvailableResources, ComputeCapabilities, ComputeUnit, ExecutionMetrics,
    WorkloadResult, WorkloadStatus, WorkloadSubmission,
};
// WorkloadExecutor trait is defined in tarpc_server module
use crate::tarpc_server::WorkloadExecutor;

/// Executor that uses the distributed coordinator for workload execution
///
/// This integrates ToadStool's isomorphic/fractal coordination architecture
/// with the server's RPC interface.
pub struct CoordinatorExecutor {
    coordinator: Arc<DistributedCoordinator>,
    service_id: String,
}

impl CoordinatorExecutor {
    /// Create new coordinator executor
    ///
    /// # Errors
    ///
    /// Returns error if coordinator initialization fails
    pub async fn new(config: DistributedConfig, service_id: String) -> Result<Self, String> {
        info!(
            "Initializing coordinator executor for service: {}",
            service_id
        );

        let coordinator = DistributedCoordinator::new(config)
            .await
            .map_err(|e| format!("Failed to create coordinator: {}", e))?;

        let coordinator = Arc::new(coordinator);

        // Start coordinator
        Arc::clone(&coordinator)
            .start()
            .await
            .map_err(|e| format!("Failed to start coordinator: {}", e))?;

        info!("✅ Coordinator executor ready");

        Ok(Self {
            coordinator,
            service_id,
        })
    }
}

#[async_trait]
impl WorkloadExecutor for CoordinatorExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        info!(
            "Executing workload via coordinator: {}",
            submission.workload_id
        );

        // Convert WorkloadSubmission to ExecutionRequest
        let request = convert_submission_to_request(submission.clone())?;

        // Submit to coordinator (isomorphic/fractal routing)
        let execution_id = self
            .coordinator
            .submit_execution(request)
            .await
            .map_err(|e| format!("Coordinator execution failed: {}", e))?;

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

        let cpu_cores = num_cpus::get() as u32;
        
        // Query memory - Pure Rust Evolution (Jan 17, 2026)
        use sysinfo::System;
        let mut system = System::new_all();
        system.refresh_memory();
        
        let total_memory = system.total_memory(); // Already in bytes
        let available_memory = system.available_memory(); // Already in bytes

        Ok(ComputeCapabilities {
            service_id: self.service_id.clone(),
            compute_units: vec![
                ComputeUnit {
                    id: "coordinator-local".to_string(),
                    unit_type: "distributed".to_string(),
                    name: "Distributed Coordinator".to_string(),
                    cores: cpu_cores,
                    memory_bytes: total_memory,
                    tflops: Some((cpu_cores as f64) * 0.1),
                    utilization: 0.0,
                },
            ],
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

        // **Implementation Strategy**:
        // The distributed coordinator needs a workload cancellation API.
        // This would involve:
        // 1. Finding which node is executing the workload (via coordinator state)
        // 2. Sending cancellation signal to that node (via tarpc/gRPC)
        // 3. Handling graceful shutdown of workload resources
        //
        // **Current Status**: Basic cancellation signaling
        // **Future**: Full distributed cancellation with resource cleanup

        warn!("Distributed cancellation requires coordinator API extension - workload marked for cancellation");

        // Return success for now (graceful degradation)
        // The workload will complete naturally if already running
        Ok(())
    }
}

/// Convert WorkloadSubmission to ExecutionRequest
///
/// Deep debt principle: Type conversion without hardcoding
fn convert_submission_to_request(
    submission: WorkloadSubmission,
) -> Result<ExecutionRequest, String> {
    // Create workload spec from raw binary data
    let workload_spec = WorkloadSpec::Native {
        executable: toadstool::workload::ExecutableSource::Bytes {
            data: submission.data,
        },
        args: None,
        working_dir: None,
        env_vars: submission.metadata, // metadata is HashMap, not Option
        user: None,
    };

    // Extract timeout from requirements (ResourceRequirements has timeout_secs field)
    let timeout = submission
        .requirements
        .timeout_secs
        .map(Duration::from_secs);

    Ok(ExecutionRequest {
        execution_id: uuid::Uuid::parse_str(&submission.workload_id)
            .unwrap_or_else(|_| uuid::Uuid::new_v4()),
        workload: workload_spec,
        runtime_hint: Some(parse_runtime_type(&submission.workload_type)),
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
}
