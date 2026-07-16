// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inline tests extracted from `coordination/discovery/core.rs` (S333).

use bytes::Bytes;
use uuid::Uuid;

use crate::ResourceRequirements;
use crate::coordination::types::{NodeType, SubTask};
use crate::types::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements as NetReq, StorageRequirements,
};

use super::{make_discovery, make_node_registration};

fn make_registration_with_hw(
    node_id: &str,
    node_type: NodeType,
    cpu: f64,
    memory_gb: f64,
    storage_gb: f64,
    hw: Vec<String>,
) -> crate::coordination::types::NodeRegistration {
    let mut reg = make_node_registration(node_id, node_type, cpu, memory_gb, storage_gb);
    reg.capabilities.specialized_hardware = hw.clone();
    reg.metadata.capabilities.specialized_hardware = hw;
    reg
}

fn sample_subtask(
    min_cores: f64,
    memory_bytes: u64,
    storage_bytes: u64,
    constraints: Vec<String>,
) -> SubTask {
    SubTask {
        id: Uuid::new_v4(),
        payload: Bytes::new(),
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: memory_bytes,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: storage_bytes,
                max_bytes: None,
            },
            network: NetReq {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 1,
        constraints,
    }
}

#[tokio::test]
async fn register_node_rejects_empty_node_id() {
    let (discovery, _) = make_discovery();
    let mut reg = make_node_registration("", NodeType::ToadStool, 4.0, 8.0, 100.0);
    reg.node_id = String::new();
    match discovery.register_node(reg).await {
        Err(e) => assert!(e.to_string().contains("Node ID cannot be empty"), "{e}"),
        Ok(_) => panic!("expected empty node id error"),
    }
}

#[tokio::test]
async fn register_node_rejects_empty_endpoints() {
    let (discovery, _) = make_discovery();
    let mut reg = make_node_registration("n1", NodeType::ToadStool, 4.0, 8.0, 100.0);
    reg.endpoints.clear();
    match discovery.register_node(reg).await {
        Err(e) => assert!(e.to_string().contains("At least one endpoint"), "{e}"),
        Ok(_) => panic!("expected empty endpoints error"),
    }
}

#[tokio::test]
async fn get_network_capacity_sums_active_nodes() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "a",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "b",
            NodeType::ToadStool,
            3.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();

    let cap = discovery.get_network_capacity().await.unwrap();
    assert_eq!(cap.total_nodes, 2);
    assert!((cap.total_cpu_cores - 5.0).abs() < f64::EPSILON);
    assert!((cap.total_memory_gb - 12.0).abs() < f64::EPSILON);
    assert!((cap.total_storage_gb - 150.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn get_optimal_distribution_assigns_best_scoring_node() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "weak",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_registration_with_hw(
            "strong",
            NodeType::ToadStool,
            16.0,
            64.0,
            500.0,
            vec!["gpu-a100".to_string()],
        ))
        .await
        .unwrap();

    let sub = sample_subtask(
        4.0,
        8 * 1024 * 1024 * 1024,
        100 * 1024 * 1024 * 1024,
        vec!["gpu-a100".to_string()],
    );
    let plan = discovery
        .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
        .await
        .unwrap();
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].target_nodes, vec!["strong".to_string()]);
}

#[tokio::test]
async fn get_optimal_distribution_errors_when_no_nodes_for_preferred_types() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "store",
            NodeType::Storage,
            8.0,
            32.0,
            1000.0,
        ))
        .await
        .unwrap();

    let sub = sample_subtask(1.0, 1024, 1024, vec![]);
    let err = discovery
        .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("No suitable nodes found"), "{err}");
}

#[tokio::test]
async fn get_optimal_distribution_errors_when_no_node_meets_requirements() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "tiny",
            NodeType::ToadStool,
            1.0,
            0.5,
            0.5,
        ))
        .await
        .unwrap();

    let sub = sample_subtask(
        64.0,
        1024 * 1024 * 1024 * 1024,
        1024 * 1024 * 1024 * 1024,
        vec![],
    );
    let err = discovery
        .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("No suitable node found for subtask"),
        "{err}"
    );
}

#[tokio::test]
async fn clone_creates_fresh_empty_node_registry() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "only",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    let cloned = discovery.clone();
    let cap = cloned.get_network_capacity().await.unwrap();
    assert_eq!(cap.total_nodes, 0);
}

#[tokio::test]
async fn register_node_returns_assigned_channels() {
    let (discovery, _) = make_discovery();
    let resp = discovery
        .register_node(make_node_registration(
            "gate-1",
            NodeType::ToadStool,
            8.0,
            32.0,
            200.0,
        ))
        .await
        .unwrap();
    assert_eq!(resp.node_id, "gate-1");
    assert_eq!(resp.status, "registered");
    assert!(resp.assigned_channels.contains(&"global".to_string()));
}

#[tokio::test]
async fn get_network_status_reports_active_nodes() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "n1",
            NodeType::ToadStool,
            4.0,
            16.0,
            100.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "n2",
            NodeType::Storage,
            2.0,
            8.0,
            500.0,
        ))
        .await
        .unwrap();

    let status = discovery.get_network_status().await.unwrap();
    assert_eq!(status.total_nodes, 2);
    assert_eq!(status.active_nodes, 2);
    assert!((status.total_capacity.cpu_cores - 6.0).abs() < f64::EPSILON);
    assert!((status.total_capacity.memory_gb - 24.0).abs() < f64::EPSILON);
    assert!(status.current_utilization >= 0.0);
    assert!(status.current_utilization <= 1.0);
}

#[tokio::test]
async fn get_network_capacity_empty_returns_zero() {
    let (discovery, _) = make_discovery();
    let cap = discovery.get_network_capacity().await.unwrap();
    assert_eq!(cap.total_nodes, 0);
    assert!((cap.total_cpu_cores).abs() < f64::EPSILON);
}

#[tokio::test]
async fn get_network_status_empty_returns_zero_utilization() {
    let (discovery, _) = make_discovery();
    let status = discovery.get_network_status().await.unwrap();
    assert_eq!(status.total_nodes, 0);
    assert_eq!(status.active_nodes, 0);
    assert!((status.current_utilization).abs() < f64::EPSILON);
}

#[tokio::test]
async fn get_optimal_distribution_multiple_subtasks() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_registration_with_hw(
            "compute",
            NodeType::ToadStool,
            16.0,
            64.0,
            500.0,
            vec!["gpu-v100".to_string()],
        ))
        .await
        .unwrap();

    let sub1 = sample_subtask(2.0, 1024 * 1024, 1024 * 1024, vec![]);
    let sub2 = sample_subtask(4.0, 2 * 1024 * 1024, 2 * 1024 * 1024, vec![]);
    let plan = discovery
        .get_optimal_distribution(&[sub1, sub2], &[NodeType::ToadStool])
        .await
        .unwrap();
    assert_eq!(plan.subtasks.len(), 2);
    assert!(plan.subtasks.iter().all(|s| s.target_nodes[0] == "compute"));
}

#[tokio::test]
async fn register_multiple_node_types() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "toad",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "store",
            NodeType::Storage,
            2.0,
            4.0,
            1000.0,
        ))
        .await
        .unwrap();

    let status = discovery.get_network_status().await.unwrap();
    assert_eq!(status.total_nodes, 2);
    assert_eq!(status.total_capacity.gpu_count, 0);
    assert!((status.total_capacity.storage_gb - 1100.0).abs() < f64::EPSILON);
}
