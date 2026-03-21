// SPDX-License-Identifier: AGPL-3.0-only

use crate::ResourceRequirements;
use crate::songbird_integration::types::*;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn test_performance_metrics_default() {
    let metrics = PerformanceMetrics::default();
    assert_eq!(metrics.request_count(), 0);
    assert_eq!(metrics.mean_latency_ms(), 0.0);
}

#[test]
fn test_message_type_registry_default() {
    let registry = MessageTypeRegistry::default();
    assert!(!registry.is_known("unknown"));
}

#[test]
fn test_broadcast_channel_empty_name() {
    let ch = BroadcastChannel::new("");
    assert_eq!(ch.name(), "");
}

#[test]
fn test_songbird_broadcast_message_all_channel_names() {
    assert_eq!(
        SongbirdBroadcastMessage::CapabilityUpdate {
            node_id: "n".to_string(),
            capabilities: NodeCapabilities {
                cpu_cores: 1.0,
                memory_gb: 1.0,
                storage_gb: 1.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "capability-updates"
    );
    assert_eq!(
        SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n".to_string(),
            health_status: "ok".to_string(),
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "health-updates"
    );
    assert_eq!(
        SongbirdBroadcastMessage::CustomMessage {
            message_type: "custom".to_string(),
            payload: serde_json::Value::Null,
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "custom"
    );
}

#[test]
fn test_execution_metrics_zero_values_serde() {
    let m = ExecutionMetrics {
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        cpu_usage: 0.0,
        memory_usage: 0,
        network_io: 0,
        disk_io: 0,
    };
    let json = serde_json::to_string(&m).unwrap();
    let _: ExecutionMetrics = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_connection_health_partial_eq() {
    assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Degraded);
}

// Re-export path tests (types/mod.rs coverage)

#[test]
fn test_load_balancing_advice_constructor() {
    let advice = LoadBalancingAdvice {
        recommended_nodes: vec!["n1".to_string(), "n2".to_string()],
        load_distribution: {
            let mut m = HashMap::new();
            m.insert("n1".to_string(), 0.6);
            m.insert("n2".to_string(), 0.4);
            m
        },
        reasoning: "Load balanced".to_string(),
    };
    assert_eq!(advice.recommended_nodes.len(), 2);
    assert!(advice.reasoning.contains("Load balanced"));
}

#[test]
fn test_resource_reservation_constructor() {
    let res = ResourceReservation {
        reservation_id: Uuid::new_v4(),
        resources: ResourceRequirements::default(),
    };
    let _ = res.reservation_id;
    assert!(res.resources.cpu.min_cores >= 0.0);
}

#[test]
fn test_network_status_constructor() {
    let status = NetworkStatus {
        total_nodes: 5,
        active_nodes: 4,
        total_capacity: NodeCapabilities {
            cpu_cores: 16.0,
            memory_gb: 32.0,
            storage_gb: 200.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        current_utilization: 0.75,
    };
    assert_eq!(status.total_nodes, 5);
    assert_eq!(status.active_nodes, 4);
    assert!((status.current_utilization - 0.75).abs() < 0.01);
}

#[test]
fn test_registration_response_constructor() {
    let resp = RegistrationResponse {
        node_id: "node-x".to_string(),
        status: "ok".to_string(),
        assigned_channels: vec!["ch1".to_string()],
    };
    assert_eq!(resp.node_id, "node-x");
    assert_eq!(resp.assigned_channels.len(), 1);
}

#[test]
fn test_types_reexport_load_estimator() {
    let est = LoadEstimator::default();
    assert_eq!(est.estimation_model, "linear");
}

#[test]
fn test_types_reexport_distribution_algorithm() {
    let algo = DistributionAlgorithm::RoundRobin;
    assert!(matches!(algo, DistributionAlgorithm::RoundRobin));
}

#[test]
fn test_types_reexport_load_balancing_strategy() {
    let strategy: LoadBalancingStrategy = "least-loaded".to_string();
    assert_eq!(strategy, "least-loaded");
}
