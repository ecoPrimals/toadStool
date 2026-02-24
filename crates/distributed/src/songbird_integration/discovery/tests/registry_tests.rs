//! NodeRegistry, NetworkHealthMonitor, CapabilityTracker tests

use std::time::Duration;

use crate::songbird_integration::types::{
    CapabilityTracker, ConnectionHealth, NetworkHealthMonitor, NodeRegistry, NodeType,
};

use super::make_node_registration;

#[test]
fn test_node_registry_new() {
    let registry = NodeRegistry::new();
    let active = registry.get_active_nodes();
    assert!(active.is_empty());
}

#[test]
fn test_node_registry_register_and_get_active() {
    let mut registry = NodeRegistry::new();
    let reg = make_node_registration("node-1", NodeType::ToadStool, 4.0, 8.0, 100.0);
    registry.register_node(reg).unwrap();
    let active = registry.get_active_nodes();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].node_id, "node-1");
}

#[test]
fn test_node_registry_get_all_nodes() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "a",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .unwrap();
    registry
        .register_node(make_node_registration(
            "b",
            NodeType::BearDog,
            1.0,
            2.0,
            25.0,
        ))
        .unwrap();
    let all = registry.get_all_nodes();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_node_registry_get_nodes_by_types() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "ts",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .unwrap();
    registry
        .register_node(make_node_registration(
            "bd",
            NodeType::BearDog,
            2.0,
            4.0,
            50.0,
        ))
        .unwrap();
    let toadstools = registry.get_nodes_by_types(&[NodeType::ToadStool]);
    assert_eq!(toadstools.len(), 1);
    assert_eq!(toadstools[0].node_id, "ts");
}

#[test]
fn test_node_registry_update_node_health() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "n1",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .unwrap();
    registry.update_node_health(&"n1".to_string(), true);
}

#[test]
fn test_network_health_monitor_new() {
    let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
    let _ = monitor;
}

#[test]
fn test_capability_tracker_new() {
    let tracker = CapabilityTracker::new();
    let _ = tracker;
}

#[test]
fn test_network_health_monitor_clone() {
    let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
    let cloned = monitor.clone();
    assert_eq!(monitor.check_interval, cloned.check_interval);
}

#[test]
fn test_capability_tracker_clone() {
    let tracker = CapabilityTracker::new();
    let _cloned = tracker.clone();
}

#[test]
fn test_network_health_monitor_state_transition_healthy_to_degraded() {
    let mut monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
    let node_id = "n1".to_string();
    monitor.update_node_health(node_id.clone(), ConnectionHealth::Healthy);
    assert_eq!(monitor.get_node_health(&node_id), ConnectionHealth::Healthy);
    monitor.update_node_health(node_id.clone(), ConnectionHealth::Degraded);
    assert_eq!(
        monitor.get_node_health(&node_id),
        ConnectionHealth::Degraded
    );
}

#[test]
fn test_network_health_monitor_state_transition_unhealthy_to_healthy() {
    let mut monitor = NetworkHealthMonitor::with_interval(Duration::from_secs(60));
    let node_id = "recovered".to_string();
    monitor.update_node_health(node_id.clone(), ConnectionHealth::Unhealthy);
    assert_eq!(
        monitor.get_node_health(&node_id),
        ConnectionHealth::Unhealthy
    );
    monitor.update_node_health(node_id.clone(), ConnectionHealth::Healthy);
    assert_eq!(monitor.get_node_health(&node_id), ConnectionHealth::Healthy);
    let healthy = monitor.healthy_nodes();
    assert_eq!(healthy.len(), 1);
    assert_eq!(healthy[0], "recovered");
}

#[test]
fn test_network_health_monitor_remove_node() {
    let mut monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
    monitor.update_node_health("removed".to_string(), ConnectionHealth::Healthy);
    assert_eq!(
        monitor.get_node_health(&"removed".to_string()),
        ConnectionHealth::Healthy
    );
    monitor.remove_node(&"removed".to_string());
    assert_eq!(
        monitor.get_node_health(&"removed".to_string()),
        ConnectionHealth::Unknown
    );
}

#[test]
fn test_network_health_monitor_unknown_for_unregistered_node() {
    let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
    assert_eq!(
        monitor.get_node_health(&"never-registered".to_string()),
        ConnectionHealth::Unknown
    );
}

#[test]
fn test_node_registry_get_nodes_by_types_multiple() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "ts1",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .unwrap();
    registry
        .register_node(make_node_registration(
            "ts2",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .unwrap();
    registry
        .register_node(make_node_registration(
            "ng",
            NodeType::NestGate,
            1.0,
            2.0,
            25.0,
        ))
        .unwrap();
    let toadstools = registry.get_nodes_by_types(&[NodeType::ToadStool]);
    assert_eq!(toadstools.len(), 2);
    let nestgates = registry.get_nodes_by_types(&[NodeType::NestGate]);
    assert_eq!(nestgates.len(), 1);
}

#[test]
fn test_node_registry_get_nodes_by_types_empty_filter() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "x",
            NodeType::ToadStool,
            4.0,
            8.0,
            100.0,
        ))
        .unwrap();
    let result = registry.get_nodes_by_types(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_node_registry_get_node() {
    let mut registry = NodeRegistry::new();
    let reg = make_node_registration("lookup", NodeType::Songbird, 1.0, 2.0, 10.0);
    registry.register_node(reg.clone()).unwrap();
    let found = registry.get_node(&"lookup".to_string());
    assert!(found.is_some());
    assert_eq!(found.unwrap().node_id, "lookup");
}

#[test]
fn test_node_registry_list_nodes() {
    let mut registry = NodeRegistry::new();
    registry
        .register_node(make_node_registration(
            "l1",
            NodeType::ToadStool,
            2.0,
            4.0,
            50.0,
        ))
        .unwrap();
    registry
        .register_node(make_node_registration(
            "l2",
            NodeType::BearDog,
            1.0,
            2.0,
            25.0,
        ))
        .unwrap();
    let list = registry.list_nodes();
    assert_eq!(list.len(), 2);
}
