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
