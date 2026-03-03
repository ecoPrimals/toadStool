// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real Chaos Engineering Test Scenarios
//!
//! These tests validate ToadStool's resilience under failure conditions.
//!
//! Run with: `cargo test --test chaos_engineering_scenarios`

use toadstool_testing::chaos::{ChaosScenario, FaultType, ResourceType};

/// Scenario 1: Network Partition During Execution
///
/// Tests system behavior when network connectivity is lost mid-execution.
/// Expected behavior:
/// - Executions should continue locally
/// - State should be preserved
/// - Recovery should be automatic when network returns
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_network_partition_during_execution() {
    let scenario = ChaosScenario::new("network_partition_during_execution")
        .with_fault(FaultType::NetworkPartition {
            duration_ms: 5000, // 5 second partition
            affected_nodes: vec!["node1".to_string(), "node2".to_string()],
        })
        .with_validation(|state| {
            // Verify system recovered
            if !state.cluster_recovered() {
                return Err("Cluster failed to recover from network partition".to_string());
            }

            // Verify no data loss
            if state.data_loss_count > 0 {
                return Err(format!(
                    "Data loss detected: {} events",
                    state.data_loss_count
                ));
            }

            // Verify recovery occurred
            if state.recovery_count == 0 {
                return Err("No recovery events recorded".to_string());
            }

            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Network partition scenario should pass");

    // Additional assertions
    assert_eq!(result.faults_injected, 1);
    assert!(result.final_state.cluster_recovered());
    println!(
        "✅ Network partition scenario passed: {:?}",
        result.duration
    );
}

/// Scenario 2: Random Executor Crash and Restart
///
/// Tests resilience when executors crash unexpectedly.
/// Expected behavior:
/// - Running workloads should be detected as failed
/// - System should attempt restart
/// - New executions should queue properly
/// - Metrics should be preserved
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_executor_crash_and_recovery() {
    let scenario = ChaosScenario::new("executor_crash_recovery")
        .with_fault(FaultType::ProcessCrash {
            node_id: "executor-1".to_string(),
            restart_delay_ms: 2000, // 2 second restart delay
        })
        .with_delayed_fault(
            FaultType::ProcessCrash {
                node_id: "executor-2".to_string(),
                restart_delay_ms: 1000,
            },
            3000, // Crash second executor after 3 seconds
        )
        .with_validation(|state| {
            // Should have recovered from both crashes
            if state.recovery_count < 2 {
                return Err(format!(
                    "Expected 2 recoveries, got {}",
                    state.recovery_count
                ));
            }

            // All nodes should be back online
            if state.failed_nodes > 0 {
                return Err(format!("{} nodes still failed", state.failed_nodes));
            }

            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Crash recovery scenario should pass");

    assert_eq!(result.faults_injected, 2);
    assert_eq!(result.final_state.recovery_count, 2);
    assert_eq!(result.final_state.failed_nodes, 0);
    println!("✅ Executor crash recovery passed: {:?}", result.duration);
}

/// Scenario 3: Resource Exhaustion (Memory Pressure)
///
/// Tests behavior under memory pressure.
/// Expected behavior:
/// - System should detect low memory
/// - New executions should be throttled or queued
/// - Existing executions should complete if possible
/// - No OOM kills of the orchestrator itself
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_memory_exhaustion() {
    let scenario = ChaosScenario::new("memory_exhaustion")
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Memory,
            consumption_percent: 90, // Consume 90% of available memory
            duration_ms: 10000,      // 10 seconds of pressure
        })
        .with_validation(|state| {
            // System should still be responsive
            if state.active_nodes == 0 {
                return Err("All nodes became unresponsive".to_string());
            }

            // Should have handled the pressure without crashing
            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Memory exhaustion scenario should pass");

    assert!(result.final_state.active_nodes > 0);
    println!(
        "✅ Memory exhaustion handling passed: {:?}",
        result.duration
    );
}

/// Scenario 4: Combined Chaos (Network + Crash + Latency)
///
/// Tests resilience under multiple simultaneous failures.
/// This is the "nightmare scenario" that validates overall system robustness.
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_combined_chaos() {
    let scenario = ChaosScenario::new("combined_chaos_nightmare")
        // Start with network latency
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 500, // 500ms latency
            duration_ms: 8000,
        })
        // Add process crash during latency
        .with_delayed_fault(
            FaultType::ProcessCrash {
                node_id: "executor-1".to_string(),
                restart_delay_ms: 3000,
            },
            2000,
        )
        // Then add network partition
        .with_delayed_fault(
            FaultType::NetworkPartition {
                duration_ms: 3000,
                affected_nodes: vec!["node2".to_string()],
            },
            5000,
        )
        // Finally add packet loss
        .with_delayed_fault(
            FaultType::PacketLoss {
                loss_rate: 0.3, // 30% packet loss
                duration_ms: 2000,
            },
            7000,
        )
        .with_validation(|state| {
            // System should have recovered from everything
            if !state.cluster_recovered() {
                return Err("Cluster failed to recover from combined chaos".to_string());
            }

            // Should have multiple recoveries
            if state.recovery_count < 2 {
                return Err(format!(
                    "Expected multiple recoveries, got {}",
                    state.recovery_count
                ));
            }

            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Combined chaos scenario should pass");

    assert_eq!(result.faults_injected, 4);
    assert!(result.final_state.cluster_recovered());
    println!("✅ Combined chaos nightmare passed: {:?}", result.duration);
}

/// Scenario 5: Sustained Resource Pressure
///
/// Tests long-term stability under resource constraints.
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_sustained_resource_pressure() {
    let scenario = ChaosScenario::new("sustained_resource_pressure")
        // High CPU usage
        .with_fault(FaultType::ResourceExhaustion {
            resource_type: ResourceType::Cpu,
            consumption_percent: 80,
            duration_ms: 15000, // 15 seconds
        })
        // Concurrent network latency
        .with_fault(FaultType::NetworkLatency {
            latency_ms: 200,
            duration_ms: 15000,
        })
        .with_validation(|state| {
            // Should maintain availability
            if state.active_nodes == 0 {
                return Err("System became unavailable under pressure".to_string());
            }

            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Sustained pressure scenario should pass");

    println!(
        "✅ Sustained resource pressure passed: {:?}",
        result.duration
    );
}

/// Scenario 6: Cascading Failures
///
/// Tests behavior when failures trigger more failures (cascade).
#[tokio::test]
#[ignore = "requires live ToadStool cluster"]
async fn test_cascading_failures() {
    let scenario = ChaosScenario::new("cascading_failures")
        // Initial failure
        .with_fault(FaultType::ProcessCrash {
            node_id: "node-1".to_string(),
            restart_delay_ms: 2000,
        })
        // Causes load on node-2, which then fails
        .with_delayed_fault(
            FaultType::ProcessCrash {
                node_id: "node-2".to_string(),
                restart_delay_ms: 2000,
            },
            1000,
        )
        // Causes load on node-3, which experiences memory pressure
        .with_delayed_fault(
            FaultType::ResourceExhaustion {
                resource_type: ResourceType::Memory,
                consumption_percent: 95,
                duration_ms: 3000,
            },
            2000,
        )
        .with_validation(|state| {
            // Should recover from cascade
            if !state.cluster_recovered() {
                return Err("Failed to recover from cascading failures".to_string());
            }

            // Should have recovered multiple times
            if state.recovery_count < 2 {
                return Err("Insufficient recovery events for cascade".to_string());
            }

            Ok(())
        });

    let result = scenario
        .run()
        .await
        .expect("Cascading failures scenario should pass");

    assert!(result.final_state.cluster_recovered());
    println!("✅ Cascading failures handled: {:?}", result.duration);
}
