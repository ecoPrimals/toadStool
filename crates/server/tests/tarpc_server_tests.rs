// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Integration tests for ToadStool tarpc server

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use tarpc::context::Context;

use toadstool_server::rpc_types::{
    AvailableResources, ComputeCapabilities, ExecutionMetrics, ResourceRequirements,
    ToadStoolComputeRpc, WorkloadPriority, WorkloadResult, WorkloadStatus, WorkloadSubmission,
};
use toadstool_server::tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};

/// Mock executor that fails on execute for testing error paths
struct FailingExecutor;

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl WorkloadExecutor for FailingExecutor {
    async fn execute(&self, _submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        Err("executor failed".to_string())
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        Err("capabilities unavailable".to_string())
    }

    async fn cancel(&self, _workload_id: &str) -> Result<(), String> {
        Err("cancel failed".to_string())
    }
}

fn sample_submission(workload_id: &str) -> WorkloadSubmission {
    WorkloadSubmission {
        workload_id: Arc::from(workload_id),
        workload_type: Arc::from("cpu_compute"),
        data: vec![1, 2, 3].into(),
        metadata: std::collections::HashMap::new(),
        priority: WorkloadPriority::Normal,
        requirements: ResourceRequirements {
            cpu_cores: Some(2),
            memory_bytes: Some(1024),
            gpu_memory_bytes: None,
            timeout_secs: Some(60),
        },
    }
}

#[tokio::test]
async fn test_server_creation() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);

    let health = server
        .health_check(Context::current())
        .await
        .expect("Health check failed");
    assert_eq!(health.version, "0.1.0");
    assert_eq!(health.active_workloads, 0);
}

#[tokio::test]
async fn test_server_creation_with_error_count() {
    let error_count = Arc::new(AtomicU64::new(0));
    let executor = Arc::new(StandaloneExecutor::new());
    let _server = ToadStoolTarpcServer::new("0.2.0", executor, Some(Arc::clone(&error_count)));
    assert_eq!(error_count.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn test_health_check() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);

    let health = server
        .health_check(Context::current())
        .await
        .expect("Health check failed");

    assert!(health.healthy);
    assert_eq!(health.version, "0.1.0");
    assert_eq!(health.active_workloads, 0);
}

#[tokio::test]
async fn test_submit_workload_success() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    let submission = sample_submission("work-001");

    let result = server
        .submit_workload(Context::current(), submission.clone())
        .await
        .expect("Submit should succeed");

    assert_eq!(result.workload_id.as_ref(), "work-001");
    assert!(matches!(result.status, WorkloadStatus::Completed));
    assert!(result.data.is_some());
}

#[tokio::test]
async fn test_submit_workload_executor_error() {
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new(
        "0.1.0",
        Arc::new(FailingExecutor),
        Some(Arc::clone(&error_count)),
    );
    let submission = sample_submission("work-fail");

    let result = server.submit_workload(Context::current(), submission).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "executor failed");
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_query_status_found() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    let submission = sample_submission("work-query");
    server
        .clone()
        .submit_workload(Context::current(), submission)
        .await
        .expect("Submit failed");

    let result = server
        .query_status(Context::current(), "work-query".to_string())
        .await
        .expect("Query should find workload");

    assert_eq!(result.workload_id.as_ref(), "work-query");
}

#[tokio::test]
async fn test_query_status_not_found() {
    let executor = Arc::new(StandaloneExecutor::new());
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new("0.1.0", executor, Some(Arc::clone(&error_count)));

    let result = server
        .query_status(Context::current(), "nonexistent-work".to_string())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Workload not found"));
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_cancel_workload() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    let submission = sample_submission("work-cancel");
    server
        .clone()
        .submit_workload(Context::current(), submission)
        .await
        .expect("Submit failed");

    let result = server
        .clone()
        .cancel_workload(Context::current(), "work-cancel".to_string())
        .await;
    assert!(result.is_ok());

    let status = server
        .query_status(Context::current(), "work-cancel".to_string())
        .await
        .expect("Should still find workload");
    assert!(matches!(status.status, WorkloadStatus::Cancelled));
}

#[tokio::test]
async fn test_cancel_workload_executor_error() {
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new(
        "0.1.0",
        Arc::new(FailingExecutor),
        Some(Arc::clone(&error_count)),
    );

    let result = server
        .cancel_workload(Context::current(), "work-x".to_string())
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "cancel failed");
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_list_workloads() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    server
        .clone()
        .submit_workload(Context::current(), sample_submission("a"))
        .await
        .expect("Submit failed");
    server
        .clone()
        .submit_workload(Context::current(), sample_submission("b"))
        .await
        .expect("Submit failed");

    let list = server
        .list_workloads(Context::current(), None)
        .await
        .expect("List should succeed");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_query_capabilities_executor_error() {
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new(
        "0.1.0",
        Arc::new(FailingExecutor),
        Some(Arc::clone(&error_count)),
    );

    let result = server.query_capabilities(Context::current()).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "capabilities unavailable");
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_standalone_executor() {
    let executor = StandaloneExecutor::new();
    let caps = executor
        .query_capabilities()
        .await
        .expect("Capabilities failed");

    assert_eq!(caps.service_id, "toadstool-standalone");
    assert!(!caps.compute_units.is_empty());

    assert!(caps.available_resources.total_cpu_cores > 0);
    assert!(caps.available_resources.total_memory_bytes > 0);
}

#[tokio::test]
async fn test_query_capabilities() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);

    let caps = server
        .query_capabilities(Context::current())
        .await
        .expect("Capabilities query failed");

    assert_eq!(caps.service_id, "toadstool-standalone");
    assert!(!caps.compute_units.is_empty());
}

#[tokio::test]
async fn test_server_clone() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    let cloned = server.clone();

    let health1 = server
        .health_check(Context::current())
        .await
        .expect("Health check failed");
    let health2 = cloned
        .health_check(Context::current())
        .await
        .expect("Health check failed");
    assert_eq!(health1.version, health2.version);
}

#[tokio::test]
async fn test_list_workloads_with_filter() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    server
        .clone()
        .submit_workload(Context::current(), sample_submission("f1"))
        .await
        .expect("Submit failed");

    let filter =
        std::collections::HashMap::from([("workload_type".to_string(), "cpu_compute".to_string())]);
    let list = server
        .list_workloads(Context::current(), Some(filter))
        .await
        .expect("List should succeed");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_health_check_resource_utilization() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);

    let health = server
        .health_check(Context::current())
        .await
        .expect("Health check failed");

    assert!(health.healthy);
    assert!(health.resource_utilization >= 0.0 && health.resource_utilization <= 1.0);
}

/// Executor that returns Queued status for testing active/queued workload counts
struct QueuedExecutor;

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl WorkloadExecutor for QueuedExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
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
        Ok(ComputeCapabilities {
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
        })
    }

    async fn cancel(&self, _workload_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn test_health_check_with_queued_workloads() {
    let executor = Arc::new(QueuedExecutor);
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);
    server
        .clone()
        .submit_workload(Context::current(), sample_submission("queued-1"))
        .await
        .expect("Submit failed");

    let health = server
        .health_check(Context::current())
        .await
        .expect("Health check failed");

    assert!(health.healthy);
    assert_eq!(health.active_workloads, 1);
    assert_eq!(health.queued_workloads, 1);
}

#[tokio::test]
#[expect(deprecated)]
async fn test_serve_tcp_debug_returns_error() {
    let executor = Arc::new(StandaloneExecutor::new());
    let server = ToadStoolTarpcServer::new("0.1.0", executor, None);

    let result = server.serve_tcp_debug("127.0.0.1:0".parse().unwrap()).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("deprecated") || err_msg.contains("not implemented"));
}

#[tokio::test]
async fn test_standalone_executor_default() {
    let executor = StandaloneExecutor::default();
    let caps = executor
        .query_capabilities()
        .await
        .expect("Capabilities failed");
    assert_eq!(caps.service_id, "toadstool-standalone");
}
