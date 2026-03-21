// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Validation tests for ExecutionGraph

use std::collections::HashMap;
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, GraphValidationError, NodeResourceRequirements,
};

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

#[test]
fn test_graph_validation_error_display() {
    assert_eq!(
        GraphValidationError::EmptyGraph.to_string(),
        "Graph is empty (no nodes)"
    );
    assert!(
        GraphValidationError::DuplicateNodeId("x".into())
            .to_string()
            .contains("Duplicate node ID")
    );
    assert!(
        GraphValidationError::SelfEdge("n".into())
            .to_string()
            .contains("Self-edge")
    );
    assert!(
        GraphValidationError::InvalidEdge {
            from: "a".into(),
            to: "b".into(),
            reason: "test".into(),
        }
        .to_string()
        .contains("Invalid edge")
    );
    assert!(
        GraphValidationError::CycleDetected(vec!["a".into(), "b".into()])
            .to_string()
            .contains("Cycle detected")
    );
}

#[test]
fn test_graph_validation_error_debug() {
    let err = GraphValidationError::EmptyGraph;
    let _ = format!("{err:?}");
}

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
