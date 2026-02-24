//! Graph edge types for workflow dependencies
//!
//! This module contains types for representing edges (dependencies) between
//! nodes in a workflow graph.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dependency edge between nodes
///
/// Edges represent execution dependencies. The graph executor must ensure
/// that all dependencies of a node are completed before starting that node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID (dependency)
    pub from: String,

    /// Target node ID (dependent)
    pub to: String,

    /// Edge type (data flow, control flow, or general dependency)
    /// Defaults to Dependency if not specified
    #[serde(default)]
    pub edge_type: EdgeType,

    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl GraphEdge {
    /// Create a simple dependency edge
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::Dependency,
            metadata: HashMap::new(),
        }
    }

    /// Create a data flow edge
    pub fn data_flow(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::DataFlow,
            metadata: HashMap::new(),
        }
    }

    /// Create a control flow edge
    pub fn control(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::Control,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to an edge
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Type of dependency edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum EdgeType {
    /// Data flows from source to target (output → input)
    DataFlow,

    /// Control flow - target waits for source to complete
    Control,

    /// General dependency - no specific semantics
    #[default]
    Dependency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let edge = GraphEdge::new("node_a", "node_b");
        assert_eq!(edge.from, "node_a");
        assert_eq!(edge.to, "node_b");
        assert_eq!(edge.edge_type, EdgeType::Dependency);
    }

    #[test]
    fn test_data_flow_edge() {
        let edge = GraphEdge::data_flow("producer", "consumer");
        assert_eq!(edge.edge_type, EdgeType::DataFlow);
    }

    #[test]
    fn test_control_edge() {
        let edge = GraphEdge::control("init", "process");
        assert_eq!(edge.edge_type, EdgeType::Control);
    }

    #[test]
    fn test_edge_with_metadata() {
        let edge = GraphEdge::new("a", "b").with_metadata("data_size", "1GB");
        assert_eq!(edge.metadata.get("data_size"), Some(&"1GB".to_string()));
    }

    #[test]
    fn test_edge_type_default() {
        let default: EdgeType = Default::default();
        assert_eq!(default, EdgeType::Dependency);
    }

    #[test]
    fn test_edge_type_serialization_roundtrip() {
        for etype in [EdgeType::DataFlow, EdgeType::Control, EdgeType::Dependency] {
            let json = serde_json::to_string(&etype).unwrap();
            let restored: EdgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(etype, restored);
        }
    }

    #[test]
    fn test_graph_edge_serialization_roundtrip() {
        let edge = GraphEdge::data_flow("producer", "consumer").with_metadata("key", "value");
        let json = serde_json::to_string(&edge).unwrap();
        let restored: GraphEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge.from, restored.from);
        assert_eq!(edge.to, restored.to);
        assert_eq!(edge.edge_type, restored.edge_type);
        assert_eq!(edge.metadata.get("key"), restored.metadata.get("key"));
    }
}
