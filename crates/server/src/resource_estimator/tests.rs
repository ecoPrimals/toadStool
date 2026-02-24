//! Tests for resource estimation

use super::estimator::ResourceEstimator;
use super::types::EstimationError;
use crate::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};
use std::collections::HashMap;
use std::time::Duration;
use toadstool::resources::{CpuRequirements, GpuRequirements, MemoryRequirements};

fn simple_node(id: &str, cpu: f64) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        primal: "toadstool".to_string(),
        operation: "cpu_compute".to_string(),
        duration: None,
        requirements: NodeResourceRequirements {
            cpu: Some(CpuRequirements {
                min_cores: cpu,
                ..Default::default()
            }),
            ..Default::default()
        },
        metadata: HashMap::new(),
    }
}

fn edge(from: &str, to: &str) -> GraphEdge {
    GraphEdge {
        from: from.to_string(),
        to: to.to_string(),
        edge_type: EdgeType::DataFlow,
        metadata: HashMap::new(),
    }
}

#[test]
fn test_empty_graph_returns_error() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "empty".to_string(),
        nodes: vec![],
        edges: vec![],
        metadata: HashMap::new(),
    };
    assert!(estimator.estimate(&graph).is_err());
}

#[test]
fn test_cyclic_graph_returns_cyclic_error() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "cycle".to_string(),
        nodes: vec![simple_node("a", 1.0), simple_node("b", 1.0)],
        edges: vec![edge("a", "b"), edge("b", "a")],
        metadata: HashMap::new(),
    };
    let err = estimator.estimate(&graph).unwrap_err();
    let is_cycle = matches!(
        err,
        EstimationError::CyclicGraph | EstimationError::InvalidGraph(_)
    );
    assert!(is_cycle, "Expected cycle error, got: {err}");
}

#[test]
fn test_self_loop_is_rejected() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "self-loop".to_string(),
        nodes: vec![simple_node("a", 1.0)],
        edges: vec![edge("a", "a")],
        metadata: HashMap::new(),
    };
    assert!(
        estimator.estimate(&graph).is_err(),
        "self-loop must be rejected"
    );
}

#[test]
fn test_invalid_edge_missing_node() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "bad-edge".to_string(),
        nodes: vec![simple_node("a", 1.0)],
        edges: vec![edge("a", "does-not-exist")],
        metadata: HashMap::new(),
    };
    assert!(
        estimator.estimate(&graph).is_err(),
        "edge to missing node must be rejected"
    );
}

#[test]
fn test_single_node_graph() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "single".to_string(),
        nodes: vec![simple_node("only", 4.0)],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert_eq!(est.max_parallelism, 1);
    assert_eq!(est.critical_path_length, 1);
    assert_eq!(est.cpu_cores, 4);
}

#[test]
fn test_sequential_cpu_peaks() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "seq".to_string(),
        nodes: vec![simple_node("a", 2.0), simple_node("b", 6.0)],
        edges: vec![edge("a", "b")],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert_eq!(est.cpu_cores, 6, "sequential peak CPU");
    assert_eq!(est.max_parallelism, 1);
}

#[test]
fn test_parallel_cpu_sums() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "par".to_string(),
        nodes: vec![
            simple_node("a", 3.0),
            simple_node("b", 5.0),
            simple_node("c", 1.0),
        ],
        edges: vec![edge("a", "c"), edge("b", "c")],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert_eq!(est.cpu_cores, 8, "parallel peak CPU = 3+5");
    assert_eq!(est.max_parallelism, 2);
}

#[test]
fn test_memory_aggregated_for_parallel_nodes() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "mem".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                primal: "toadstool".to_string(),
                operation: "gpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    memory: Some(MemoryRequirements {
                        min_bytes: 2 * 1024 * 1024 * 1024,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "b".to_string(),
                primal: "toadstool".to_string(),
                operation: "gpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    memory: Some(MemoryRequirements {
                        min_bytes: 4 * 1024 * 1024 * 1024,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            },
        ],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    let expected_bytes = 6 * 1024 * 1024 * 1024u64;
    assert_eq!(est.memory_bytes, expected_bytes, "parallel memory sum");
}

#[test]
fn test_gpu_memory_aggregated() {
    let estimator = ResourceEstimator::new();
    let gpu_node = |id: &str, vram_mb: u64| GraphNode {
        id: id.to_string(),
        primal: "toadstool".to_string(),
        operation: "gpu_compute".to_string(),
        duration: None,
        requirements: NodeResourceRequirements {
            gpu: Some(GpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: None,
                min_memory_bytes: Some(vram_mb * 1024 * 1024),
            }),
            ..Default::default()
        },
        metadata: HashMap::new(),
    };
    let graph = ExecutionGraph {
        id: "gpu".to_string(),
        nodes: vec![gpu_node("g1", 4096), gpu_node("g2", 8192)],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    let expected = (4096 + 8192) * 1024 * 1024;
    assert_eq!(est.gpu_memory_bytes, expected);
}

#[test]
fn test_high_cpu_generates_warning() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "high-cpu".to_string(),
        nodes: vec![simple_node("a", 33.0), simple_node("b", 33.0)],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert!(
        est.warnings.iter().any(|w| w.contains("CPU")),
        "Expected high CPU warning, got: {:?}",
        est.warnings
    );
}

#[test]
fn test_high_memory_generates_warning() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "high-mem".to_string(),
        nodes: vec![GraphNode {
            id: "big".to_string(),
            primal: "toadstool".to_string(),
            operation: "cpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                memory: Some(MemoryRequirements {
                    min_bytes: 130 * 1024 * 1024 * 1024,
                    ..Default::default()
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert!(
        est.warnings.iter().any(|w| w.contains("memory")),
        "Expected high memory warning, got: {:?}",
        est.warnings
    );
}

#[test]
fn test_high_gpu_memory_generates_warning() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "high-gpu".to_string(),
        nodes: vec![GraphNode {
            id: "gpu".to_string(),
            primal: "toadstool".to_string(),
            operation: "gpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                gpu: Some(GpuRequirements {
                    min_units: 1,
                    max_units: None,
                    gpu_type: None,
                    min_memory_bytes: Some(50 * 1024 * 1024 * 1024),
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert!(
        est.warnings
            .iter()
            .any(|w| w.to_lowercase().contains("gpu")),
        "Expected high GPU memory warning, got: {:?}",
        est.warnings
    );
}

#[test]
fn test_default_and_new_are_equivalent() {
    let a = ResourceEstimator::new();
    let b = ResourceEstimator::default();
    let graph = ExecutionGraph {
        id: "g".to_string(),
        nodes: vec![simple_node("n", 1.0)],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let ea = a.estimate(&graph).unwrap();
    let eb = b.estimate(&graph).unwrap();
    assert_eq!(ea.cpu_cores, eb.cpu_cores);
    assert_eq!(ea.max_parallelism, eb.max_parallelism);
}

#[test]
fn test_duration_from_metadata_hint() {
    let estimator = ResourceEstimator::new();
    let mut meta = HashMap::new();
    meta.insert("estimated_duration_secs".to_string(), "180".to_string());
    let graph = ExecutionGraph {
        id: "dur".to_string(),
        nodes: vec![GraphNode {
            id: "slow".to_string(),
            primal: "toadstool".to_string(),
            operation: "custom_operation".to_string(),
            duration: None,
            requirements: NodeResourceRequirements::default(),
            metadata: meta,
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let est = estimator.estimate(&graph).unwrap();
    assert!(
        est.estimated_duration >= Duration::from_secs(180),
        "duration must reflect metadata hint: got {:?}",
        est.estimated_duration
    );
}

#[test]
fn test_neural_compute_duration_longer_than_cpu() {
    let estimator = ResourceEstimator::new();
    let make_graph = |op: &str| ExecutionGraph {
        id: op.to_string(),
        nodes: vec![GraphNode {
            id: "n".to_string(),
            primal: "toadstool".to_string(),
            operation: op.to_string(),
            duration: None,
            requirements: NodeResourceRequirements::default(),
            metadata: HashMap::new(),
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };
    let cpu_est = estimator.estimate(&make_graph("cpu_compute")).unwrap();
    let neural_est = estimator.estimate(&make_graph("neural_compute")).unwrap();
    assert!(
        neural_est.estimated_duration >= cpu_est.estimated_duration,
        "neural_compute should have >= duration than cpu_compute"
    );
}

#[test]
fn test_simple_linear_graph() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "linear-graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 4.0,
                        ..Default::default()
                    }),
                    memory: Some(MemoryRequirements {
                        min_bytes: 2 * 1024 * 1024 * 1024,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-2".to_string(),
                primal: "toadstool".to_string(),
                operation: "gpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 2.0,
                        ..Default::default()
                    }),
                    memory: Some(MemoryRequirements {
                        min_bytes: 4 * 1024 * 1024 * 1024,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
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
    let estimate = estimator.estimate(&graph).unwrap();
    assert_eq!(estimate.max_parallelism, 1);
    assert_eq!(estimate.critical_path_length, 2);
    assert_eq!(estimate.cpu_cores, 4);
    assert_eq!(estimate.memory_bytes, 4 * 1024 * 1024 * 1024);
}

#[test]
fn test_parallel_graph() {
    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "parallel-graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 2.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-2".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 2.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
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
    let estimate = estimator.estimate(&graph).unwrap();
    assert_eq!(estimate.max_parallelism, 2);
    assert_eq!(estimate.critical_path_length, 2);
    assert_eq!(estimate.cpu_cores, 4);
}
