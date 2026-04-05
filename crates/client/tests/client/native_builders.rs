// SPDX-License-Identifier: AGPL-3.0-or-later
//! ExecutionInfo and related tests (continuation from wasm_builders)

use std::time::SystemTime;
use toadstool_client::*;
use uuid::Uuid;

#[test]
fn test_execution_info_timeout() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Timeout,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: Some(SystemTime::now()),
        runtime_type: Some("python".to_string()),
        error_message: Some("Execution exceeded timeout of 300s".to_string()),
        output: None,
        metrics: None,
    };

    assert!(matches!(info.status, ExecutionStatus::Timeout));
    assert!(info.error_message.as_ref().unwrap().contains("timeout"));
}

#[test]
fn test_execution_info_with_runtime_type() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: None,
        runtime_type: Some("container-docker".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    assert_eq!(info.runtime_type, Some("container-docker".to_string()));
}

#[test]
fn test_execution_info_serialization() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: Some(SystemTime::now()),
        runtime_type: Some("native".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("execution_id"));
}

#[test]
fn test_execution_info_clone() {
    let info1 = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: SystemTime::now(),
        started_at: None,
        completed_at: None,
        runtime_type: None,
        error_message: None,
        output: None,
        metrics: None,
    };

    let info2 = info1.clone();
    assert_eq!(info1.execution_id, info2.execution_id);
}

// ============================================================================
// ExecutionOutput Tests
// ============================================================================

#[test]
fn test_execution_output_success() {
    let output = ExecutionOutput {
        stdout: Some("Hello, World!".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stdout.is_some());
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_stderr() {
    let output = ExecutionOutput {
        stdout: Some("Output".to_string()),
        stderr: Some("Warning: deprecated".to_string()),
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stderr.is_some());
    assert!(output.stderr.as_ref().unwrap().contains("Warning"));
}

#[test]
fn test_execution_output_failure() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: Some("Error: file not found".to_string()),
        exit_code: Some(1),
        artifacts: vec![],
    };

    assert!(output.stdout.is_none());
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_artifacts() {
    let output = ExecutionOutput {
        stdout: Some("Build complete".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![
            "output.txt".to_string(),
            "binary".to_string(),
            "logs.tar.gz".to_string(),
        ],
    };

    assert_eq!(output.artifacts.len(), 3);
    assert!(output.artifacts.contains(&"binary".to_string()));
}

#[test]
fn test_execution_output_large_stdout() {
    let large_output = "x".repeat(10000);
    let output = ExecutionOutput {
        stdout: Some(large_output.clone()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert_eq!(output.stdout.as_ref().unwrap().len(), 10000);
}

#[test]
fn test_execution_output_empty() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stdout.is_none());
    assert!(output.stderr.is_none());
}

#[test]
fn test_execution_output_no_exit_code() {
    let output = ExecutionOutput {
        stdout: Some("Running...".to_string()),
        stderr: None,
        exit_code: None,
        artifacts: vec![],
    };

    assert!(output.exit_code.is_none());
}

#[test]
fn test_execution_output_signal_terminated() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: Some("Killed".to_string()),
        exit_code: Some(128 + 9), // SIGKILL
        artifacts: vec![],
    };

    assert_eq!(output.exit_code, Some(137));
}

#[test]
fn test_execution_output_serialization() {
    let output = ExecutionOutput {
        stdout: Some("test".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec!["file.txt".to_string()],
    };

    let json = serde_json::to_string(&output).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_execution_output_clone() {
    let output1 = ExecutionOutput {
        stdout: Some("data".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    let output2 = output1.clone();
    assert_eq!(output1.exit_code, output2.exit_code);
}

// ============================================================================
// ExecutionMetrics Tests
// ============================================================================

#[test]
fn test_execution_metrics_basic() {
    let metrics = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 75.5,
        memory_peak_bytes: 512 * 1024 * 1024,
        network_bytes_sent: 1000,
        network_bytes_received: 2000,
    };

    assert_eq!(metrics.duration_ms, 5000);
    assert_eq!(metrics.cpu_usage_percent, 75.5);
}

#[test]
fn test_execution_metrics_short_duration() {
    let metrics = ExecutionMetrics {
        duration_ms: 10,
        cpu_usage_percent: 5.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert!(metrics.duration_ms < 100);
}

#[test]
fn test_execution_metrics_long_duration() {
    let metrics = ExecutionMetrics {
        duration_ms: 3_600_000, // 1 hour
        cpu_usage_percent: 95.0,
        memory_peak_bytes: 16 * 1024 * 1024 * 1024,
        network_bytes_sent: 1024 * 1024 * 1024,
        network_bytes_received: 2 * 1024 * 1024 * 1024,
    };

    assert!(metrics.duration_ms > 1_000_000);
}

#[test]
fn test_execution_metrics_low_cpu() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 0.5,
        memory_peak_bytes: 1024,
        network_bytes_sent: 100,
        network_bytes_received: 100,
    };

    assert!(metrics.cpu_usage_percent < 1.0);
}

#[test]
fn test_execution_metrics_high_cpu() {
    let metrics = ExecutionMetrics {
        duration_ms: 10000,
        cpu_usage_percent: 99.9,
        memory_peak_bytes: 1024 * 1024,
        network_bytes_sent: 1000,
        network_bytes_received: 1000,
    };

    assert!(metrics.cpu_usage_percent > 99.0);
}

#[test]
fn test_execution_metrics_high_memory() {
    let metrics = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 50.0,
        memory_peak_bytes: 32 * 1024 * 1024 * 1024, // 32GB
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert!(metrics.memory_peak_bytes > 16 * 1024 * 1024 * 1024);
}

#[test]
fn test_execution_metrics_high_network() {
    let metrics = ExecutionMetrics {
        duration_ms: 30000,
        cpu_usage_percent: 30.0,
        memory_peak_bytes: 1024 * 1024,
        network_bytes_sent: 10 * 1024 * 1024 * 1024, // 10GB
        network_bytes_received: 5 * 1024 * 1024 * 1024, // 5GB
    };

    assert!(metrics.network_bytes_sent > 1024 * 1024 * 1024);
}

#[test]
fn test_execution_metrics_zero_network() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 10.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert_eq!(metrics.network_bytes_sent, 0);
    assert_eq!(metrics.network_bytes_received, 0);
}

#[test]
fn test_execution_metrics_serialization() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 50.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 100,
        network_bytes_received: 200,
    };

    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("duration_ms"));
}

#[test]
fn test_execution_metrics_clone() {
    let metrics1 = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 60.0,
        memory_peak_bytes: 2048,
        network_bytes_sent: 500,
        network_bytes_received: 1000,
    };

    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.duration_ms, metrics2.duration_ms);
}

// ============================================================================
// ToadStoolEvent Tests
// ============================================================================

#[test]
fn test_event_execution_status_changed() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: Uuid::new_v4().to_string(),
        status: "running".to_string(),
    };

    match &event {
        ToadStoolEvent::ExecutionStatusChanged { execution_id, .. } => {
            assert!(!execution_id.is_empty());
        }
        _ => panic!("Expected ExecutionStatusChanged event"),
    }
}

#[test]
fn test_event_cluster_health_changed() {
    let event = ToadStoolEvent::ClusterHealthChanged { healthy: true };
    match &event {
        ToadStoolEvent::ClusterHealthChanged { healthy } => assert!(*healthy),
        _ => panic!("Expected ClusterHealthChanged"),
    }
}
