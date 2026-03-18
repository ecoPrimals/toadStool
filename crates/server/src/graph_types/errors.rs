// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph validation errors

/// Graph validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum GraphValidationError {
    /// Graph has no nodes.
    #[error("Graph is empty (no nodes)")]
    EmptyGraph,

    /// Duplicate node ID.
    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(String),

    /// Invalid edge.
    #[error("Invalid edge from '{from}' to '{to}': {reason}")]
    InvalidEdge {
        /// Source node ID.
        from: String,
        /// Target node ID.
        to: String,
        /// Reason for invalidity.
        reason: String,
    },

    /// Self-edge (node pointing to itself).
    #[error("Self-edge detected on node '{0}'")]
    SelfEdge(String),

    /// Cycle detected.
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
    fn test_empty_graph_display() {
        let err = GraphValidationError::EmptyGraph;
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_duplicate_node_id_display() {
        let err = GraphValidationError::DuplicateNodeId("node1".to_string());
        assert!(err.to_string().contains("node1"));
        assert!(err.to_string().contains("Duplicate"));
    }

    #[test]
    fn test_invalid_edge_display() {
        let err = GraphValidationError::InvalidEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            reason: "invalid".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains('a'));
        assert!(s.contains('b'));
        assert!(s.contains("invalid"));
    }

    #[test]
    fn test_self_edge_display() {
        let err = GraphValidationError::SelfEdge("node1".to_string());
        assert!(err.to_string().contains("Self-edge"));
        assert!(err.to_string().contains("node1"));
    }

    #[test]
    fn test_cycle_detected_display() {
        let err = GraphValidationError::CycleDetected(vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
        ]);
        assert!(err.to_string().contains("a -> b -> a"));
    }

    #[test]
    fn test_missing_source_helper() {
        let err = GraphValidationError::missing_source("from_node", "to_node");
        let msg = err.to_string();
        assert!(msg.contains("from_node"));
        assert!(msg.contains("to_node"));
        assert!(msg.contains("source node does not exist"));
    }

    #[test]
    fn test_missing_target_helper() {
        let err = GraphValidationError::missing_target("from_node", "to_node");
        let msg = err.to_string();
        assert!(msg.contains("from_node"));
        assert!(msg.contains("to_node"));
        assert!(msg.contains("target node does not exist"));
    }
}
