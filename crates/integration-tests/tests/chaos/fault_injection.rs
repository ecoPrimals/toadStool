//! Fault injection tests
//!
//! Verifies that ToadStool components handle injected faults gracefully:
//! invalid inputs, resource exhaustion signals, and malformed state.

use std::time::Duration;
use tokio::time::timeout;
use toadstool_testing::chaos::{ChaosScenario, FaultType, ResourceType, SystemState};

#[tokio::test]
async fn test_fault_scenario_construction() {
    let scenario = ChaosScenario::new("construction_smoke")
        .with_fault(FaultType::NetworkPartition {
            duration_ms: 100,
            affected_nodes: vec!["node1".to_string()],
        })
        .with_timeout(Duration::from_secs(5));

    assert_eq!(scenario.name, "construction_smoke");
    assert_eq!(scenario.faults.len(), 1);
}

#[tokio::test]
async fn test_fault_scenario_multiple_faults() {
    let scenario = ChaosScenario::new("multi_fault")
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 50,
            duration_ms: 200,
        })
        .with_fault(FaultType::PacketLoss {
            loss_rate: 0.05,
            duration_ms: 100,
        })
        .with_timeout(Duration::from_secs(5));

    assert_eq!(scenario.faults.len(), 2);
}

#[tokio::test]
async fn test_fault_scenario_delayed_fault() {
    let scenario = ChaosScenario::new("delayed_fault")
        .with_delayed_fault(
            FaultType::ProcessCrash {
                node_id: "test-worker".to_string(),
                restart_delay_ms: 100,
            },
            200,
        )
        .with_timeout(Duration::from_secs(5));

    assert_eq!(scenario.faults.len(), 1);
    assert!(scenario.faults[0].inject_at_ms > 0);
}

#[tokio::test]
async fn test_fault_scenario_with_validation() {
    let scenario = ChaosScenario::new("validated_fault")
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Memory,
            consumption_percent: 80,
            duration_ms: 100,
        })
        .with_validation(|state: &SystemState| {
            if state.data_loss_count == 0 {
                Ok(())
            } else {
                Err(format!("Unexpected data loss: {}", state.data_loss_count))
            }
        })
        .with_timeout(Duration::from_secs(5));

    assert!(scenario.validator.is_some());
}

#[tokio::test]
async fn test_fault_scenario_runs() {
    let scenario = ChaosScenario::new("run_smoke")
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 1,
            duration_ms: 50,
        })
        .with_timeout(Duration::from_secs(10));

    let result = timeout(Duration::from_secs(15), scenario.run()).await;
    assert!(result.is_ok(), "Scenario should complete within timeout");
}

#[tokio::test]
async fn test_fault_scenario_cpu_exhaustion() {
    let scenario = ChaosScenario::new("cpu_exhaustion")
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Cpu,
            consumption_percent: 90,
            duration_ms: 100,
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok(), "CPU exhaustion scenario should complete");
}

#[tokio::test]
async fn test_fault_scenario_disk_exhaustion() {
    let scenario = ChaosScenario::new("disk_exhaustion")
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Disk,
            consumption_percent: 95,
            duration_ms: 100,
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok(), "Disk exhaustion scenario should complete");
}

#[tokio::test]
async fn test_system_state_metrics() {
    let mut state = SystemState::default();
    state.set_metric("latency_ms", 42.0);

    let val = state.get_metric("latency_ms");
    assert_eq!(val, Some(42.0), "Should retrieve set metric");

    let missing = state.get_metric("nonexistent");
    assert_eq!(missing, None, "Missing metric should return None");
}

#[tokio::test]
async fn test_concurrent_fault_scenarios() {
    let handles: Vec<_> = (0..4_u64)
        .map(|i| {
            tokio::spawn(async move {
                let scenario = ChaosScenario::new(format!("concurrent-{i}"))
                    .with_fault(FaultType::PacketLoss {
                        loss_rate: i as f64 * 0.05,
                        duration_ms: 50,
                    })
                    .with_timeout(Duration::from_secs(5));

                scenario.run().await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.expect("Task should not panic");
        assert!(result.is_ok(), "Each concurrent scenario should complete");
    }
}

#[tokio::test]
async fn test_fault_scenario_high_packet_loss() {
    let scenario = ChaosScenario::new("high_packet_loss")
        .with_fault(FaultType::PacketLoss {
            loss_rate: 0.50,
            duration_ms: 100,
        })
        .with_timeout(Duration::from_secs(5));

    let result = timeout(Duration::from_secs(10), scenario.run()).await;
    assert!(result.is_ok(), "High packet loss scenario should complete");
}
