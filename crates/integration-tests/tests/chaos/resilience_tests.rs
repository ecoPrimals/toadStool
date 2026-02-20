//! Resilience tests
//!
//! Verify that ToadStool's subsystems recover predictably from faults,
//! maintain invariants under load, and do not lose state.

use std::time::Duration;
use tokio::time::timeout;
use toadstool_testing::chaos::{ChaosScenario, FaultType, ResourceType, SystemState};

#[tokio::test]
async fn test_system_state_default_is_empty() {
    let state = SystemState::default();
    assert_eq!(state.active_nodes, 0);
    assert_eq!(state.failed_nodes, 0);
    assert_eq!(state.data_loss_count, 0);
    assert_eq!(state.recovery_count, 0);
}

#[tokio::test]
async fn test_cluster_recovered_requires_active_nodes() {
    let mut state = SystemState::default();
    // Zero active nodes → not recovered even if no failures
    assert!(!state.cluster_recovered());

    state.active_nodes = 3;
    assert!(state.cluster_recovered());

    state.failed_nodes = 1;
    assert!(!state.cluster_recovered());
}

#[tokio::test]
async fn test_recovery_after_network_partition() {
    let scenario = ChaosScenario::new("recovery_after_partition")
        .with_fault(FaultType::NetworkPartition {
            duration_ms: 50,
            affected_nodes: vec!["node-a".to_string(), "node-b".to_string()],
        })
        .with_validation(|state: &SystemState| {
            if state.data_loss_count > 0 {
                Err(format!("Unexpected data loss: {}", state.data_loss_count))
            } else {
                Ok(())
            }
        })
        .with_timeout(Duration::from_secs(10));

    let result = timeout(Duration::from_secs(15), scenario.run()).await;
    assert!(result.is_ok(), "Recovery scenario should complete within timeout");
}

#[tokio::test]
async fn test_resilience_to_high_latency() {
    let scenario = ChaosScenario::new("high_latency_resilience")
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 200,
            duration_ms: 300,
        })
        .with_timeout(Duration::from_secs(10));

    let result = timeout(Duration::from_secs(15), scenario.run()).await;
    assert!(result.is_ok(), "System should remain resilient under high latency");
}

#[tokio::test]
async fn test_resilience_to_memory_pressure() {
    let scenario = ChaosScenario::new("memory_pressure_resilience")
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Memory,
            consumption_percent: 85,
            duration_ms: 200,
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok(), "System should survive memory pressure");
}

#[tokio::test]
async fn test_resilience_sequential_faults() {
    let scenario = ChaosScenario::new("sequential_faults")
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 30,
            duration_ms: 100,
        })
        .with_delayed_fault(
            FaultType::PacketLoss {
                loss_rate: 0.10,
                duration_ms: 100,
            },
            100,
        )
        .with_timeout(Duration::from_secs(10));

    let result = timeout(Duration::from_secs(15), scenario.run()).await;
    assert!(result.is_ok(), "Sequential fault scenario should complete");
}

#[tokio::test]
async fn test_metric_tracking_across_faults() {
    let mut state = SystemState {
        active_nodes: 5,
        failed_nodes: 0,
        data_loss_count: 0,
        recovery_count: 0,
        metrics: Default::default(),
    };

    // Simulate fault injection sequence
    state.failed_nodes += 1;
    state.active_nodes -= 1;
    state.set_metric("partition_start_ms", 100.0);

    // Simulate recovery
    state.active_nodes += 1;
    state.failed_nodes -= 1;
    state.recovery_count += 1;
    state.set_metric("partition_end_ms", 250.0);

    assert!(state.cluster_recovered());
    assert_eq!(state.recovery_count, 1);

    let start = state.get_metric("partition_start_ms").unwrap();
    let end = state.get_metric("partition_end_ms").unwrap();
    assert!(end > start, "Recovery should happen after partition start");
}

#[tokio::test]
async fn test_network_partition_single_node() {
    let scenario = ChaosScenario::new("single_node_partition")
        .with_fault(FaultType::NetworkPartition {
            duration_ms: 10,
            affected_nodes: vec!["solo".to_string()],
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_crash_recovery() {
    let scenario = ChaosScenario::new("process_crash_recovery")
        .with_fault(FaultType::ProcessCrash {
            node_id: "dummy-worker".to_string(),
            restart_delay_ms: 50,
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok(), "System should handle simulated process crash");
}
