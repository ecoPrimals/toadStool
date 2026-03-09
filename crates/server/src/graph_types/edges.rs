// SPDX-License-Identifier: AGPL-3.0-only
//! Graph edge types for workflow dependencies

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dependency edge between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID (dependency)
    pub from: String,

    /// Target node ID (dependent)
    pub to: String,

    /// Edge type (data flow, control flow, or general dependency)
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
    fn test_graph_edge_new() {
        let edge = GraphEdge::new("a", "b");
        assert_eq!(edge.from, "a");
        assert_eq!(edge.to, "b");
        assert_eq!(edge.edge_type, EdgeType::Dependency);
        assert!(edge.metadata.is_empty());
    }

    #[test]
    fn test_graph_edge_data_flow() {
        let edge = GraphEdge::data_flow("src", "dst");
        assert_eq!(edge.from, "src");
        assert_eq!(edge.to, "dst");
        assert_eq!(edge.edge_type, EdgeType::DataFlow);
    }

    #[test]
    fn test_graph_edge_control() {
        let edge = GraphEdge::control("ctrl", "target");
        assert_eq!(edge.edge_type, EdgeType::Control);
    }

    #[test]
    fn test_edge_type_default() {
        let default: EdgeType = EdgeType::default();
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
        let edge = GraphEdge::data_flow("from", "to");
        let json = serde_json::to_string(&edge).unwrap();
        let restored: GraphEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge.from, restored.from);
        assert_eq!(edge.to, restored.to);
        assert_eq!(edge.edge_type, restored.edge_type);
    }
}
