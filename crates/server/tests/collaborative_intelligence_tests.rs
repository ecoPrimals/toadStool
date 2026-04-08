// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::default_trait_access,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::no_effect_underscore_binding,
    clippy::unreadable_literal
)]
//! Comprehensive tests for collaborative intelligence resource planning
//!
//! Tests complex graph scenarios, edge cases, and integration with ToadStool systems.
//!
//! ## Modern Idiomatic Rust
//!
//! These tests demonstrate the new builder pattern for ergonomic graph construction.

use toadstool_server::{
    graph_types::{ExecutionGraph, GraphEdge, GraphNode},
    resource_estimator::ResourceEstimator,
    resource_optimizer::ResourceOptimizer,
    resource_validator::ResourceValidator,
};
use toadstool_testing::gpu_guards;

/// Helper to create a graph node using the builder pattern
fn create_node(
    id: &str,
    operation: &str,
    cpu_cores: f64,
    memory_gb: u64,
    duration_secs: u64,
) -> GraphNode {
    GraphNode::builder(id, operation)
        .cpu(cpu_cores)
        .memory_gb(memory_gb)
        .duration_secs(duration_secs)
        .build()
}

/// Helper to create a graph node with GPU requirements
fn create_gpu_node(
    id: &str,
    operation: &str,
    cpu_cores: f64,
    memory_gb: u64,
    gpu_memory_gb: u64,
    duration_secs: u64,
) -> GraphNode {
    GraphNode::builder(id, operation)
        .cpu(cpu_cores)
        .memory_gb(memory_gb)
        .gpu_memory_gb(gpu_memory_gb)
        .duration_secs(duration_secs)
        .build()
}

/// Helper to create a simple edge using the new constructor
fn create_edge(from: &str, to: &str) -> GraphEdge {
    GraphEdge::new(from, to)
}

/// Test sequential workflow (5+ nodes in a chain)
#[tokio::test]
async fn test_sequential_workflow() {
    let graph = ExecutionGraph {
        id: "sequential_pipeline".to_string(),
        nodes: vec![
            create_node("node_a", "load_data", 2.0, 1, 5),
            create_node("node_b", "preprocess", 4.0, 2, 10),
            create_node("node_c", "transform", 4.0, 3, 15),
            create_node("node_d", "analyze", 8.0, 4, 20),
            create_node("node_e", "store_results", 2.0, 1, 5),
        ],
        edges: vec![
            create_edge("node_a", "node_b"),
            create_edge("node_b", "node_c"),
            create_edge("node_c", "node_d"),
            create_edge("node_d", "node_e"),
        ],
        metadata: Default::default(),
    };

    assert!(graph.validate().is_ok(), "Graph should be valid");

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    assert_eq!(
        estimate.max_parallelism, 1,
        "Sequential graph should have parallelism of 1"
    );
    assert_eq!(
        estimate.critical_path_length, 5,
        "Critical path should include all 5 nodes"
    );
    assert_eq!(estimate.cpu_cores, 8, "Peak CPU should be node_d's 8 cores");

    if gpu_guards::is_wgpu_safe() {
        let validator = ResourceValidator::new();
        let availability = validator
            .validate_availability(&graph)
            .await
            .expect("Validation should succeed");

        println!(
            "Sequential workflow - Available: {}, Gaps: {}",
            availability.available,
            availability.gaps.len()
        );
    } else {
        eprintln!("{}", gpu_guards::wgpu_skip_reason());
    }

    let optimizer = ResourceOptimizer::new();
    let suggestions = optimizer
        .suggest_optimizations(&graph)
        .await
        .expect("Optimization should succeed");

    println!(
        "Sequential workflow suggestions: {}",
        suggestions.opportunities.len()
    );
}

/// Test parallel workflow (10+ nodes executing concurrently)
#[tokio::test]
async fn test_parallel_workflow() {
    let mut nodes = vec![create_node("root", "prepare_data", 2.0, 1, 5)];

    for i in 0..10 {
        nodes.push(create_node(
            &format!("worker_{i}"),
            "parallel_process",
            4.0,
            2,
            30,
        ));
    }

    nodes.push(create_node("sink", "aggregate_results", 4.0, 5, 10));

    let mut edges = vec![];
    for i in 0..10 {
        edges.push(create_edge("root", &format!("worker_{i}")));
        edges.push(create_edge(&format!("worker_{i}"), "sink"));
    }

    let graph = ExecutionGraph {
        id: "parallel_pipeline".to_string(),
        nodes,
        edges,
        metadata: Default::default(),
    };

    assert!(graph.validate().is_ok(), "Graph should be valid");

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    assert_eq!(
        estimate.max_parallelism, 10,
        "Should have 10 parallel workers"
    );
    assert_eq!(
        estimate.critical_path_length, 3,
        "Critical path: root → worker → sink"
    );
    assert_eq!(
        estimate.cpu_cores, 40,
        "Peak CPU should be 10 workers * 4 cores"
    );

    let optimizer = ResourceOptimizer::new();
    let suggestions = optimizer
        .suggest_optimizations(&graph)
        .await
        .expect("Optimization should succeed");

    println!(
        "Parallel workflow suggestions: {}",
        suggestions.opportunities.len()
    );
}

/// Test diamond topology
#[tokio::test]
async fn test_diamond_topology() {
    let graph = ExecutionGraph {
        id: "diamond_pattern".to_string(),
        nodes: vec![
            create_node("a", "start", 2.0, 1, 5),
            create_node("b", "branch_1", 4.0, 2, 20),
            create_node("c", "branch_2", 4.0, 2, 15),
            create_node("d", "merge", 4.0, 3, 10),
        ],
        edges: vec![
            create_edge("a", "b"),
            create_edge("a", "c"),
            create_edge("b", "d"),
            create_edge("c", "d"),
        ],
        metadata: Default::default(),
    };

    assert!(graph.validate().is_ok(), "Graph should be valid");

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    assert_eq!(
        estimate.max_parallelism, 2,
        "Should have 2 parallel branches"
    );
    assert_eq!(estimate.critical_path_length, 3, "Critical path: a → b → d");
    assert_eq!(
        estimate.cpu_cores, 8,
        "Peak CPU should be 2 branches * 4 cores"
    );

    println!(
        "Diamond topology - Parallelism: {}, Critical path: {}",
        estimate.max_parallelism, estimate.critical_path_length
    );
}

/// Test large graph (100+ nodes)
#[tokio::test]
async fn test_large_graph_performance() {
    let mut nodes = vec![create_node("root", "start", 2.0, 1, 1)];
    let mut edges = vec![];

    // Level 1: 10 nodes
    for i in 0..10 {
        let node_id = format!("level1_{i}");
        nodes.push(create_node(&node_id, "process", 2.0, 1, 2));
        edges.push(create_edge("root", &node_id));
    }

    // Level 2: 90 nodes (9 per level1 node)
    for i in 0..10 {
        let parent_id = format!("level1_{i}");
        for j in 0..9 {
            let node_id = format!("level2_{i}_{j}");
            nodes.push(create_node(&node_id, "compute", 1.0, 1, 5));
            edges.push(create_edge(&parent_id, &node_id));
        }
    }

    let graph = ExecutionGraph {
        id: "large_graph".to_string(),
        nodes,
        edges,
        metadata: Default::default(),
    };

    assert_eq!(graph.nodes.len(), 101, "Should have 101 nodes");
    assert!(graph.validate().is_ok(), "Large graph should be valid");

    // Test performance
    use std::time::Instant;
    let start = Instant::now();

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    let duration = start.elapsed();
    println!("Large graph estimation took: {duration:?}");

    // Performance target: < 100ms for 100+ node graph
    assert!(
        duration.as_millis() < 100,
        "Estimation should complete in <100ms, took {duration:?}"
    );

    assert_eq!(
        estimate.max_parallelism, 90,
        "Should have 90 parallel nodes at level 2"
    );
    assert_eq!(
        estimate.critical_path_length, 3,
        "Critical path: root → level1 → level2"
    );
}

/// Test GPU-accelerated workflow
#[tokio::test]
async fn test_gpu_workflow() {
    let graph = ExecutionGraph {
        id: "gpu_pipeline".to_string(),
        nodes: vec![
            create_node("load", "load_model", 4.0, 8, 10),
            create_gpu_node("inference", "gpu_compute", 2.0, 4, 16, 60),
            create_node("postprocess", "cpu_compute", 8.0, 16, 30),
        ],
        edges: vec![
            create_edge("load", "inference"),
            create_edge("inference", "postprocess"),
        ],
        metadata: Default::default(),
    };

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    assert_eq!(
        estimate.gpu_memory_bytes,
        16 * 1024 * 1024 * 1024,
        "Should require 16GB GPU memory"
    );

    if gpu_guards::is_wgpu_safe() {
        let validator = ResourceValidator::new();
        let availability = validator
            .validate_availability(&graph)
            .await
            .expect("Validation should succeed");

        if !availability.available {
            println!(
                "GPU gaps found: {}",
                availability
                    .gaps
                    .iter()
                    .filter(|gap| gap.resource_type.contains("gpu"))
                    .count()
            );
        }
    } else {
        eprintln!("{}", gpu_guards::wgpu_skip_reason());
    }

    let optimizer = ResourceOptimizer::new();
    let suggestions = optimizer
        .suggest_optimizations(&graph)
        .await
        .expect("Optimization should succeed");

    println!(
        "GPU workflow suggestions: {}",
        suggestions.opportunities.len()
    );
}

/// Test cycle detection
#[tokio::test]
async fn test_cycle_detection() {
    let cyclic_graph = ExecutionGraph {
        id: "cyclic".to_string(),
        nodes: vec![
            create_node("a", "op", 2.0, 1, 5),
            create_node("b", "op", 2.0, 1, 5),
        ],
        edges: vec![
            create_edge("a", "b"),
            create_edge("b", "a"), // Cycle!
        ],
        metadata: Default::default(),
    };

    assert!(cyclic_graph.validate().is_err(), "Should detect cycle");

    let estimator = ResourceEstimator::new();
    let result = estimator.estimate(&cyclic_graph);
    assert!(result.is_err(), "Estimation should fail for cyclic graph");
}

/// Test fork-join pattern
#[tokio::test]
async fn test_fork_join_pattern() {
    // Create fork-join: A → (B, C, D) → E
    let graph = ExecutionGraph {
        id: "fork_join".to_string(),
        nodes: vec![
            create_node("a", "prepare", 2.0, 1, 5),
            create_node("b", "task1", 4.0, 2, 10),
            create_node("c", "task2", 4.0, 2, 15),
            create_node("d", "task3", 4.0, 2, 12),
            create_node("e", "merge", 4.0, 4, 8),
        ],
        edges: vec![
            create_edge("a", "b"),
            create_edge("a", "c"),
            create_edge("a", "d"),
            create_edge("b", "e"),
            create_edge("c", "e"),
            create_edge("d", "e"),
        ],
        metadata: Default::default(),
    };

    assert!(graph.validate().is_ok(), "Fork-join graph should be valid");

    let estimator = ResourceEstimator::new();
    let estimate = estimator
        .estimate(&graph)
        .expect("Estimation should succeed");

    assert_eq!(estimate.max_parallelism, 3, "Should have 3 parallel tasks");
    assert_eq!(
        estimate.critical_path_length, 3,
        "Critical path: a → c (longest) → e"
    );

    println!(
        "Fork-join - Parallelism: {}, Duration: {}s",
        estimate.max_parallelism,
        estimate.estimated_duration.as_secs()
    );
}
