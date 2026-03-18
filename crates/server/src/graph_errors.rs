// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph validation errors
//!
//! This module contains error types for graph validation failures.

/// Graph validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum GraphValidationError {
    #[error("Graph is empty (no nodes)")]
    EmptyGraph,

    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(String),

    #[error("Invalid edge from '{from}' to '{to}': {reason}")]
    InvalidEdge {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Self-edge detected on node '{0}'")]
    SelfEdge(String),

    #[error("Cycle detected in graph: {}", .0.join(" -> "))]
    CycleDetected(Vec<String>),
}

impl GraphValidationError {
    /// Create an InvalidEdge error for a missing source node
    pub fn missing_source(from: &str, to: &str) -> Self {
        Self::InvalidEdge {
            from: from.to_string(),
            to: to.to_string(),
            reason: "source node does not exist".to_string(),
        }
    }

    /// Create an InvalidEdge error for a missing target node
    pub fn missing_target(from: &str, to: &str) -> Self {
        Self::InvalidEdge {
            from: from.to_string(),
            to: to.to_string(),
            reason: "target node does not exist".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = GraphValidationError::EmptyGraph;
        assert!(err.to_string().contains("empty"));

        let err = GraphValidationError::DuplicateNodeId("node1".to_string());
        assert!(err.to_string().contains("node1"));

        let err = GraphValidationError::SelfEdge("node1".to_string());
        assert!(err.to_string().contains("Self-edge"));

        let err = GraphValidationError::CycleDetected(vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
        ]);
        assert!(err.to_string().contains("a -> b -> a"));
    }

    #[test]
    fn test_helper_constructors() {
        let err = GraphValidationError::missing_source("from_node", "to_node");
        let msg = err.to_string();
        assert!(msg.contains("from_node"));
        assert!(msg.contains("source node does not exist"));

        let err = GraphValidationError::missing_target("from_node", "to_node");
        let msg = err.to_string();
        assert!(msg.contains("target node does not exist"));
    }
}
