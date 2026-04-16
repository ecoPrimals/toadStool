// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consolidated workload executor doubles for tests (see [`crate::tarpc_server::WorkloadExecutorDispatch`]).
//!
//! Not used by production server startup paths; kept for unit and integration tests
//! that need deterministic or error-injecting behavior.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use super::WorkloadExecutor;
use crate::rpc_types::{
    AvailableResources, ComputeCapabilities, ExecutionMetrics, WorkloadResult, WorkloadStatus,
    WorkloadSubmission,
};

/// Test-only executor behavior (wrapped by [`crate::tarpc_server::WorkloadExecutorDispatch::TestDouble`]).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum TestWorkloadDouble {
    /// Same behavior as the historical `MockExecutor` in `tarpc_server` tests.
    Mock,
    /// Unit-test `FailingExecutor`: execute/capabilities fail; cancel succeeds.
    FailingUnit,
    /// Integration-test `FailingExecutor`: distinct capability/cancel error strings.
    FailingIntegration,
    /// Unit nested `QueuedExecutor` (cpu_cores_used = 1, service_id `queued`).
    QueuedUnit,
    /// Integration `QueuedExecutor` (cpu_cores_used = 0, service_id `queued-test`).
    QueuedIntegration,
    /// Returns `Running` workloads; cancel fails with a fixed message.
    CancelFailing,
    /// Monotonic tag bytes in result data (duplicate-id map tests).
    SeqTag(Arc<AtomicU8>),
    /// Workloads stay in `Running` (health active/queued split tests).
    Running,
}

impl WorkloadExecutor for TestWorkloadDouble {
    fn execute(
        &self,
        submission: WorkloadSubmission,
    ) -> impl Future<Output = Result<WorkloadResult, String>> + Send + '_ {
        let kind = self.clone();
        async move {
            match kind {
                Self::Mock => Ok(WorkloadResult {
                    workload_id: submission.workload_id,
                    status: WorkloadStatus::Completed,
                    data: Some(submission.data.clone()),
                    error: None,
                    metrics: ExecutionMetrics {
                        queued_duration_secs: 0.0,
                        execution_duration_secs: 0.1,
                        cpu_cores_used: 1,
                        memory_used_bytes: u64::try_from(submission.data.len()).unwrap_or(u64::MAX),
                        gpu_memory_used_bytes: None,
                    },
                }),
                Self::FailingUnit | Self::FailingIntegration => Err("executor failed".to_string()),
                Self::QueuedUnit => Ok(WorkloadResult {
                    workload_id: submission.workload_id,
                    status: WorkloadStatus::Queued,
                    data: None,
                    error: None,
                    metrics: ExecutionMetrics {
                        queued_duration_secs: 0.0,
                        execution_duration_secs: 0.0,
                        cpu_cores_used: 1,
                        memory_used_bytes: 0,
                        gpu_memory_used_bytes: None,
                    },
                }),
                Self::QueuedIntegration => Ok(WorkloadResult {
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
                }),
                Self::CancelFailing => Ok(WorkloadResult {
                    workload_id: submission.workload_id,
                    status: WorkloadStatus::Running,
                    data: None,
                    error: None,
                    metrics: ExecutionMetrics {
                        queued_duration_secs: 0.0,
                        execution_duration_secs: 0.0,
                        cpu_cores_used: 1,
                        memory_used_bytes: 0,
                        gpu_memory_used_bytes: None,
                    },
                }),
                Self::SeqTag(counter) => {
                    let tag = counter.fetch_add(1, Ordering::SeqCst);
                    Ok(WorkloadResult {
                        workload_id: submission.workload_id,
                        status: WorkloadStatus::Completed,
                        data: Some(vec![tag].into()),
                        error: None,
                        metrics: ExecutionMetrics {
                            queued_duration_secs: 0.0,
                            execution_duration_secs: 0.01,
                            cpu_cores_used: 1,
                            memory_used_bytes: 1,
                            gpu_memory_used_bytes: None,
                        },
                    })
                }
                Self::Running => Ok(WorkloadResult {
                    workload_id: submission.workload_id,
                    status: WorkloadStatus::Running,
                    data: None,
                    error: None,
                    metrics: ExecutionMetrics {
                        queued_duration_secs: 0.0,
                        execution_duration_secs: 0.0,
                        cpu_cores_used: 2,
                        memory_used_bytes: 0,
                        gpu_memory_used_bytes: None,
                    },
                }),
            }
        }
    }

    fn query_capabilities(
        &self,
    ) -> impl Future<Output = Result<ComputeCapabilities, String>> + Send + '_ {
        let kind = self.clone();
        async move {
            match kind {
                Self::Mock => Ok(ComputeCapabilities {
                    service_id: "mock".to_string(),
                    compute_units: vec![],
                    supported_workload_types: vec!["cpu_compute".to_string()],
                    available_resources: AvailableResources {
                        total_cpu_cores: 4,
                        available_cpu_cores: 4,
                        total_memory_bytes: 8_000_000_000,
                        available_memory_bytes: 4_000_000_000,
                        total_gpu_memory_bytes: None,
                        available_gpu_memory_bytes: None,
                        cpu_utilization: 0.0,
                        memory_utilization: 50.0,
                        gpu_utilization: None,
                    },
                    metadata: std::collections::HashMap::new(),
                }),
                Self::FailingUnit => Err("capabilities failed".to_string()),
                Self::FailingIntegration => Err("capabilities unavailable".to_string()),
                Self::QueuedUnit => Ok(ComputeCapabilities {
                    service_id: "queued".to_string(),
                    compute_units: vec![],
                    supported_workload_types: vec!["cpu_compute".to_string()],
                    available_resources: AvailableResources {
                        total_cpu_cores: 4,
                        available_cpu_cores: 4,
                        total_memory_bytes: 8_000_000_000,
                        available_memory_bytes: 4_000_000_000,
                        total_gpu_memory_bytes: None,
                        available_gpu_memory_bytes: None,
                        cpu_utilization: 0.0,
                        memory_utilization: 50.0,
                        gpu_utilization: None,
                    },
                    metadata: std::collections::HashMap::new(),
                }),
                Self::QueuedIntegration => Ok(ComputeCapabilities {
                    service_id: "queued-test".to_string(),
                    compute_units: vec![],
                    supported_workload_types: vec![],
                    available_resources: AvailableResources {
                        total_cpu_cores: 1,
                        available_cpu_cores: 1,
                        total_memory_bytes: 1024,
                        available_memory_bytes: 1024,
                        total_gpu_memory_bytes: None,
                        available_gpu_memory_bytes: None,
                        cpu_utilization: 0.0,
                        memory_utilization: 0.0,
                        gpu_utilization: None,
                    },
                    metadata: std::collections::HashMap::new(),
                }),
                Self::CancelFailing => Ok(ComputeCapabilities {
                    service_id: "cancel-fail".to_string(),
                    compute_units: vec![],
                    supported_workload_types: vec!["cpu_compute".to_string()],
                    available_resources: AvailableResources {
                        total_cpu_cores: 4,
                        available_cpu_cores: 4,
                        total_memory_bytes: 8_000_000_000,
                        available_memory_bytes: 4_000_000_000,
                        total_gpu_memory_bytes: None,
                        available_gpu_memory_bytes: None,
                        cpu_utilization: 0.0,
                        memory_utilization: 50.0,
                        gpu_utilization: None,
                    },
                    metadata: std::collections::HashMap::new(),
                }),
                Self::SeqTag(_) => Err("unused".to_string()),
                Self::Running => Ok(ComputeCapabilities {
                    service_id: "running-test".to_string(),
                    compute_units: vec![],
                    supported_workload_types: vec![],
                    available_resources: AvailableResources {
                        total_cpu_cores: 1,
                        available_cpu_cores: 1,
                        total_memory_bytes: 1024,
                        available_memory_bytes: 1024,
                        total_gpu_memory_bytes: None,
                        available_gpu_memory_bytes: None,
                        cpu_utilization: 0.0,
                        memory_utilization: 0.0,
                        gpu_utilization: None,
                    },
                    metadata: std::collections::HashMap::new(),
                }),
            }
        }
    }

    fn cancel<'a>(
        &'a self,
        workload_id: &'a str,
    ) -> impl Future<Output = Result<(), String>> + Send + 'a {
        let kind = self.clone();
        async move {
            match kind {
                Self::Mock | Self::QueuedUnit | Self::QueuedIntegration | Self::SeqTag(_) => {
                    let _ = workload_id;
                    Ok(())
                }
                Self::FailingUnit => Ok(()),
                Self::FailingIntegration => Err("cancel failed".to_string()),
                Self::CancelFailing => Err("cancel failed".to_string()),
                Self::Running => Ok(()),
            }
        }
    }
}
