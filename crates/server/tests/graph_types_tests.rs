// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Unit tests for execution graph types (`ExecutionGraph`, `GraphNode`, `GraphEdge`, etc.)
//!
//! Extracted from `graph_types.rs` to reduce file size and improve maintainability.
//! Tests validation, builders, serialization, and edge cases.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, GraphValidationError, NodeResourceRequirements,
};

// ───── Validation ───────────────────────────────────────────────────────────

#[test]
fn test_valid_graph() {
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
        ],
        edges: vec![GraphEdge {
            from: "node-1".to_string(),
            to: "node-2".to_string(),
            edge_type: EdgeType::DataFlow,
            metadata: HashMap::new(),
        }],
        metadata: HashMap::new(),
    };

    assert!(graph.validate().is_ok());
}

#[test]
fn test_empty_graph() {
    let graph = ExecutionGraph {
        id: "test-graph".to_string(),
        nodes: vec![],
        edges: vec![],
        metadata: HashMap::new(),
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphValidationError::EmptyGraph)
    ));
}

#[test]
fn test_duplicate_node_id() {
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
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "gpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
        ],
        edges: vec![],
        metadata: HashMap::new(),
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphValidationError::DuplicateNodeId(_))
    ));
}

#[test]
fn test_cycle_detection() {
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
        ],
        edges: vec![
            GraphEdge {
                from: "node-1".to_string(),
                to: "node-2".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
            GraphEdge {
                from: "node-2".to_string(),
                to: "node-1".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
        ],
        metadata: HashMap::new(),
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphValidationError::CycleDetected(_))
    ));
}

#[test]
fn test_self_edge() {
    let graph = ExecutionGraph {
        id: "test-graph".to_string(),
        nodes: vec![GraphNode {
            id: "node-1".to_string(),
            primal: "toadstool".to_string(),
            operation: "cpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements::default(),
            metadata: HashMap::new(),
        }],
        edges: vec![GraphEdge {
            from: "node-1".to_string(),
            to: "node-1".to_string(),
            edge_type: EdgeType::DataFlow,
            metadata: HashMap::new(),
        }],
        metadata: HashMap::new(),
    };

    assert!(matches!(
        graph.validate(),
        Err(GraphValidationError::SelfEdge(_))
    ));
}

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

// ───── ExecutionGraph methods ─────────────────────────────────────────────

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
fn test_invalid_edge_source_not_found() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("b", "op"))
        .connect("nonexistent", "b")
        .build();

    let err = graph.validate().unwrap_err();
    match &err {
        GraphValidationError::InvalidEdge { from, to, reason } => {
            assert_eq!(from, "nonexistent");
            assert_eq!(to, "b");
            assert!(reason.contains("Source node"));
        }
        _ => panic!("expected InvalidEdge, got {err:?}"),
    }
}

#[test]
fn test_invalid_edge_target_not_found() {
    let graph = ExecutionGraph::builder("g1")
        .node(GraphNode::simple("a", "op"))
        .connect("a", "nonexistent")
        .build();

    let err = graph.validate().unwrap_err();
    match &err {
        GraphValidationError::InvalidEdge { from, to, reason } => {
            assert_eq!(from, "a");
            assert_eq!(to, "nonexistent");
            assert!(reason.contains("Target node"));
        }
        _ => panic!("expected InvalidEdge, got {err:?}"),
    }
}

#[test]
fn test_cycle_three_nodes() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("a", "op1"),
            GraphNode::simple("b", "op2"),
            GraphNode::simple("c", "op3"),
        ])
        .connect("a", "b")
        .connect("b", "c")
        .connect("c", "a")
        .build();

    let err = graph.validate().unwrap_err();
    match &err {
        GraphValidationError::CycleDetected(path) => {
            assert_eq!(path.len(), 2, "cycle path is back-edge (from, to)");
            let has_a = path.iter().any(|s| s == "a");
            let has_c = path.iter().any(|s| s == "c");
            assert!(
                has_a && has_c,
                "path {path:?} should involve cycle nodes a,c"
            );
        }
        _ => panic!("expected CycleDetected, got {err:?}"),
    }
}

#[test]
fn test_valid_dag_with_fork_and_join() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([
            GraphNode::simple("source", "op1"),
            GraphNode::simple("a", "op2"),
            GraphNode::simple("b", "op3"),
            GraphNode::simple("sink", "op4"),
        ])
        .connect("source", "a")
        .connect("source", "b")
        .connect("a", "sink")
        .connect("b", "sink")
        .build();

    assert!(graph.validate().is_ok());
}

// ───── ExecutionGraph builder ────────────────────────────────────────────

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

// ───── GraphNode builder and simple ──────────────────────────────────────

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
        .duration(Duration::from_secs(120))
        .duration_secs(60)
        .metadata("model", "gpt-4")
        .build();

    assert_eq!(node.id, "my_node");
    assert_eq!(node.primal, "squirrel");
    assert_eq!(node.operation, "gpu_compute");
    assert_eq!(node.duration, Some(Duration::from_secs(60)));

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

// ───── GraphEdge constructors ─────────────────────────────────────────────

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

// ───── EdgeType ───────────────────────────────────────────────────────────

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

// ───── NodeResourceRequirements ──────────────────────────────────────────

#[test]
fn test_node_resource_requirements_default() {
    let req = NodeResourceRequirements::default();
    assert!(req.cpu.is_none());
    assert!(req.memory.is_none());
    assert!(req.storage.is_none());
    assert!(req.gpu.is_none());
    assert!(req.network.is_none());
}

// ───── GraphValidationError Display ──────────────────────────────────────

#[test]
fn test_graph_validation_error_display() {
    assert_eq!(
        GraphValidationError::EmptyGraph.to_string(),
        "Graph is empty (no nodes)"
    );
    assert!(GraphValidationError::DuplicateNodeId("x".into())
        .to_string()
        .contains("Duplicate node ID"));
    assert!(GraphValidationError::SelfEdge("n".into())
        .to_string()
        .contains("Self-edge"));
    assert!(GraphValidationError::InvalidEdge {
        from: "a".into(),
        to: "b".into(),
        reason: "test".into(),
    }
    .to_string()
    .contains("Invalid edge"));
    assert!(
        GraphValidationError::CycleDetected(vec!["a".into(), "b".into()])
            .to_string()
            .contains("Cycle detected")
    );
}

// ───── Serialization round-trips ──────────────────────────────────────────

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

// ───── Debug trait ────────────────────────────────────────────────────────

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

// ───── Clone ──────────────────────────────────────────────────────────────

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

// ───── ExecutionGraphBuilder.edges() ─────────────────────────────────────

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

// ───── GraphNodeBuilder individual methods ──────────────────────────────

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

// ───── DFS Color::Black branch (diamond / cross-edge) ─────────────────────

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

// ───── default_primal via deserialization ─────────────────────────────────

#[test]
fn test_graph_node_deserialize_default_primal() {
    let json = r#"{"id":"n","operation":"op"}"#;
    let node: GraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.primal, "toadstool");
}

// ───── Duration serialization ────────────────────────────────────────────

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

// ───── NodeResourceRequirements serialization ─────────────────────────────

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

// ───── GraphEdge with metadata ───────────────────────────────────────────

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

// ───── EdgeType serialization variants ─────────────────────────────────────

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

// ───── ExecutionGraph metadata default ────────────────────────────────────

#[test]
fn test_execution_graph_deserialize_missing_metadata() {
    let json = r#"{"id":"g1","nodes":[],"edges":[]}"#;
    let graph: ExecutionGraph = serde_json::from_str(json).unwrap();
    assert!(graph.metadata.is_empty());
}

// ───── GraphValidationError Debug ──────────────────────────────────────────

#[test]
fn test_graph_validation_error_debug() {
    let err = GraphValidationError::EmptyGraph;
    let _ = format!("{err:?}");
}

// ───── Cycle path content ─────────────────────────────────────────────────

#[test]
fn test_cycle_detected_path_content() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([GraphNode::simple("x", "op1"), GraphNode::simple("y", "op2")])
        .connect("x", "y")
        .connect("y", "x")
        .build();

    let err = graph.validate().unwrap_err();
    if let GraphValidationError::CycleDetected(path) = &err {
        assert_eq!(path.len(), 2);
        assert!(path.contains(&"x".to_string()));
        assert!(path.contains(&"y".to_string()));
    } else {
        panic!("expected CycleDetected, got {err:?}");
    }
}

// ───── Node with no outgoing edges ────────────────────────────────────────

#[test]
fn test_dfs_node_without_neighbors() {
    let graph = ExecutionGraph::builder("g1")
        .nodes([GraphNode::simple("sink", "op")])
        .build();

    assert!(graph.validate().is_ok());
    let node = graph.get_node("sink").unwrap();
    assert!(node.id == "sink");
}

// ───── GraphValidationError Display for all variants ───────────────────────

#[test]
fn test_graph_validation_error_display_cycle_detected() {
    let err = GraphValidationError::CycleDetected(vec!["a".into(), "b".into()]);
    let s = err.to_string();
    assert!(s.contains("Cycle detected"));
    assert!(s.contains('a'));
    assert!(s.contains('b'));
}

#[test]
fn test_graph_validation_error_display_duplicate_node_id() {
    let err = GraphValidationError::DuplicateNodeId("dup-id".into());
    assert!(err.to_string().contains("dup-id"));
}

#[test]
fn test_graph_validation_error_display_self_edge() {
    let err = GraphValidationError::SelfEdge("self-node".into());
    assert!(err.to_string().contains("self-node"));
}

#[test]
fn test_graph_validation_error_display_invalid_edge() {
    let err = GraphValidationError::InvalidEdge {
        from: "src".into(),
        to: "dst".into(),
        reason: "custom reason".into(),
    };
    let s = err.to_string();
    assert!(s.contains("src"));
    assert!(s.contains("dst"));
    assert!(s.contains("custom reason"));
}
