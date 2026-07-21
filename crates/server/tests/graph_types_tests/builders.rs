// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Builder and constructor tests

use std::time::Duration;
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};

#[test]
fn test_execution_graph_builder() {
    let graph = ExecutionGraph::builder("my-graph")
        .node(GraphNode::simple("n1", "op1"))
        .nodes([
            GraphNode::simple("n2", "op2"),
            GraphNode::simple("n3", "op3"),
        ])
        .connect("n1", "n2")
        .edge(GraphEdge::data_flow("n2", "n3"))
        .metadata("key", "value")
        .build();

    assert_eq!(graph.id, "my-graph");
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.metadata.get("key"), Some(&"value".to_string()));
    assert!(graph.validate().is_ok());
}

#[test]
fn test_execution_graph_simple() {
    let graph = ExecutionGraph::simple("empty-graph");
    assert_eq!(graph.id, "empty-graph");
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

#[test]
fn test_graph_node_simple() {
    let node = GraphNode::simple("my_node", "gpu_compute");
    assert_eq!(node.id, "my_node");
    assert_eq!(node.primal, "toadstool");
    assert_eq!(node.operation, "gpu_compute");
    assert!(node.requirements.cpu.is_none());
    assert!(node.duration.is_none());
    assert!(node.metadata.is_empty());
}

#[test]
fn test_graph_node_builder_minimal() {
    let node = GraphNode::builder("id", "op").build();
    assert_eq!(node.id, "id");
    assert_eq!(node.primal, "toadstool");
    assert_eq!(node.operation, "op");
}

#[test]
fn test_graph_node_builder_full() {
    let node = GraphNode::builder("my_node", "gpu_compute")
        .primal("squirrel")
        .cpu(4.0)
        .memory(1024 * 1024 * 1024)
        .memory_gb(8)
        .gpu_memory(16 * 1024 * 1024 * 1024)
        .gpu_memory_gb(16)
        .storage(100 * 1024 * 1024)
        .storage_gb(100)
        .network_bandwidth(1000)
        .duration(Duration::from_mins(2))
        .duration_secs(60)
        .metadata("model", "gpt-4")
        .build();

    assert_eq!(node.id, "my_node");
    assert_eq!(node.primal, "squirrel");
    assert_eq!(node.operation, "gpu_compute");
    assert_eq!(node.duration, Some(Duration::from_mins(1)));

    let req = node.requirements;
    assert!(req.cpu.is_some());
    assert_eq!(req.cpu.as_ref().unwrap().min_cores, 4.0);

    assert!(req.memory.is_some());
    assert_eq!(
        req.memory.as_ref().unwrap().min_bytes,
        8 * 1024 * 1024 * 1024
    );

    assert!(req.gpu.is_some());
    assert_eq!(
        req.gpu.as_ref().unwrap().min_memory_bytes,
        Some(16 * 1024 * 1024 * 1024)
    );

    assert!(req.storage.is_some());
    assert_eq!(
        req.storage.as_ref().unwrap().min_bytes,
        100 * 1024 * 1024 * 1024
    );

    assert!(req.network.is_some());
    assert_eq!(
        req.network.as_ref().unwrap().min_bandwidth,
        Some(1000 * 125000)
    );

    assert_eq!(node.metadata.get("model"), Some(&"gpt-4".to_string()));
}

#[test]
fn test_graph_node_builder_memory_gb_conversion() {
    let node = GraphNode::builder("n", "op").memory_gb(4).build();
    assert_eq!(
        node.requirements.memory.unwrap().min_bytes,
        4 * 1024 * 1024 * 1024
    );
}

#[test]
fn test_graph_node_builder_storage_gb_conversion() {
    let node = GraphNode::builder("n", "op").storage_gb(2).build();
    assert_eq!(
        node.requirements.storage.unwrap().min_bytes,
        2 * 1024 * 1024 * 1024
    );
}

#[test]
fn test_graph_node_builder_metadata() {
    let node = GraphNode::builder("n", "op")
        .metadata("k1", "v1")
        .metadata("k2", "v2")
        .build();
    assert_eq!(node.metadata.get("k1"), Some(&"v1".to_string()));
    assert_eq!(node.metadata.get("k2"), Some(&"v2".to_string()));
}

#[test]
fn test_graph_edge_new() {
    let edge = GraphEdge::new("a", "b");
    assert_eq!(edge.from, "a");
    assert_eq!(edge.to, "b");
    assert_eq!(edge.edge_type, EdgeType::Dependency);
}

#[test]
fn test_graph_edge_data_flow() {
    let edge = GraphEdge::data_flow("src", "dst");
    assert_eq!(edge.edge_type, EdgeType::DataFlow);
}

#[test]
fn test_graph_edge_control() {
    let edge = GraphEdge::control("ctrl", "target");
    assert_eq!(edge.edge_type, EdgeType::Control);
}

#[test]
fn test_graph_edge_with_strings() {
    let edge = GraphEdge::new(String::from("x"), String::from("y"));
    assert_eq!(edge.from, "x");
    assert_eq!(edge.to, "y");
}

#[test]
fn test_edge_type_default() {
    let et: EdgeType = EdgeType::default();
    assert_eq!(et, EdgeType::Dependency);
}

#[test]
fn test_edge_type_variants() {
    assert_eq!(EdgeType::DataFlow, EdgeType::DataFlow);
    assert_eq!(EdgeType::Control, EdgeType::Control);
    assert_eq!(EdgeType::Dependency, EdgeType::Dependency);
}

#[test]
fn test_node_resource_requirements_default() {
    let req = NodeResourceRequirements::default();
    assert!(req.cpu.is_none());
    assert!(req.memory.is_none());
    assert!(req.storage.is_none());
    assert!(req.gpu.is_none());
    assert!(req.network.is_none());
}

#[test]
fn test_execution_graph_builder_edges() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("a", "op1"),
            GraphNode::simple("b", "op2"),
            GraphNode::simple("c", "op3"),
        ])
        .edges([
            GraphEdge::new("a", "b"),
            GraphEdge::new("b", "c"),
            GraphEdge::control("a", "c"),
        ])
        .build();

    assert_eq!(graph.edges.len(), 3);
    assert!(graph.validate().is_ok());
}

#[test]
fn test_graph_node_builder_memory_bytes() {
    let node = GraphNode::builder("n", "op")
        .memory(1024 * 1024 * 1024)
        .build();
    assert_eq!(
        node.requirements.memory.as_ref().unwrap().min_bytes,
        1024 * 1024 * 1024
    );
}

#[test]
fn test_graph_node_builder_gpu_memory_bytes() {
    let node = GraphNode::builder("n", "op")
        .gpu_memory(8 * 1024 * 1024 * 1024)
        .build();
    assert_eq!(
        node.requirements.gpu.as_ref().unwrap().min_memory_bytes,
        Some(8 * 1024 * 1024 * 1024)
    );
}

#[test]
fn test_graph_node_builder_storage_bytes() {
    let node = GraphNode::builder("n", "op")
        .storage(500 * 1024 * 1024)
        .build();
    assert_eq!(
        node.requirements.storage.as_ref().unwrap().min_bytes,
        500 * 1024 * 1024
    );
}

#[test]
fn test_graph_node_builder_duration_direct() {
    let node = GraphNode::builder("n", "op")
        .duration(Duration::from_secs(90))
        .build();
    assert_eq!(node.duration, Some(Duration::from_secs(90)));
}
