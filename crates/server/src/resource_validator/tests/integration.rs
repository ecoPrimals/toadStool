// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use toadstool::resources::{CpuRequirements, MemoryRequirements};

use crate::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};
use crate::resource_validator::system_query;
use crate::resource_validator::{ResourceValidator, ValidationError};

use super::helpers::wgpu_safe_or_skip;

#[tokio::test(flavor = "current_thread")]
async fn test_validate_small_graph() {
    if !wgpu_safe_or_skip() {
        return;
    }
    let validator = ResourceValidator::new();

    let graph = ExecutionGraph {
        id: "small-graph".to_string(),
        nodes: vec![GraphNode {
            id: "node-1".to_string(),
            primal: "toadstool".to_string(),
            operation: "cpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                cpu: Some(CpuRequirements {
                    min_cores: 2.0,
                    ..Default::default()
                }),
                memory: Some(MemoryRequirements {
                    min_bytes: 1024 * 1024 * 1024, // 1GB
                    ..Default::default()
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };

    let result = validator.validate_availability(&graph).await.unwrap();

    // Small graph should be available on most systems
    assert!(result.available);
    assert!(result.gaps.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_large_graph() {
    if !wgpu_safe_or_skip() {
        return;
    }
    let validator = ResourceValidator::new();

    // Create a graph that requires more resources than any system has
    let graph = ExecutionGraph {
        id: "huge-graph".to_string(),
        nodes: vec![GraphNode {
            id: "node-1".to_string(),
            primal: "toadstool".to_string(),
            operation: "cpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                cpu: Some(CpuRequirements {
                    min_cores: 1000.0, // Unrealistic
                    ..Default::default()
                }),
                memory: Some(MemoryRequirements {
                    min_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
                    ..Default::default()
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        }],
        edges: vec![],
        metadata: HashMap::new(),
    };

    let result = validator.validate_availability(&graph).await.unwrap();

    // Huge graph should not be available
    assert!(!result.available);
    assert!(!result.gaps.is_empty());

    // Should have CPU and memory gaps
    assert!(result.gaps.iter().any(|g| g.resource_type == "cpu_cores"));
    assert!(result.gaps.iter().any(|g| g.resource_type == "memory"));
}

#[tokio::test(flavor = "current_thread")]
async fn query_system_capabilities_returns_nonzero_cpu() {
    let caps = system_query::query_system_capabilities().await.unwrap();
    assert!(caps.total_cpu_cores > 0);
    assert!(caps.available_cpu_cores <= caps.total_cpu_cores);
}

#[tokio::test(flavor = "current_thread")]
async fn validate_availability_fails_on_cyclic_graph() {
    let validator = ResourceValidator::new();
    let graph = ExecutionGraph {
        id: "cycle-graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
            GraphNode {
                id: "b".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            },
        ],
        edges: vec![
            GraphEdge {
                from: "a".to_string(),
                to: "b".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
            GraphEdge {
                from: "b".to_string(),
                to: "a".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            },
        ],
        metadata: HashMap::new(),
    };
    let err = validator.validate_availability(&graph).await.unwrap_err();
    assert!(matches!(err, ValidationError::EstimationFailed(_)));
}
