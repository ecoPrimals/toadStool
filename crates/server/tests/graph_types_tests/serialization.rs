// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Serialization, clone, and debug tests

use std::time::Duration;
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};

#[test]
fn test_execution_graph_serialization_roundtrip() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::builder("n1", "op")
                .duration_secs(300)
                .metadata("foo", "bar")
                .build(),
            GraphNode::simple("n2", "op2"),
        ])
        .connect("n1", "n2")
        .build();

    let json = serde_json::to_string(&graph).unwrap();
    let restored: ExecutionGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(graph.id, restored.id);
    assert_eq!(graph.nodes.len(), restored.nodes.len());
    assert_eq!(graph.edges.len(), restored.edges.len());
}

#[test]
fn test_graph_node_serialization_with_duration() {
    let node = GraphNode::builder("n", "op").duration_secs(120).build();
    let json = serde_json::to_string(&node).unwrap();
    assert!(json.contains("120"));
    let restored: GraphNode = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.duration, Some(Duration::from_secs(120)));
}

#[test]
fn test_graph_node_serialization_without_duration() {
    let node = GraphNode::simple("n", "op");
    let json = serde_json::to_string(&node).unwrap();
    let restored: GraphNode = serde_json::from_str(&json).unwrap();
    assert!(restored.duration.is_none());
}

#[test]
fn test_graph_edge_serialization_roundtrip() {
    let edge = GraphEdge::data_flow("a", "b");
    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains("data_flow"));
    let restored: GraphEdge = serde_json::from_str(&json).unwrap();
    assert_eq!(edge.from, restored.from);
    assert_eq!(edge.to, restored.to);
    assert_eq!(edge.edge_type, restored.edge_type);
}

#[test]
fn test_edge_type_serialization_snake_case() {
    let json = serde_json::to_string(&EdgeType::DataFlow).unwrap();
    assert_eq!(json, "\"data_flow\"");
    let restored: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, EdgeType::DataFlow);
}

#[test]
fn test_debug_implementations() {
    let graph = ExecutionGraph::simple("g");
    let _ = format!("{graph:?}");

    let node = GraphNode::simple("n", "op");
    let _ = format!("{node:?}");

    let edge = GraphEdge::new("a", "b");
    let _ = format!("{edge:?}");

    let req = NodeResourceRequirements::default();
    let _ = format!("{req:?}");
}

#[test]
fn test_clone_execution_graph() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("a", "op"))
        .build();
    let cloned = graph.clone();
    assert_eq!(graph.id, cloned.id);
}

#[test]
fn test_clone_graph_node() {
    let node = GraphNode::builder("n", "op").cpu(2.0).build();
    let cloned = node.clone();
    assert_eq!(node.id, cloned.id);
    assert_eq!(cloned.requirements.cpu.unwrap().min_cores, 2.0);
}

#[test]
fn test_clone_graph_edge() {
    let edge = GraphEdge::data_flow("a", "b");
    let cloned = edge.clone();
    assert_eq!(edge.from, cloned.from);
    assert_eq!(edge.edge_type, cloned.edge_type);
}

#[test]
fn test_clone_node_resource_requirements() {
    let node = GraphNode::builder("n", "op").cpu(2.0).build();
    let req = node.requirements;
    let cloned = req.clone();
    assert_eq!(cloned.cpu.as_ref().unwrap().min_cores, 2.0);
}

#[test]
fn test_clone_edge_type() {
    let et = EdgeType::Control;
    let cloned = et;
    assert_eq!(et, cloned);
}

#[test]
fn test_graph_node_deserialize_default_primal() {
    let json = r#"{"id":"n","operation":"op"}"#;
    let node: GraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.primal, "toadstool");
}

#[test]
fn test_graph_node_deserialize_duration_null() {
    let json = r#"{"id":"n","operation":"op","duration":null}"#;
    let node: GraphNode = serde_json::from_str(json).unwrap();
    assert!(node.duration.is_none());
}

#[test]
fn test_graph_node_serialize_duration_none_omitted() {
    let node = GraphNode::simple("n", "op");
    let json = serde_json::to_string(&node).unwrap();
    assert!(
        !json.contains("duration"),
        "None duration should be omitted"
    );
}

#[test]
fn test_node_resource_requirements_serialization_roundtrip() {
    let node = GraphNode::builder("n", "op").cpu(4.0).memory_gb(8).build();
    let req = node.requirements;

    let json = serde_json::to_string(&req).unwrap();
    let restored: NodeResourceRequirements = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.cpu.as_ref().unwrap().min_cores, 4.0);
    assert_eq!(
        restored.memory.as_ref().unwrap().min_bytes,
        8 * 1024 * 1024 * 1024
    );
}

#[test]
fn test_graph_edge_with_metadata_serialization() {
    let mut edge = GraphEdge::new("a", "b");
    edge.metadata.insert("bandwidth".into(), "1Gbps".into());

    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains("bandwidth"));
    let restored: GraphEdge = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.metadata.get("bandwidth"),
        Some(&"1Gbps".to_string())
    );
}

#[test]
fn test_edge_type_control_serialization() {
    let json = serde_json::to_string(&EdgeType::Control).unwrap();
    assert_eq!(json, "\"control\"");
    let restored: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, EdgeType::Control);
}

#[test]
fn test_edge_type_dependency_serialization() {
    let json = serde_json::to_string(&EdgeType::Dependency).unwrap();
    assert_eq!(json, "\"dependency\"");
}

#[test]
fn test_execution_graph_deserialize_missing_metadata() {
    let json = r#"{"id":"g1","nodes":[],"edges":[]}"#;
    let graph: ExecutionGraph = serde_json::from_str(json).unwrap();
    assert!(graph.metadata.is_empty());
}
