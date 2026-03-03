// SPDX-License-Identifier: AGPL-3.0-or-later
//! SongbirdNetworkDiscovery and type serialization tests

use crate::songbird_integration::types::{
    CoordinationStrategy, DistributionPlan, NetworkCapacity, NodeRegistration, NodeType,
    RegistrationResponse, SubTask, SubTaskPlan,
};
use crate::types::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
use crate::types::ResourceRequirements;

use super::{make_discovery, make_node_registration};

fn make_subtask() -> SubTask {
    SubTask {
        id: uuid::Uuid::new_v4(),
        payload: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 0,
        constraints: vec![],
    }
}

#[test]
fn test_songbird_discovery_for_test() {
    let (discovery, _) = make_discovery();
    let _ = discovery;
}

#[tokio::test]
async fn test_songbird_discovery_register_node_success() {
    let (discovery, _) = make_discovery();
    let reg = make_node_registration("reg-node", NodeType::ToadStool, 4.0, 8.0, 100.0);
    let result = discovery.register_node(reg).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.node_id, "reg-node");
    assert_eq!(resp.status, "registered");
}

#[tokio::test]
async fn test_songbird_discovery_register_node_empty_id() {
    let (discovery, _) = make_discovery();
    let mut reg = make_node_registration("x", NodeType::ToadStool, 4.0, 8.0, 100.0);
    reg.node_id = String::new();
    let result = discovery.register_node(reg).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("empty"));
    }
}

#[tokio::test]
async fn test_songbird_discovery_register_node_empty_endpoints() {
    let (discovery, _) = make_discovery();
    let mut reg = make_node_registration("x", NodeType::ToadStool, 4.0, 8.0, 100.0);
    reg.endpoints = vec![];
    let result = discovery.register_node(reg).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("endpoint"));
    }
}

#[tokio::test]
async fn test_songbird_discovery_get_network_capacity_empty() {
    let (discovery, _) = make_discovery();
    let capacity = discovery.get_network_capacity().await.unwrap();
    assert_eq!(capacity.total_nodes, 0);
    assert_eq!(capacity.total_cpu_cores, 0.0);
    assert_eq!(capacity.total_memory_gb, 0.0);
}

#[tokio::test]
async fn test_songbird_discovery_get_network_capacity_with_nodes() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "n1",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "n2",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    let capacity = discovery.get_network_capacity().await.unwrap();
    assert_eq!(capacity.total_nodes, 2);
    assert!((capacity.total_cpu_cores - 6.0).abs() < 0.01);
    assert!((capacity.total_memory_gb - 12.0).abs() < 0.01);
}

#[tokio::test]
async fn test_songbird_discovery_get_optimal_distribution_no_nodes() {
    let (discovery, _) = make_discovery();
    let subtask = make_subtask();
    let result = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("suitable") || e.to_string().contains("distribution"));
    }
}

#[tokio::test]
async fn test_songbird_discovery_get_network_status() {
    let (discovery, _) = make_discovery();
    let status = discovery.get_network_status().await.unwrap();
    assert_eq!(status.total_nodes, 0);
    assert_eq!(status.active_nodes, 0);
}

#[test]
fn test_node_type_serialization() {
    let t = NodeType::ToadStool;
    let json = serde_json::to_string(&t).unwrap();
    let restored: NodeType = serde_json::from_str(&json).unwrap();
    assert!(matches!(restored, NodeType::ToadStool));
    let t2 = NodeType::Custom("my-type".to_string());
    let json2 = serde_json::to_string(&t2).unwrap();
    let restored2: NodeType = serde_json::from_str(&json2).unwrap();
    assert!(matches!(restored2, NodeType::Custom(s) if s == "my-type"));
}

#[test]
fn test_node_registration_serialization_roundtrip() {
    let reg = make_node_registration("ser-node", NodeType::NestGate, 2.0, 4.0, 50.0);
    let json = serde_json::to_string(&reg).unwrap();
    let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.node_id, reg.node_id);
    assert!(matches!(parsed.node_type, NodeType::NestGate));
}

#[test]
fn test_network_capacity_default_values() {
    let cap = NetworkCapacity {
        total_nodes: 0,
        total_cpu_cores: 0.0,
        total_memory_gb: 0.0,
        total_storage_gb: 0.0,
    };
    assert_eq!(cap.total_nodes, 0);
    assert_eq!(cap.total_storage_gb, 0.0);
}

#[test]
fn test_registration_response_structure() {
    let resp = RegistrationResponse {
        node_id: "resp-node".to_string(),
        status: "registered".to_string(),
        assigned_channels: vec!["global".to_string(), "type_ToadStool".to_string()],
    };
    assert_eq!(resp.node_id, "resp-node");
    assert_eq!(resp.assigned_channels.len(), 2);
}

#[tokio::test]
async fn test_songbird_discovery_get_optimal_distribution_with_nodes() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "n1",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    let subtask = SubTask {
        id: uuid::Uuid::new_v4(),
        payload: vec![1, 2, 3],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 0,
        constraints: vec![],
    };
    let result = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await;
    assert!(result.is_ok());
    let plan = result.unwrap();
    assert_eq!(plan.subtasks.len(), 1);
    assert!(matches!(
        plan.coordination_strategy,
        CoordinationStrategy::Parallel
    ));
}

#[tokio::test]
async fn test_songbird_discovery_get_network_status_with_nodes() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "stat-node",
            NodeType::ToadStool,
            8.0,
            16.0,
            200.0,
        ))
        .await
        .unwrap();
    let status = discovery.get_network_status().await.unwrap();
    assert_eq!(status.total_nodes, 1);
    assert_eq!(status.active_nodes, 1);
    assert!((status.total_capacity.cpu_cores - 8.0).abs() < 0.01);
    assert!((status.total_capacity.memory_gb - 16.0).abs() < 0.01);
}

#[tokio::test]
async fn test_songbird_discovery_clone_creates_fresh_registry() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "clone-test",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    let cloned = discovery.clone();
    let capacity = cloned.get_network_capacity().await.unwrap();
    assert_eq!(capacity.total_nodes, 0, "Clone has fresh empty registry");
}

#[test]
fn test_subtask_plan_structure() {
    let plan = SubTaskPlan {
        subtask_id: uuid::Uuid::new_v4(),
        target_nodes: vec!["node-1".to_string()],
        resource_allocation: ResourceRequirements::default(),
        dependencies: vec![],
    };
    assert_eq!(plan.target_nodes.len(), 1);
}

#[test]
fn test_distribution_plan_structure() {
    let plan = DistributionPlan {
        plan_id: uuid::Uuid::new_v4(),
        job_id: uuid::Uuid::new_v4(),
        subtasks: vec![],
        coordination_strategy: CoordinationStrategy::Parallel,
    };
    assert!(plan.subtasks.is_empty());
}

#[tokio::test]
async fn test_node_registry_register_direct() {
    use crate::songbird_integration::types::NodeRegistry;
    let mut registry = NodeRegistry::new();
    let reg = make_node_registration("direct-reg", NodeType::Songbird, 1.0, 2.0, 10.0);
    registry.register(reg);
    assert_eq!(registry.get_all_nodes().len(), 1);
}

#[tokio::test]
async fn test_get_network_capacity_single_node() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "single",
            NodeType::NestGate,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    let cap = discovery.get_network_capacity().await.unwrap();
    assert_eq!(cap.total_nodes, 1);
    assert!((cap.total_cpu_cores - 2.0).abs() < 0.01);
}

#[tokio::test]
async fn test_node_registration_tracking_multiple_registrations() {
    let (discovery, _) = make_discovery();
    for i in 0..5 {
        discovery
            .register_node(make_node_registration(
                &format!("node-{}", i),
                NodeType::ToadStool,
                4.0 + i as f64,
                8.0,
                100.0,
            ))
            .await
            .unwrap();
    }
    let capacity = discovery.get_network_capacity().await.unwrap();
    assert_eq!(capacity.total_nodes, 5);
    assert!((capacity.total_cpu_cores - (4.0 + 5.0 + 6.0 + 7.0 + 8.0)).abs() < 0.01);
}

#[tokio::test]
async fn test_node_registration_overwrites_same_node_id() {
    use crate::songbird_integration::types::NodeRegistry;
    let mut registry = NodeRegistry::new();
    let reg1 = make_node_registration("dup-node", NodeType::ToadStool, 2.0, 4.0, 50.0);
    let reg2 = make_node_registration("dup-node", NodeType::BearDog, 8.0, 16.0, 200.0);
    registry.register_node(reg1).unwrap();
    registry.register_node(reg2).unwrap();
    let nodes = registry.get_active_nodes();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node_id, "dup-node");
    assert!(matches!(nodes[0].node_type, NodeType::BearDog));
}

#[tokio::test]
async fn test_get_optimal_distribution_subtask_specialized_hardware_bonus() {
    let (discovery, _) = make_discovery();
    let mut reg = make_node_registration("gpu-node", NodeType::ToadStool, 8.0, 32.0, 500.0);
    reg.capabilities.specialized_hardware = vec!["nvidia".to_string()];
    discovery.register_node(reg).await.unwrap();
    let subtask = SubTask {
        id: uuid::Uuid::new_v4(),
        payload: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 0,
        constraints: vec!["nvidia".to_string()],
    };
    let result = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_optimal_distribution_multiple_subtasks() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "n1",
            NodeType::ToadStool,
            8.0,
            16.0,
            200.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "n2",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    let subtasks = vec![
        SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![1],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        },
        SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![2],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        },
    ];
    let result = discovery
        .get_optimal_distribution(&subtasks, &[NodeType::ToadStool])
        .await;
    assert!(result.is_ok());
    let plan = result.unwrap();
    assert_eq!(plan.subtasks.len(), 2);
}

#[tokio::test]
async fn test_capability_matching_prefers_node_with_specialized_hardware() {
    let (discovery, _) = make_discovery();
    let mut gpu_node = make_node_registration("gpu-node", NodeType::ToadStool, 8.0, 32.0, 500.0);
    gpu_node.capabilities.specialized_hardware = vec!["nvidia".to_string()];
    discovery.register_node(gpu_node).await.unwrap();
    let mut cpu_node = make_node_registration("cpu-only", NodeType::ToadStool, 8.0, 32.0, 500.0);
    cpu_node.capabilities.specialized_hardware = vec![];
    discovery.register_node(cpu_node).await.unwrap();
    let subtask = SubTask {
        id: uuid::Uuid::new_v4(),
        payload: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 0,
        constraints: vec!["nvidia".to_string()],
    };
    let plan = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await
        .unwrap();
    assert_eq!(plan.subtasks[0].target_nodes[0], "gpu-node");
}

#[tokio::test]
async fn test_capability_matching_selects_higher_cpu_for_excess_ratio_bonus() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "small",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "large",
            NodeType::ToadStool,
            16.0,
            32.0,
            500.0,
        ))
        .await
        .unwrap();
    let subtask = make_subtask();
    let plan = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await
        .unwrap();
    assert_eq!(plan.subtasks[0].target_nodes[0], "large");
}

#[tokio::test]
async fn test_network_topology_capacity_aggregation() {
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
            NodeType::NestGate,
            4.0,
            8.0,
            100.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "c",
            NodeType::BearDog,
            1.0,
            2.0,
            25.0,
        ))
        .await
        .unwrap();
    let capacity = discovery.get_network_capacity().await.unwrap();
    assert_eq!(capacity.total_nodes, 3);
    assert!((capacity.total_cpu_cores - 7.0).abs() < 0.01);
    assert!((capacity.total_memory_gb - 14.0).abs() < 0.01);
    assert!((capacity.total_storage_gb - 175.0).abs() < 0.01);
}

#[tokio::test]
async fn test_recovery_from_node_failure_update_health_on_existing_node() {
    use crate::songbird_integration::types::NodeRegistry;
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "failing-node",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .unwrap();
    registry.update_node_health(&"failing-node".to_string(), false);
    let nodes = registry.get_active_nodes();
    assert_eq!(nodes.len(), 1);
    registry.update_node_health(&"failing-node".to_string(), true);
    let nodes_after = registry.get_active_nodes();
    assert_eq!(nodes_after.len(), 1);
}

#[tokio::test]
async fn test_recovery_update_health_on_nonexistent_node_no_panic() {
    use crate::songbird_integration::types::NodeRegistry;
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "exists",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .unwrap();
    registry.update_node_health(&"nonexistent".to_string(), true);
    registry.update_node_health(&"nonexistent".to_string(), false);
    let nodes = registry.get_active_nodes();
    assert_eq!(nodes.len(), 1);
}

#[tokio::test]
async fn test_get_optimal_distribution_prefers_nestgate_when_requested() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "ts",
            NodeType::ToadStool,
            8.0,
            16.0,
            200.0,
        ))
        .await
        .unwrap();
    discovery
        .register_node(make_node_registration(
            "ng",
            NodeType::NestGate,
            8.0,
            16.0,
            200.0,
        ))
        .await
        .unwrap();
    let subtask = make_subtask();
    let plan = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::NestGate])
        .await
        .unwrap();
    assert_eq!(plan.subtasks[0].target_nodes[0], "ng");
}

#[tokio::test]
async fn test_registration_response_assigned_channels_include_type() {
    let (discovery, _) = make_discovery();
    let reg = make_node_registration("channels-test", NodeType::BearDog, 2.0, 4.0, 50.0);
    let resp = discovery.register_node(reg).await.unwrap();
    assert!(resp.assigned_channels.contains(&"global".to_string()));
    assert!(resp.assigned_channels.iter().any(|c| c.contains("BearDog")));
}

#[tokio::test]
async fn test_find_best_node_no_node_meets_requirements_returns_error() {
    let (discovery, _) = make_discovery();
    discovery
        .register_node(make_node_registration(
            "weak",
            NodeType::ToadStool,
            1.0,
            1.0,
            10.0,
        ))
        .await
        .unwrap();
    let subtask = SubTask {
        id: uuid::Uuid::new_v4(),
        payload: vec![],
        resource_requirements: ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 16.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 64 * 1024 * 1024 * 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 100 * 1024 * 1024 * 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        },
        priority: 0,
        constraints: vec![],
    };
    let result = discovery
        .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
        .await;
    assert!(result.is_err());
}
