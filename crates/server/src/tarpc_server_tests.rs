use super::*;
use crate::rpc_types::{ResourceRequirements, WorkloadPriority};
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;

struct MockExecutor;

#[async_trait::async_trait]
impl WorkloadExecutor for MockExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        Ok(WorkloadResult {
            workload_id: submission.workload_id,
            status: WorkloadStatus::Completed,
            data: Some(submission.data.clone()),
            error: None,
            metrics: ExecutionMetrics {
                queued_duration_secs: 0.0,
                execution_duration_secs: 0.1,
                cpu_cores_used: 1,
                memory_used_bytes: submission.data.len() as u64,
                gpu_memory_used_bytes: None,
            },
        })
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        Ok(ComputeCapabilities {
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
        })
    }

    async fn cancel(&self, workload_id: &str) -> Result<(), String> {
        let _ = workload_id;
        Ok(())
    }
}

#[test]
fn test_tarpc_server_new() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test-v1", executor, None);
    assert!(server.error_count.load(Ordering::Relaxed) == 0);
}

#[test]
fn test_tarpc_server_new_with_error_count() {
    let executor = Arc::new(MockExecutor);
    let error_count = Arc::new(AtomicU64::new(42));
    let server = ToadStoolTarpcServer::new("v1", executor, Some(Arc::clone(&error_count)));
    assert_eq!(server.error_count.load(Ordering::Relaxed), 42);
}

#[test]
fn test_tarpc_server_clone() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);
    let cloned = server.clone();
    assert_eq!(server.version.as_ref(), cloned.version.as_ref());
}

#[tokio::test]
async fn test_serve_tcp_debug_deprecated_returns_err() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let result = server.serve_tcp_debug(addr).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("deprecated"));
}

#[test]
fn test_standalone_executor_new() {
    let exec = StandaloneExecutor::new();
    assert_eq!(exec.capabilities.service_id, "toadstool-standalone");
    assert!(!exec.capabilities.compute_units.is_empty());
    assert!(exec
        .capabilities
        .supported_workload_types
        .contains(&"cpu_compute".to_string()));
}

#[test]
fn test_standalone_executor_default() {
    let exec = StandaloneExecutor::default();
    assert_eq!(exec.capabilities.service_id, "toadstool-standalone");
}

fn mk_submission(id: &str, workload_type: &str, data: Vec<u8>) -> WorkloadSubmission {
    WorkloadSubmission {
        workload_id: id.to_string(),
        workload_type: workload_type.to_string(),
        data: data.into(),
        metadata: std::collections::HashMap::new(),
        priority: WorkloadPriority::Normal,
        requirements: ResourceRequirements::default(),
    }
}

#[tokio::test]
async fn test_standalone_executor_execute() {
    let exec = StandaloneExecutor::new();
    let submission = mk_submission("test-wl-1", "cpu_compute", vec![1, 2, 3]);
    let result = exec.execute(submission).await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.workload_id, "test-wl-1");
    assert!(matches!(res.status, WorkloadStatus::Completed));
    assert!(res.data.is_some());
}

#[tokio::test]
async fn test_standalone_executor_execute_gpu_hint() {
    let exec = StandaloneExecutor::new();
    let submission = mk_submission("gpu-wl", "gpu_compute", vec![0u8; 100]);
    let result = exec.execute(submission).await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.metrics.gpu_memory_used_bytes.is_some());
}

#[tokio::test]
async fn test_standalone_executor_query_capabilities() {
    let exec = StandaloneExecutor::new();
    let caps = exec.query_capabilities().await;
    assert!(caps.is_ok());
    let c = caps.unwrap();
    assert_eq!(c.service_id, "toadstool-standalone");
    assert!(!c.compute_units.is_empty());
}

#[tokio::test]
async fn test_standalone_executor_cancel() {
    let exec = StandaloneExecutor::new();
    let result = exec.cancel("any-id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_executor_submit_and_query() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test", executor, None);
    let submission = mk_submission("mock-1", "cpu_compute", vec![]);
    let result = server
        .clone()
        .submit_workload(tarpc::context::current(), submission)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().workload_id, "mock-1");
}

#[tokio::test]
async fn test_mock_executor_query_status() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test", executor, None);
    let submission = mk_submission("status-test", "cpu_compute", vec![]);
    server
        .clone()
        .submit_workload(tarpc::context::current(), submission)
        .await
        .unwrap();
    let status = server
        .clone()
        .query_status(tarpc::context::current(), "status-test".to_string())
        .await;
    assert!(status.is_ok());
}

#[tokio::test]
async fn test_mock_executor_query_status_not_found() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test", executor, None);
    let status = server
        .clone()
        .query_status(
            tarpc::context::current(),
            "nonexistent-workload".to_string(),
        )
        .await;
    assert!(status.is_err());
    assert!(status.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_mock_executor_list_workloads() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test", executor, None);
    let list = server
        .clone()
        .list_workloads(tarpc::context::current(), None)
        .await;
    assert!(list.is_ok());
    assert!(list.unwrap().is_empty());
}

#[tokio::test]
async fn test_mock_executor_health_check() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("test-v1", executor, None);
    let health = server.clone().health_check(tarpc::context::current()).await;
    assert!(health.is_ok());
    let h = health.unwrap();
    assert!(h.healthy);
    assert_eq!(h.version, "test-v1");
    assert!(h.resource_utilization >= 0.0 && h.resource_utilization <= 1.0);
}

#[tokio::test]
async fn test_workload_result_serialization() {
    let result = WorkloadResult {
        workload_id: "ser-1".to_string(),
        status: WorkloadStatus::Completed,
        data: Some(vec![1, 2, 3].into()),
        error: None,
        metrics: ExecutionMetrics {
            queued_duration_secs: 0.0,
            execution_duration_secs: 1.5,
            cpu_cores_used: 4,
            memory_used_bytes: 1024,
            gpu_memory_used_bytes: Some(4096),
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let restored: WorkloadResult = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.workload_id, result.workload_id);
    assert!(matches!(restored.status, WorkloadStatus::Completed));
}

#[tokio::test]
async fn test_workload_submission_serialization() {
    let sub = mk_submission("sub-1", "gpu_compute", vec![0xff, 0xfe]);
    let json = serde_json::to_string(&sub).unwrap();
    let restored: WorkloadSubmission = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.workload_id, sub.workload_id);
    assert_eq!(restored.workload_type, sub.workload_type);
}

#[tokio::test]
async fn test_workload_status_variants() {
    let running = WorkloadStatus::Running;
    let completed = WorkloadStatus::Completed;
    let cancelled = WorkloadStatus::Cancelled;
    let _ = (running, completed, cancelled);
}

struct FailingExecutor;

#[async_trait::async_trait]
impl WorkloadExecutor for FailingExecutor {
    async fn execute(&self, _submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        Err("executor failed".to_string())
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        Err("capabilities failed".to_string())
    }

    async fn cancel(&self, _workload_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn test_submit_workload_executor_error_increments_error_count() {
    let executor = Arc::new(FailingExecutor);
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new("v1", executor, Some(Arc::clone(&error_count)));

    let submission = mk_submission("fail-1", "cpu_compute", vec![]);
    let result = server
        .clone()
        .submit_workload(tarpc::context::current(), submission)
        .await;

    assert!(result.is_err());
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_query_capabilities_executor_error() {
    let executor = Arc::new(FailingExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);

    let result = server
        .clone()
        .query_capabilities(tarpc::context::current())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("capabilities failed"));
}

#[tokio::test]
async fn test_query_status_not_found_increments_error_count() {
    let executor = Arc::new(MockExecutor);
    let error_count = Arc::new(AtomicU64::new(0));
    let server = ToadStoolTarpcServer::new("v1", executor, Some(Arc::clone(&error_count)));

    let result = server
        .clone()
        .query_status(tarpc::context::current(), "nonexistent-id".to_string())
        .await;

    assert!(result.is_err());
    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_cancel_workload_success() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);

    let submission = mk_submission("cancel-me", "cpu_compute", vec![]);
    server
        .clone()
        .submit_workload(tarpc::context::current(), submission)
        .await
        .unwrap();

    let result = server
        .clone()
        .cancel_workload(tarpc::context::current(), "cancel-me".to_string())
        .await;
    assert!(result.is_ok());

    let status = server
        .clone()
        .query_status(tarpc::context::current(), "cancel-me".to_string())
        .await;
    assert!(status.is_ok());
    assert!(matches!(status.unwrap().status, WorkloadStatus::Cancelled));
}

#[tokio::test]
async fn test_list_workloads_with_filter() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);

    let filter = Some(std::collections::HashMap::from([(
        "status".to_string(),
        "running".to_string(),
    )]));
    let list = server
        .clone()
        .list_workloads(tarpc::context::current(), filter)
        .await;
    assert!(list.is_ok());
    assert!(list.unwrap().is_empty());
}

#[tokio::test]
async fn test_list_workloads_after_submit() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);

    let submission = mk_submission("list-test", "cpu_compute", vec![]);
    server
        .clone()
        .submit_workload(tarpc::context::current(), submission)
        .await
        .unwrap();

    let list = server
        .clone()
        .list_workloads(tarpc::context::current(), None)
        .await;
    assert!(list.is_ok());
    let workloads = list.unwrap();
    assert_eq!(workloads.len(), 1);
    assert_eq!(workloads[0].workload_id, "list-test");
}

#[tokio::test]
async fn test_health_check_includes_error_count() {
    let executor = Arc::new(FailingExecutor);
    let error_count = Arc::new(AtomicU64::new(5));
    let server = ToadStoolTarpcServer::new("v1", executor, Some(Arc::clone(&error_count)));

    let _ = server
        .clone()
        .submit_workload(tarpc::context::current(), mk_submission("x", "cpu", vec![]))
        .await;

    let health = server.clone().health_check(tarpc::context::current()).await;
    assert!(health.is_ok());
    let h = health.unwrap();
    assert!(h.error_count >= 5);
}

#[tokio::test]
async fn test_standalone_executor_compute_units_has_tflops() {
    let exec = StandaloneExecutor::new();
    let caps = exec.query_capabilities().await.unwrap();
    assert!(!caps.compute_units.is_empty());
    let unit = &caps.compute_units[0];
    assert!(unit.tflops.is_some());
}

#[tokio::test]
async fn test_standalone_executor_execute_empty_data() {
    let exec = StandaloneExecutor::new();
    let submission = mk_submission("empty", "cpu_compute", vec![]);
    let result = exec.execute(submission).await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.data.is_some());
    assert!(res.data.unwrap().is_empty());
}

#[tokio::test]
async fn test_standalone_executor_execute_large_data_truncates() {
    let exec = StandaloneExecutor::new();
    let data = vec![0u8; 2048];
    let submission = mk_submission("large", "cpu_compute", data);
    let result = exec.execute(submission).await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.data.is_some());
    assert!(res.data.unwrap().len() <= 1024);
}

#[tokio::test]
async fn test_mock_executor_cancel_workload_not_tracked() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("v1", executor, None);

    let result = server
        .clone()
        .cancel_workload(tarpc::context::current(), "never-submitted".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tarpc_server_version_reflected_in_health() {
    let executor = Arc::new(MockExecutor);
    let server = ToadStoolTarpcServer::new("2.3.4", executor, None);
    let health = server.health_check(tarpc::context::current()).await;
    assert!(health.is_ok());
    assert_eq!(health.unwrap().version, "2.3.4");
}
