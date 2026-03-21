// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! ExecutionGraph methods and get_dependencies tests

use std::collections::HashMap;
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};

#[test]
fn test_get_dependencies() {
    let graph = ExecutionGraph {
        id: "test-graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-2".to_string(),
                primal: "toadstool".to_string(),
                operation: "gpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-3".to_string(),
                primal: "toadstool".to_string(),
                operation: "storage".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
        ],
        edges: vec![
            GraphEdge {
                from: "node-1".to_string(),
                to: "node-3".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
            GraphEdge {
                from: "node-2".to_string(),
                to: "node-3".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
        ],
        metadata: HashMap::new(),
    };

    let deps = graph.get_dependencies("node-3");
    assert_eq!(deps.len(), 2);
    assert!(deps.iter().any(|n| n.id == "node-1"));
    assert!(deps.iter().any(|n| n.id == "node-2"));
}

#[test]
fn test_get_dependencies_empty() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("a", "op"))
        .build();
    let deps = graph.get_dependencies("a");
    assert!(deps.is_empty());
}

#[test]
fn test_get_dependencies_single() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([GraphNode::simple("a", "op1"), GraphNode::simple("b", "op2")])
        .connect("a", "b")
        .build();
    let deps = graph.get_dependencies("b");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].id, "a");
}

#[test]
fn test_get_dependencies_multiple() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("node-1", "op1"),
            GraphNode::simple("node-2", "op2"),
            GraphNode::simple("node-3", "op3"),
        ])
        .connect("node-1", "node-3")
        .connect("node-2", "node-3")
        .build();

    let deps = graph.get_dependencies("node-3");
    assert_eq!(deps.len(), 2);
    assert!(deps.iter().any(|n| n.id == "node-1"));
    assert!(deps.iter().any(|n| n.id == "node-2"));
}

#[test]
fn test_get_node_found() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("a", "op1"))
        .node(GraphNode::simple("b", "op2"))
        .connect("a", "b")
        .build();

    let node = graph.get_node("a");
    assert!(node.is_some());
    let node = node.unwrap();
    assert_eq!(node.id, "a");
    assert_eq!(node.operation, "op1");
}

#[test]
fn test_get_node_not_found() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("a", "op1"))
        .build();

    assert!(graph.get_node("nonexistent").is_none());
}

#[test]
fn test_get_dependents() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("a", "op1"),
            GraphNode::simple("b", "op2"),
            GraphNode::simple("c", "op3"),
        ])
        .connect("a", "b")
        .connect("a", "c")
        .build();

    let dependents = graph.get_dependents("a");
    assert_eq!(dependents.len(), 2);
    let ids: Vec<&str> = dependents.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
}

#[test]
fn test_get_dependents_empty() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([GraphNode::simple("a", "op1"), GraphNode::simple("b", "op2")])
        .connect("a", "b")
        .build();

    let dependents = graph.get_dependents("b");
    assert!(dependents.is_empty());
}

#[test]
fn test_dfs_node_without_neighbors() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([GraphNode::simple("sink", "op")])
        .build();

    assert!(graph.validate().is_ok());
    let node = graph.get_node("sink").unwrap();
    assert!(node.id == "sink");
}

#[test]
fn test_dfs_black_node_skip() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("a", "op1"),
            GraphNode::simple("b", "op2"),
            GraphNode::simple("c", "op3"),
        ])
        .connect("a", "b")
        .connect("a", "c")
        .connect("c", "b")
        .build();

    assert!(graph.validate().is_ok(), "diamond with cross-edge is DAG");
}
