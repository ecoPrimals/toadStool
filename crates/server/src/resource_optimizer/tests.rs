// SPDX-License-Identifier: AGPL-3.0-only
#![allow(deprecated)]
use std::collections::HashMap;
use std::time::Duration;

use toadstool_common::interned_strings::primals;

use super::*;
use crate::graph_types::{EdgeType, GraphEdge, GraphNode, NodeResourceRequirements};
use crate::resource_estimator::{NodeEstimate, ResourceEstimate};
use crate::resource_validator::SystemCapabilities;

#[tokio::test(flavor = "current_thread")]
async fn test_suggest_optimizations_sequential_graph() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph {
        id: "sequential-graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "node-1".to_string(),
                primal: primals::TOADSTOOL.to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-2".to_string(),
                primal: primals::TOADSTOOL.to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "node-3".to_string(),
                primal: primals::TOADSTOOL.to_string(),
                operation: "cpu_compute".to_string(),
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
                to: "node-3".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
        ],
        metadata: HashMap::new(),
    };
    let suggestions = optimizer.suggest_optimizations(&graph).await.unwrap();
    assert!(!suggestions.bottlenecks.is_empty());
    assert!(suggestions
        .bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::SequentialExecution));
    assert!(!suggestions.opportunities.is_empty());
}

#[test]
fn test_default_optimizer() {
    let default_opt = ResourceOptimizer::default();
    let new_opt = ResourceOptimizer::new();
    assert_eq!(
        std::mem::size_of_val(&default_opt),
        std::mem::size_of_val(&new_opt)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_optimization_error_estimation_failed_empty_graph() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph::simple("empty");
    let err = optimizer.suggest_optimizations(&graph).await.unwrap_err();
    match &err {
        OptimizationError::EstimationFailed(e) => {
            assert!(e.to_string().contains("empty") || e.to_string().contains("Invalid"));
        }
        _ => panic!("expected EstimationFailed, got {err:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_long_critical_path_bottleneck() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph::builder("long-path")
        .nodes([
            GraphNode::simple("n1", "cpu_compute"),
            GraphNode::simple("n2", "cpu_compute"),
            GraphNode::simple("n3", "cpu_compute"),
            GraphNode::simple("n4", "cpu_compute"),
            GraphNode::simple("n5", "cpu_compute"),
            GraphNode::simple("n6", "cpu_compute"),
            GraphNode::simple("n7", "cpu_compute"),
        ])
        .connect("n1", "n2")
        .connect("n2", "n3")
        .connect("n3", "n4")
        .connect("n4", "n5")
        .connect("n5", "n6")
        .connect("n6", "n7")
        .build();
    let suggestions = optimizer.suggest_optimizations(&graph).await.unwrap();
    assert!(suggestions
        .bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::LongCriticalPath));
}

#[tokio::test(flavor = "current_thread")]
async fn test_caching_opportunity() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph::builder("caching")
        .nodes([
            GraphNode::simple("fan", "cpu_compute"),
            GraphNode::simple("b", "cpu_compute"),
            GraphNode::simple("c", "cpu_compute"),
        ])
        .connect("fan", "b")
        .connect("fan", "c")
        .build();
    let suggestions = optimizer.suggest_optimizations(&graph).await.unwrap();
    assert!(suggestions
        .opportunities
        .iter()
        .any(|o| o.opportunity_type == OpportunityType::Caching));
}

#[tokio::test(flavor = "current_thread")]
async fn test_batching_opportunity() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph::builder("batch")
        .nodes([
            GraphNode::simple("n1", "storage"),
            GraphNode::simple("n2", "storage"),
            GraphNode::simple("n3", "storage"),
            GraphNode::simple("n4", "storage"),
        ])
        .connect("n1", "n2")
        .connect("n2", "n3")
        .connect("n3", "n4")
        .build();
    let suggestions = optimizer.suggest_optimizations(&graph).await.unwrap();
    assert!(suggestions
        .opportunities
        .iter()
        .any(|o| o.opportunity_type == OpportunityType::Batching));
}

#[test]
fn test_optimization_error_display() {
    let err = OptimizationError::SystemQueryFailed("test failure".into());
    assert!(err.to_string().contains("test failure"));
}

#[test]
fn test_bottleneck_type_serialization() {
    let t = BottleneckType::ResourceContention;
    let json = serde_json::to_string(&t).unwrap();
    let restored: BottleneckType = serde_json::from_str(&json).unwrap();
    assert_eq!(t, restored);
}

#[test]
fn test_optimization_suggestions_roundtrip() {
    let suggestions = OptimizationSuggestions {
        graph_id: "g1".into(),
        bottlenecks: vec![Bottleneck {
            bottleneck_type: BottleneckType::InefficientAllocation,
            affected_nodes: vec!["n1".into()],
            severity: 0.5,
            description: "test".into(),
            time_impact_secs: 10,
        }],
        opportunities: vec![Opportunity {
            opportunity_type: OpportunityType::NodeSplitting,
            affected_nodes: vec!["n2".into()],
            benefit: 0.6,
            description: "split".into(),
            recommendation: "do it".into(),
            time_savings_secs: 5,
            resource_savings: HashMap::new(),
        }],
        estimated_improvement: ImprovementEstimate {
            current_duration_secs: 100,
            optimized_duration_secs: 80,
            time_savings_secs: 20,
            speedup_factor: 1.25,
            current_resources: HashMap::new(),
            optimized_resources: HashMap::new(),
        },
        priority_order: vec!["first".into()],
    };
    let json = serde_json::to_string(&suggestions).unwrap();
    let _restored: OptimizationSuggestions = serde_json::from_str(&json).unwrap();
}

// ── Pure function unit tests (allocation, cost) ─────────────────────────────

fn mock_capabilities(gpu_count: usize) -> SystemCapabilities {
    SystemCapabilities {
        total_cpu_cores: 32,
        available_cpu_cores: 24,
        total_memory_bytes: 64 * 1024 * 1024 * 1024,
        available_memory_bytes: 48 * 1024 * 1024 * 1024,
        total_gpu_memory_bytes: 8 * 1024 * 1024 * 1024,
        available_gpu_memory_bytes: 6 * 1024 * 1024 * 1024,
        total_storage_bytes: 256 * 1024 * 1024 * 1024,
        available_storage_bytes: 128 * 1024 * 1024 * 1024,
        network_bandwidth_mbps: 1000,
        gpu_count,
        gpu_types: if gpu_count > 0 {
            vec!["NVIDIA RTX".to_string()]
        } else {
            vec![]
        },
    }
}

fn mock_estimate(
    max_parallelism: usize,
    critical_path_length: usize,
    memory_bytes: u64,
    gpu_memory_bytes: u64,
    node_estimates: HashMap<String, NodeEstimate>,
    estimated_duration_secs: u64,
) -> ResourceEstimate {
    ResourceEstimate {
        graph_id: "mock".to_string(),
        cpu_cores: 8,
        memory_bytes,
        gpu_memory_bytes,
        storage_bytes: 1024 * 1024 * 1024,
        network_bandwidth_mbps: 100,
        estimated_duration: Duration::from_secs(estimated_duration_secs),
        max_parallelism,
        critical_path_length,
        node_estimates,
        warnings: vec![],
    }
}

#[test]
fn test_identify_bottlenecks_sequential() {
    let graph = ExecutionGraph::builder("seq")
        .nodes([
            GraphNode::simple("n1", "cpu_compute"),
            GraphNode::simple("n2", "cpu_compute"),
        ])
        .connect("n1", "n2")
        .build();
    let estimate = mock_estimate(1, 2, 8 * 1024 * 1024 * 1024, 0, HashMap::new(), 60);
    let caps = mock_capabilities(0);

    let bottlenecks = identify_bottlenecks(&graph, &estimate, &caps);
    assert!(bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::SequentialExecution));
}

#[test]
fn test_identify_bottlenecks_long_critical_path() {
    let graph = ExecutionGraph::builder("long")
        .nodes([
            GraphNode::simple("n1", "cpu_compute"),
            GraphNode::simple("n2", "cpu_compute"),
            GraphNode::simple("n3", "cpu_compute"),
            GraphNode::simple("n4", "cpu_compute"),
            GraphNode::simple("n5", "cpu_compute"),
            GraphNode::simple("n6", "cpu_compute"),
        ])
        .connect("n1", "n2")
        .connect("n2", "n3")
        .connect("n3", "n4")
        .connect("n4", "n5")
        .connect("n5", "n6")
        .build();
    let mut node_est = HashMap::new();
    node_est.insert(
        "n1".into(),
        NodeEstimate {
            node_id: "n1".into(),
            cpu_cores: 2,
            memory_bytes: 1024 * 1024 * 1024,
            gpu_memory_bytes: 0,
            duration: Duration::from_secs(10),
            parallelism_level: 0,
        },
    );
    let estimate = mock_estimate(1, 6, 8 * 1024 * 1024 * 1024, 0, node_est, 60);
    let caps = mock_capabilities(0);

    let bottlenecks = identify_bottlenecks(&graph, &estimate, &caps);
    assert!(bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::LongCriticalPath));
}

#[test]
fn test_identify_bottlenecks_memory() {
    let graph = ExecutionGraph::builder("mem")
        .nodes([GraphNode::simple("big", "cpu_compute")])
        .build();
    let mut node_est = HashMap::new();
    node_est.insert(
        "big".into(),
        NodeEstimate {
            node_id: "big".into(),
            cpu_cores: 4,
            memory_bytes: 80 * 1024 * 1024 * 1024, // 80GB
            gpu_memory_bytes: 0,
            duration: Duration::from_secs(30),
            parallelism_level: 0,
        },
    );
    let estimate = mock_estimate(1, 1, 80 * 1024 * 1024 * 1024, 0, node_est, 30);
    let caps = mock_capabilities(0);

    let bottlenecks = identify_bottlenecks(&graph, &estimate, &caps);
    assert!(bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::MemoryBottleneck));
}

#[test]
fn test_identify_bottlenecks_gpu_underutilization() {
    let graph = ExecutionGraph::builder("gpu")
        .nodes([
            GraphNode::simple("n1", "cpu_compute"),
            GraphNode::simple("n2", "cpu_compute"),
        ])
        .build();
    let estimate = mock_estimate(2, 1, 4 * 1024 * 1024 * 1024, 0, HashMap::new(), 90);
    let caps = mock_capabilities(1); // GPU available but not used

    let bottlenecks = identify_bottlenecks(&graph, &estimate, &caps);
    assert!(bottlenecks
        .iter()
        .any(|b| b.bottleneck_type == BottleneckType::GpuUnderutilization));
}

#[test]
fn test_discover_opportunities_caching() {
    let graph = ExecutionGraph::builder("cache")
        .nodes([
            GraphNode::simple("fan", "cpu_compute"),
            GraphNode::simple("a", "cpu_compute"),
            GraphNode::simple("b", "cpu_compute"),
        ])
        .connect("fan", "a")
        .connect("fan", "b")
        .build();
    let mut node_est = HashMap::new();
    for (id, lvl) in [("fan", 0), ("a", 1), ("b", 1)] {
        node_est.insert(
            id.into(),
            NodeEstimate {
                node_id: id.into(),
                cpu_cores: 2,
                memory_bytes: 1024 * 1024 * 1024,
                gpu_memory_bytes: 0,
                duration: Duration::from_secs(10),
                parallelism_level: lvl,
            },
        );
    }
    let estimate = mock_estimate(2, 2, 4 * 1024 * 1024 * 1024, 0, node_est, 20);
    let caps = mock_capabilities(0);

    let opportunities = discover_opportunities(&graph, &estimate, &caps);
    assert!(opportunities
        .iter()
        .any(|o| o.opportunity_type == OpportunityType::Caching));
}

#[test]
fn test_discover_opportunities_batching() {
    let graph = ExecutionGraph::builder("batch")
        .nodes([
            GraphNode::simple("n1", "storage"),
            GraphNode::simple("n2", "storage"),
            GraphNode::simple("n3", "storage"),
            GraphNode::simple("n4", "storage"),
        ])
        .connect("n1", "n2")
        .connect("n2", "n3")
        .connect("n3", "n4")
        .build();
    let mut node_est = HashMap::new();
    for (id, lvl) in [("n1", 0), ("n2", 1), ("n3", 2), ("n4", 3)] {
        node_est.insert(
            id.into(),
            NodeEstimate {
                node_id: id.into(),
                cpu_cores: 1,
                memory_bytes: 512 * 1024 * 1024,
                gpu_memory_bytes: 0,
                duration: Duration::from_secs(5),
                parallelism_level: lvl,
            },
        );
    }
    let estimate = mock_estimate(1, 4, 2 * 1024 * 1024 * 1024, 0, node_est, 20);
    let caps = mock_capabilities(0);

    let opportunities = discover_opportunities(&graph, &estimate, &caps);
    assert!(opportunities
        .iter()
        .any(|o| o.opportunity_type == OpportunityType::Batching));
}

#[test]
fn test_estimate_improvement() {
    let estimate = ResourceEstimate {
        graph_id: "g".into(),
        cpu_cores: 8,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_memory_bytes: 0,
        storage_bytes: 1024 * 1024 * 1024,
        network_bandwidth_mbps: 100,
        estimated_duration: Duration::from_secs(100),
        max_parallelism: 2,
        critical_path_length: 3,
        node_estimates: HashMap::new(),
        warnings: vec![],
    };
    let opportunities = vec![Opportunity {
        opportunity_type: OpportunityType::Parallelization,
        affected_nodes: vec!["n1".into()],
        benefit: 0.7,
        description: "test".into(),
        recommendation: "do it".into(),
        time_savings_secs: 30,
        resource_savings: HashMap::new(),
    }];

    let improvement = estimate_improvement(&estimate, &opportunities);
    assert!(improvement.time_savings_secs > 0);
    assert!(improvement.optimized_duration_secs < improvement.current_duration_secs);
}

#[test]
fn test_rank_by_priority() {
    let opportunities = vec![
        Opportunity {
            opportunity_type: OpportunityType::Caching,
            affected_nodes: vec!["a".into()],
            benefit: 0.4,
            description: String::new(),
            recommendation: String::new(),
            time_savings_secs: 20,
            resource_savings: HashMap::new(),
        },
        Opportunity {
            opportunity_type: OpportunityType::GpuAcceleration,
            affected_nodes: vec!["b".into()],
            benefit: 0.8,
            description: String::new(),
            recommendation: String::new(),
            time_savings_secs: 60,
            resource_savings: HashMap::new(),
        },
    ];

    let ranked = rank_by_priority(&opportunities);
    assert_eq!(ranked.len(), 2);
    // Higher benefit * time_savings should rank first
    assert!(ranked[0].contains("GpuAcceleration") || ranked[0].contains('b'));
}
