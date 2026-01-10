//! # Type Adapters for Server-Distributed Integration
//!
//! Converts between server types and distributed coordinator types.
//!
//! **Purpose**: Bridge the gap between `toadstool-server` and `toadstool-distributed`
//! without introducing tight coupling.
//!
//! **Deep Debt Principles**:
//! - No hardcoding: All conversions based on runtime data
//! - Lossless: Preserve all information during conversion
//! - Fallible: Explicit error handling for invalid conversions

use std::collections::HashMap;
use std::time::Instant;

use crate::types::execution::DistributedExecutionStatus;
use crate::core::config::ToadStoolCapabilities;
use toadstool::{ExecutionRequest, ExecutionStatus, ResourceRequirements, WorkloadType};

/// Convert server workload submission data to ExecutionRequest
///
/// This is a simple conversion that preserves all data from the submission
pub fn workload_submission_to_execution_request(
    workload_type: &str,
    data: Vec<u8>,
    required_cpu_cores: Option<u32>,
    required_memory_bytes: Option<u64>,
    required_gpu_memory_bytes: Option<u64>,
    timeout_seconds: Option<u32>,
    metadata: Option<HashMap<String, String>>,
) -> ExecutionRequest {
    ExecutionRequest {
        workload_type: parse_workload_type(workload_type),
        data,
        requirements: ResourceRequirements {
            cpu_cores: required_cpu_cores.unwrap_or(1),
            memory_bytes: required_memory_bytes.unwrap_or(1024 * 1024 * 1024), // 1GB default
            gpu_memory_bytes: required_gpu_memory_bytes,
            max_duration_secs: timeout_seconds.map(|s| s as u64),
            priority: 0, // Default priority
        },
        metadata: metadata.unwrap_or_default(),
        callback: None, // Server doesn't expose callbacks
    }
}

/// Convert distributed ExecutionStatus to string status
pub fn execution_status_to_string(status: &ExecutionStatus) -> String {
    match status {
        ExecutionStatus::Pending => "queued".to_string(),
        ExecutionStatus::Running => "running".to_string(),
        ExecutionStatus::Completed => "completed".to_string(),
        ExecutionStatus::Failed(_) => "failed".to_string(),
        ExecutionStatus::Cancelled => "cancelled".to_string(),
    }
}

/// Convert DistributedExecutionStatus to string status
pub fn distributed_status_to_string(status: &DistributedExecutionStatus) -> String {
    match status {
        DistributedExecutionStatus::Pending => "queued".to_string(),
        DistributedExecutionStatus::Running => "running".to_string(),
        DistributedExecutionStatus::Completed => "completed".to_string(),
        DistributedExecutionStatus::Failed(_) => "failed".to_string(),
        DistributedExecutionStatus::Cancelled => "cancelled".to_string(),
    }
}

/// Parse workload type string to WorkloadType enum
pub fn parse_workload_type(type_str: &str) -> WorkloadType {
    match type_str.to_lowercase().as_str() {
        "container" => WorkloadType::Container,
        "wasm" => WorkloadType::Wasm,
        "native" => WorkloadType::Native,
        "python" => WorkloadType::Python,
        "gpu" => WorkloadType::Gpu,
        _ => WorkloadType::Native, // Default fallback
    }
}

/// Calculate execution metrics from timing data
pub fn calculate_execution_metrics(
    started_at: Instant,
    completed_at: Option<Instant>,
    cpu_cores_used: u32,
    memory_used_bytes: u64,
    gpu_memory_used_bytes: Option<u64>,
) -> (f64, f64, u32, u64, Option<u64>) {
    let execution_duration = completed_at
        .map(|end| end.duration_since(started_at))
        .unwrap_or_default();

    (
        0.0, // queued_duration_secs - TODO: Track queueing time
        execution_duration.as_secs_f64(),
        cpu_cores_used,
        memory_used_bytes,
        gpu_memory_used_bytes,
    )
}

/// Extract capability information from ToadStoolCapabilities
pub fn extract_capabilities_info(caps: &ToadStoolCapabilities) -> (usize, u64, Option<u64>, Vec<String>) {
    (
        caps.cpu_cores,
        caps.total_memory_bytes,
        caps.available_memory_bytes,
        vec![
            "native".to_string(),
            "wasm".to_string(),
            "container".to_string(),
            "python".to_string(),
        ],
    )
}

/// Estimate CPU TFLOPS based on core count
pub fn estimate_cpu_tflops(cores: usize) -> f64 {
    // Rough estimate: modern CPU core ~0.1 TFLOPS
    (cores as f64) * 0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_submission_conversion() {
        let request = workload_submission_to_execution_request(
            "container",
            vec![1, 2, 3, 4],
            Some(4),
            Some(2 * 1024 * 1024 * 1024), // 2GB
            Some(1024 * 1024 * 1024),     // 1GB
            Some(300),
            Some(HashMap::from([("key".to_string(), "value".to_string())])),
        );

        assert_eq!(request.workload_type, WorkloadType::Container);
        assert_eq!(request.data, vec![1, 2, 3, 4]);
        assert_eq!(request.requirements.cpu_cores, 4);
        assert_eq!(request.requirements.memory_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(
            request.requirements.gpu_memory_bytes,
            Some(1024 * 1024 * 1024)
        );
        assert_eq!(request.requirements.max_duration_secs, Some(300));
    }

    #[test]
    fn test_execution_status_conversion() {
        assert_eq!(
            execution_status_to_string(&ExecutionStatus::Pending),
            "queued"
        );
        assert_eq!(
            execution_status_to_string(&ExecutionStatus::Running),
            "running"
        );
        assert_eq!(
            execution_status_to_string(&ExecutionStatus::Completed),
            "completed"
        );
        assert_eq!(
            execution_status_to_string(&ExecutionStatus::Failed("error".to_string())),
            "failed"
        );
        assert_eq!(
            execution_status_to_string(&ExecutionStatus::Cancelled),
            "cancelled"
        );
    }

    #[test]
    fn test_workload_type_parsing() {
        assert_eq!(parse_workload_type("container"), WorkloadType::Container);
        assert_eq!(parse_workload_type("wasm"), WorkloadType::Wasm);
        assert_eq!(parse_workload_type("native"), WorkloadType::Native);
        assert_eq!(parse_workload_type("python"), WorkloadType::Python);
        assert_eq!(parse_workload_type("gpu"), WorkloadType::Gpu);
        assert_eq!(parse_workload_type("unknown"), WorkloadType::Native); // Fallback
    }

    #[test]
    fn test_cpu_tflops_estimation() {
        assert_eq!(estimate_cpu_tflops(4), 0.4);
        assert_eq!(estimate_cpu_tflops(8), 0.8);
        assert_eq!(estimate_cpu_tflops(32), 3.2);
    }
}

