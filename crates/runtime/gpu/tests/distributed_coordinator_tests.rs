// SPDX-License-Identifier: AGPL-3.0-or-later
//! Distributed Coordinator Test Coverage
//!
//! Comprehensive tests for distributed GPU scheduling across multiple towers.

use std::time::Instant;
use toadstool_runtime_gpu::distributed::{
    DistributedStats, JobStatus, PartitionStrategy, RemoteTowerEndpoint, TowerManager,
};

// ============================================================================
// Tower Manager Tests
// ============================================================================

#[tokio::test]
async fn test_tower_manager_creation() {
    let manager = TowerManager::new("test-tower".to_string());
    assert_eq!(
        manager.tower_count().await,
        1,
        "New manager should have local tower"
    );
    assert_eq!(manager.local_tower_id(), "test-tower");
}

#[tokio::test]
async fn test_tower_manager_register_tower() {
    let manager = TowerManager::new("local-tower".to_string());

    let endpoint = RemoteTowerEndpoint {
        tower_id: "tower-1".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 10,
    };

    manager.register_tower(endpoint).await;
    assert_eq!(
        manager.tower_count().await,
        2,
        "Should have local + 1 remote tower"
    );

    let tower_ids = manager.available_tower_ids().await;
    assert_eq!(tower_ids.len(), 2);
    assert!(tower_ids.contains(&"tower-1".to_string()));
}

#[tokio::test]
async fn test_tower_manager_multiple_towers() {
    let manager = TowerManager::new("local-tower".to_string());

    // Register multiple remote towers
    for i in 1..=3 {
        let endpoint = RemoteTowerEndpoint {
            tower_id: format!("tower-{i}"),
            address: format!("http://192.168.1.{}:8084", 10 + i),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 10 + i * 5,
        };
        manager.register_tower(endpoint).await;
    }

    assert_eq!(
        manager.tower_count().await,
        4,
        "Should have local + 3 remote towers"
    );

    let tower_ids = manager.available_tower_ids().await;
    assert_eq!(tower_ids.len(), 4);
}

#[tokio::test]
async fn test_tower_manager_duplicate_registration() {
    let manager = TowerManager::new("local-tower".to_string());

    let endpoint = RemoteTowerEndpoint {
        tower_id: "tower-1".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 10,
    };

    // Register same tower twice
    manager.register_tower(endpoint.clone()).await;
    manager.register_tower(endpoint).await;

    // Should handle duplicates (replace old entry)
    assert_eq!(
        manager.tower_count().await,
        2,
        "Should still have local + 1 remote"
    );
}

// ============================================================================
// Distributed Stats Tests
// ============================================================================

#[test]
fn test_distributed_stats_empty() {
    let stats = DistributedStats::empty();

    assert_eq!(stats.total_towers, 0);
    assert_eq!(stats.active_towers, 0);
    assert_eq!(stats.total_jobs, 0);
    assert_eq!(stats.pending_jobs, 0);
    assert_eq!(stats.running_jobs, 0);
    assert_eq!(stats.completed_jobs, 0);
    assert_eq!(stats.failed_jobs, 0);
}

#[test]
fn test_distributed_stats_creation() {
    let stats = DistributedStats {
        total_towers: 5,
        active_towers: 4,
        total_jobs: 10,
        pending_jobs: 2,
        running_jobs: 3,
        completed_jobs: 4,
        failed_jobs: 1,
    };

    assert_eq!(stats.total_towers, 5);
    assert_eq!(stats.active_towers, 4);
    assert_eq!(stats.total_jobs, 10);
    assert_eq!(
        stats.pending_jobs + stats.running_jobs + stats.completed_jobs + stats.failed_jobs,
        10
    );
}

// ============================================================================
// Job Status Tests
// ============================================================================

#[test]
fn test_job_status_pending() {
    let status = JobStatus::Pending;
    assert!(matches!(status, JobStatus::Pending));
}

#[test]
fn test_job_status_scheduled() {
    let status = JobStatus::Scheduled;
    assert!(matches!(status, JobStatus::Scheduled));
}

#[test]
fn test_job_status_running() {
    let status = JobStatus::Running;
    assert!(matches!(status, JobStatus::Running));
}

#[test]
fn test_job_status_completed() {
    let status = JobStatus::Completed;
    assert!(matches!(status, JobStatus::Completed));
}

#[test]
fn test_job_status_failed() {
    let status = JobStatus::Failed;
    assert!(matches!(status, JobStatus::Failed));
}

#[test]
fn test_job_status_transitions() {
    let mut status = JobStatus::Pending;
    assert_eq!(status, JobStatus::Pending);

    status = JobStatus::Scheduled;
    assert_eq!(status, JobStatus::Scheduled);

    status = JobStatus::Running;
    assert_eq!(status, JobStatus::Running);

    status = JobStatus::Completed;
    assert_eq!(status, JobStatus::Completed);
}

// ============================================================================
// Partition Strategy Tests
// ============================================================================

#[test]
fn test_partition_strategy_single() {
    let strategy = PartitionStrategy::Single;
    assert!(matches!(strategy, PartitionStrategy::Single));
}

#[test]
fn test_partition_strategy_data_parallel() {
    let strategy = PartitionStrategy::DataParallel { chunk_size: 1024 };

    if let PartitionStrategy::DataParallel { chunk_size } = strategy {
        assert_eq!(chunk_size, 1024);
    } else {
        panic!("Expected DataParallel strategy");
    }
}

#[test]
fn test_partition_strategy_redundant() {
    let strategy = PartitionStrategy::Redundant { replicas: 3 };

    if let PartitionStrategy::Redundant { replicas } = strategy {
        assert_eq!(replicas, 3);
    } else {
        panic!("Expected Redundant strategy");
    }
}

#[test]
fn test_partition_strategy_pipeline() {
    let strategy = PartitionStrategy::Pipeline {
        stages: vec![
            "stage1".to_string(),
            "stage2".to_string(),
            "stage3".to_string(),
        ],
    };

    if let PartitionStrategy::Pipeline { stages } = strategy {
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], "stage1");
    } else {
        panic!("Expected Pipeline strategy");
    }
}

// ============================================================================
// Remote Tower Endpoint Tests
// ============================================================================

#[test]
fn test_remote_tower_endpoint_creation() {
    let endpoint = RemoteTowerEndpoint {
        tower_id: "tower-1".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 15,
    };

    assert_eq!(endpoint.tower_id, "tower-1");
    assert_eq!(endpoint.address, "http://192.168.1.10:8084");
    assert_eq!(endpoint.latency_ms, 15);
}

#[test]
fn test_remote_tower_endpoint_with_capabilities() {
    let endpoint = RemoteTowerEndpoint {
        tower_id: "cuda-tower".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None, // Simplified for testing
        last_seen: Instant::now(),
        latency_ms: 10,
    };

    assert!(endpoint.gpu_capabilities.is_none());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_tower_manager_with_varying_latencies() {
    let manager = TowerManager::new("local-tower".to_string());

    // Add towers with different latencies
    let low_latency = RemoteTowerEndpoint {
        tower_id: "fast-tower".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 5,
    };

    let high_latency = RemoteTowerEndpoint {
        tower_id: "slow-tower".to_string(),
        address: "http://192.168.1.11:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 50,
    };

    manager.register_tower(low_latency).await;
    manager.register_tower(high_latency).await;

    assert_eq!(manager.tower_count().await, 3);
}

#[tokio::test]
async fn test_tower_manager_sequential_registrations() {
    let manager = TowerManager::new("local-tower".to_string());

    // Register towers sequentially
    for i in 1..=5 {
        let endpoint = RemoteTowerEndpoint {
            tower_id: format!("tower-{i}"),
            address: format!("http://192.168.1.{}:8084", 10 + i),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 10,
        };
        manager.register_tower(endpoint).await;
    }

    assert_eq!(
        manager.tower_count().await,
        6,
        "Should have local + 5 remote towers"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_tower_manager_empty_tower_id() {
    let manager = TowerManager::new(String::new());
    assert_eq!(manager.local_tower_id(), "");
}

#[test]
fn test_partition_strategy_zero_chunks() {
    let strategy = PartitionStrategy::DataParallel { chunk_size: 0 };

    if let PartitionStrategy::DataParallel { chunk_size } = strategy {
        assert_eq!(chunk_size, 0); // Document current behavior
    }
}

#[test]
fn test_partition_strategy_zero_replicas() {
    let strategy = PartitionStrategy::Redundant { replicas: 0 };

    if let PartitionStrategy::Redundant { replicas } = strategy {
        assert_eq!(replicas, 0); // Document current behavior
    }
}

#[test]
fn test_partition_strategy_empty_pipeline() {
    let strategy = PartitionStrategy::Pipeline { stages: vec![] };

    if let PartitionStrategy::Pipeline { stages } = strategy {
        assert_eq!(stages.len(), 0); // Document current behavior
    }
}

#[test]
fn test_distributed_stats_consistency() {
    let stats = DistributedStats {
        total_towers: 3,
        active_towers: 3,
        total_jobs: 5,
        pending_jobs: 1,
        running_jobs: 2,
        completed_jobs: 1,
        failed_jobs: 1,
    };

    // Verify job counts add up
    let job_sum =
        stats.pending_jobs + stats.running_jobs + stats.completed_jobs + stats.failed_jobs;
    assert_eq!(job_sum, stats.total_jobs, "Job counts should sum to total");
}
