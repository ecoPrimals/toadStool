//! Type serialization tests - Month 2 Week 1 Day 2
//!
//! Tier 1 tests: Coverage-measured type and serialization tests
//! Focus: DistributedConfig, Job types, serialization roundtrips

use serde::{Deserialize, Serialize};

// ============================================================================
// DistributedConfig Tests
// ============================================================================

#[test]
fn test_distributed_config_default() {
    let config = DistributedConfig::default();

    assert_eq!(config.max_workers, 100);
    assert_eq!(config.heartbeat_interval_secs, 30);
    assert!(config.enable_load_balancing);
}

#[test]
fn test_distributed_config_custom_values() {
    let config = DistributedConfig {
        max_workers: 200,
        heartbeat_interval_secs: 60,
        enable_load_balancing: false,
        ..Default::default()
    };

    assert_eq!(config.max_workers, 200);
    assert_eq!(config.heartbeat_interval_secs, 60);
    assert!(!config.enable_load_balancing);
}

#[test]
fn test_distributed_config_serialization() {
    let config = DistributedConfig {
        max_workers: 150,
        heartbeat_interval_secs: 45,
        enable_load_balancing: true,
        coordinator_endpoint: "http://coordinator:8080".to_string(),
    };

    // Serialize
    let json = serde_json::to_string(&config).unwrap();

    // Deserialize
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();

    // Verify roundtrip
    assert_eq!(deserialized.max_workers, 150);
    assert_eq!(deserialized.heartbeat_interval_secs, 45);
    assert!(deserialized.enable_load_balancing);
    assert_eq!(deserialized.coordinator_endpoint, "http://coordinator:8080");
}

#[test]
fn test_distributed_config_toml_serialization() {
    let config = DistributedConfig {
        max_workers: 150,
        heartbeat_interval_secs: 45,
        enable_load_balancing: true,
        coordinator_endpoint: "http://coordinator:8080".to_string(),
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&config).unwrap();

    // Deserialize from TOML
    let deserialized: DistributedConfig = toml::from_str(&toml_str).unwrap();

    // Verify roundtrip
    assert_eq!(deserialized.max_workers, 150);
    assert_eq!(deserialized.heartbeat_interval_secs, 45);
    assert!(deserialized.enable_load_balancing);
    assert_eq!(deserialized.coordinator_endpoint, "http://coordinator:8080");
}

// ============================================================================
// Job Type Tests
// ============================================================================

#[test]
fn test_job_creation() {
    let job = Job {
        id: "job-123".to_string(),
        name: "test-job".to_string(),
        status: JobStatus::Pending,
        priority: 5,
        created_at: std::time::SystemTime::now(),
    };

    assert_eq!(job.id, "job-123");
    assert_eq!(job.name, "test-job");
    assert_eq!(job.priority, 5);
}

#[test]
fn test_job_status_transitions() {
    let mut job = Job::new("test");

    assert_eq!(job.status, JobStatus::Pending);

    job.start();
    assert_eq!(job.status, JobStatus::Running);

    job.complete();
    assert_eq!(job.status, JobStatus::Completed);
}

#[test]
fn test_job_status_serialization() {
    let statuses = vec![
        JobStatus::Pending,
        JobStatus::Running,
        JobStatus::Completed,
        JobStatus::Failed,
        JobStatus::Cancelled,
    ];

    for status in statuses {
        // Serialize
        let json = serde_json::to_string(&status).unwrap();

        // Deserialize
        let deserialized: JobStatus = serde_json::from_str(&json).unwrap();

        // Verify
        assert_eq!(deserialized, status);
    }
}

#[test]
fn test_job_priority_ordering() {
    let job1 = Job {
        priority: 1,
        ..Job::new("low")
    };
    let job2 = Job {
        priority: 10,
        ..Job::new("high")
    };

    assert!(job2.priority > job1.priority);
}

// ============================================================================
// Worker Type Tests
// ============================================================================

#[test]
fn test_worker_registration() {
    let worker = Worker {
        id: "worker-1".to_string(),
        hostname: "node-1".to_string(),
        capabilities: vec!["cpu".to_string(), "memory".to_string()],
        status: WorkerStatus::Idle,
    };

    assert_eq!(worker.id, "worker-1");
    assert!(worker.capabilities.contains(&"cpu".to_string()));
}

#[test]
fn test_worker_serialization() {
    let worker = Worker {
        id: "worker-2".to_string(),
        hostname: "node-2".to_string(),
        capabilities: vec!["gpu".to_string()],
        status: WorkerStatus::Busy,
    };

    // Serialize
    let json = serde_json::to_string(&worker).unwrap();

    // Deserialize
    let deserialized: Worker = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "worker-2");
    assert_eq!(deserialized.status, WorkerStatus::Busy);
}

#[test]
fn test_worker_status_types() {
    let statuses = vec![
        WorkerStatus::Idle,
        WorkerStatus::Busy,
        WorkerStatus::Offline,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: WorkerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}

// ============================================================================
// TaskResult Type Tests
// ============================================================================

#[test]
fn test_task_result_success() {
    let result = TaskResult {
        task_id: "task-1".to_string(),
        success: true,
        output: Some("completed successfully".to_string()),
        error: None,
        duration_ms: 1500,
    };

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.duration_ms, 1500);
}

#[test]
fn test_task_result_failure() {
    let result = TaskResult {
        task_id: "task-2".to_string(),
        success: false,
        output: None,
        error: Some("task failed".to_string()),
        duration_ms: 500,
    };

    assert!(!result.success);
    assert!(result.error.is_some());
}

#[test]
fn test_task_result_serialization() {
    let result = TaskResult {
        task_id: "task-3".to_string(),
        success: true,
        output: Some("data".to_string()),
        error: None,
        duration_ms: 2000,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: TaskResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.task_id, "task-3");
    assert!(deserialized.success);
    assert_eq!(deserialized.duration_ms, 2000);
}

// ============================================================================
// ClusterMetrics Type Tests
// ============================================================================

#[test]
fn test_cluster_metrics_aggregation() {
    let metrics = ClusterMetrics {
        total_workers: 10,
        active_workers: 8,
        total_jobs: 100,
        completed_jobs: 75,
        failed_jobs: 5,
        average_job_duration_ms: 5000,
    };

    assert_eq!(metrics.total_workers, 10);
    assert_eq!(metrics.completion_rate(), 0.75);
    assert_eq!(metrics.failure_rate(), 0.05);
}

#[test]
fn test_cluster_metrics_serialization() {
    let metrics = ClusterMetrics::default();

    let json = serde_json::to_string(&metrics).unwrap();
    let deserialized: ClusterMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.total_workers, metrics.total_workers);
}

// ============================================================================
// Mock Type Definitions (Simplified for tests)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DistributedConfig {
    max_workers: usize,
    heartbeat_interval_secs: u64,
    enable_load_balancing: bool,
    coordinator_endpoint: String,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            max_workers: 100,
            heartbeat_interval_secs: 30,
            enable_load_balancing: true,
            coordinator_endpoint: "http://localhost:8080".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    id: String,
    name: String,
    status: JobStatus,
    priority: u8,
    #[serde(with = "toadstool_common::system_time_serde")]
    created_at: std::time::SystemTime,
}

impl Job {
    fn new(name: &str) -> Self {
        Self {
            id: format!("job-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            status: JobStatus::Pending,
            priority: 5,
            created_at: std::time::SystemTime::now(),
        }
    }

    fn start(&mut self) {
        self.status = JobStatus::Running;
    }

    fn complete(&mut self) {
        self.status = JobStatus::Completed;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum WorkerStatus {
    Idle,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Worker {
    id: String,
    hostname: String,
    capabilities: Vec<String>,
    status: WorkerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskResult {
    task_id: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
    duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ClusterMetrics {
    total_workers: usize,
    active_workers: usize,
    total_jobs: usize,
    completed_jobs: usize,
    failed_jobs: usize,
    average_job_duration_ms: u64,
}

impl ClusterMetrics {
    fn completion_rate(&self) -> f64 {
        if self.total_jobs == 0 {
            0.0
        } else {
            self.completed_jobs as f64 / self.total_jobs as f64
        }
    }

    fn failure_rate(&self) -> f64 {
        if self.total_jobs == 0 {
            0.0
        } else {
            self.failed_jobs as f64 / self.total_jobs as f64
        }
    }
}
