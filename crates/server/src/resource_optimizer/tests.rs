#![allow(deprecated)]
use std::collections::HashMap;

use toadstool_common::interned_strings::primals;

use super::*;
use crate::graph_types::{EdgeType, GraphEdge, GraphNode, NodeResourceRequirements};

#[tokio::test]
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

#[tokio::test]
async fn test_optimization_error_estimation_failed_empty_graph() {
    let optimizer = ResourceOptimizer::new();
    let graph = ExecutionGraph::simple("empty");
    let err = optimizer.suggest_optimizations(&graph).await.unwrap_err();
    match &err {
        OptimizationError::EstimationFailed(e) => {
            assert!(e.to_string().contains("empty") || e.to_string().contains("Invalid"));
        }
        _ => panic!("expected EstimationFailed, got {:?}", err),
    }
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
