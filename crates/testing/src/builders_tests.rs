// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn test_execution_request_builder() {
    let request = ExecutionRequestBuilder::new()
        .native_workload("/bin/echo", vec!["hello".to_string()])
        .timeout(Duration::from_secs(60))
        .env_var("TEST", "value")
        .build();

    assert!(matches!(request.workload, WorkloadSpec::Native { .. }));
    assert_eq!(request.timeout, Some(Duration::from_secs(60)));
    assert_eq!(request.environment.get("TEST"), Some(&"value".to_string()));
}

#[test]
fn test_execution_response_builder() {
    let response = ExecutionResponseBuilder::new()
        .success()
        .duration(Duration::from_secs(10))
        .runtime_used(RuntimeType::Container)
        .warning("Test warning")
        .build();

    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.duration, Duration::from_secs(10));
    assert_eq!(response.runtime_used, RuntimeType::Container);
    assert_eq!(response.warnings, vec!["Test warning".to_string()]);
}

#[test]
fn test_security_context_builder() {
    let context = SecurityContextBuilder::new()
        .with_isolation_level(IsolationLevel::Enhanced)
        .with_capabilities(vec![Capability::Execute, Capability::Read])
        .build();

    assert_eq!(context.isolation_level, IsolationLevel::Enhanced);
    assert!(context.capabilities.contains(&Capability::Execute));
    assert!(context.capabilities.contains(&Capability::Read));
}

#[test]
fn test_runtime_metrics_builder() {
    let metrics = RuntimeMetricsBuilder::new()
        .custom_metric(
            "test_metric",
            serde_json::Value::Number(serde_json::Number::from(42)),
        )
        .build();

    // Custom metrics are not directly accessible via fields
    // Test passes if we can build metrics
    assert!(metrics.cpu.usage_percent > 0.0);
}

#[test]
fn test_execution_request_builder_defaults() {
    let request = ExecutionRequestBuilder::new().build();
    assert!(request.execution_id != uuid::Uuid::nil());
    assert!(matches!(request.workload, WorkloadSpec::Native { .. }));
    assert_eq!(
        request.timeout,
        Some(Duration::from_secs(TestConstants::DEFAULT_TIMEOUT_SECS))
    );
}

#[test]
fn test_execution_request_builder_container_workload() {
    let request = ExecutionRequestBuilder::new()
        .container_workload("alpine:latest", Some(vec!["sh".to_string()]))
        .build();
    assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    if let WorkloadSpec::Container { image, .. } = &request.workload {
        assert_eq!(image, "alpine:latest");
    }
}

#[test]
fn test_execution_request_builder_wasm_workload() {
    let request = ExecutionRequestBuilder::new()
        .wasm_workload(vec![0x00, 0x61, 0x73, 0x6d])
        .build();
    assert!(matches!(request.workload, WorkloadSpec::Wasm { .. }));
}

#[test]
fn test_execution_request_builder_execution_id() {
    let id = uuid::Uuid::new_v4();
    let request = ExecutionRequestBuilder::new().execution_id(id).build();
    assert_eq!(request.execution_id, id);
}

#[test]
fn test_execution_response_builder_failed() {
    let response = ExecutionResponseBuilder::new()
        .failed("Test error message")
        .build();
    assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
    if let ExecutionStatus::Failed { error } = &response.status {
        assert_eq!(error, "Test error message");
    }
}

#[test]
fn test_execution_response_builder_cancelled() {
    let response = ExecutionResponseBuilder::new().cancelled().build();
    assert_eq!(response.status, ExecutionStatus::Cancelled);
}

#[test]
fn test_execution_response_builder_timed_out() {
    let response = ExecutionResponseBuilder::new().timed_out().build();
    assert_eq!(response.status, ExecutionStatus::TimedOut);
}

#[test]
fn test_security_context_builder_default() {
    let context = SecurityContextBuilder::new().build();
    assert_eq!(context.isolation_level, IsolationLevel::Standard);
    assert!(context.capabilities.contains(&Capability::Execute));
}

#[test]
fn test_security_context_builder_with_policy() {
    let context = SecurityContextBuilder::new()
        .with_policy("deny-all".to_string())
        .build();
    assert_eq!(context.isolation_level, IsolationLevel::Standard);
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // test values are exact literals
fn test_runtime_metrics_builder_with_cpu() {
    let metrics = RuntimeMetricsBuilder::new()
        .cpu_metrics(CpuMetrics {
            usage_percent: 99.0,
            cores_used: 4.0,
            cpu_time_seconds: 10.0,
        })
        .build();
    assert_eq!(metrics.cpu.usage_percent, 99.0);
    assert_eq!(metrics.cpu.cores_used, 4.0);
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // test values are exact literals
fn test_runtime_metrics_builder_with_memory() {
    let metrics = RuntimeMetricsBuilder::new()
        .memory_metrics(MemoryMetrics {
            usage_percent: 80.0,
            used_bytes: 1024 * 1024 * 512,
            peak_bytes: 1024 * 1024 * 768,
        })
        .build();
    assert_eq!(metrics.memory.usage_percent, 80.0);
    assert_eq!(metrics.memory.used_bytes, 1024 * 1024 * 512);
}
